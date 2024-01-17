use crate::{
    cli::args::{
        BundlerAndUoPoolArgs, BundlerArgs, CreateWalletArgs, MetricArgs, RpcArgs, StorageType,
        UoPoolArgs,
    },
    metrics::launch_metrics_exporter,
    utils::unwrap_path_or_home,
};
use alloy_chains::{Chain, NamedChain};
use ethers::{providers::Middleware, types::Address};
use parking_lot::RwLock;
use silius_contracts::EntryPoint;
use silius_grpc::{
    bundler_client::BundlerClient, bundler_service_run, uo_pool_client::UoPoolClient,
    uopool_service_run,
};
use silius_mempool::{
    init_env,
    metrics::MetricsHandler,
    validate::validator::{new_canonical, new_canonical_unsafe},
    CodeHashes, DatabaseTable, EntitiesReputation, Mempool, Reputation, UserOperations,
    UserOperationsByEntity, UserOperationsBySender, WriteMap,
};
use silius_primitives::{
    bundler::SendStrategy,
    constants::{
        entry_point, flashbots_relay_endpoints,
        p2p::{NODE_ENR_FILE_NAME, NODE_KEY_FILE_NAME},
        storage::DATABASE_FOLDER_NAME,
        validation::reputation::{
            BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, MIN_UNSTAKE_DELAY, THROTTLING_SLACK,
        },
    },
    provider::BlockStream,
    reputation::ReputationEntry,
    simulation::CodeHash,
    UserOperationHash, UserOperationSigned, Wallet,
};
use silius_rpc::{
    debug_api::{DebugApiServer, DebugApiServerImpl},
    eth_api::{EthApiServer, EthApiServerImpl},
    web3_api::{Web3ApiServer, Web3ApiServerImpl},
    JsonRpcServer, JsonRpcServerType,
};
use std::{
    collections::{HashMap, HashSet},
    future::pending,
    net::SocketAddr,
    str::FromStr,
    sync::Arc,
};
use tracing::{info, warn};

pub async fn launch_bundler<M>(
    bundler_args: BundlerArgs,
    uopool_args: UoPoolArgs,
    common_args: BundlerAndUoPoolArgs,
    rpc_args: RpcArgs,
    metrics_args: MetricArgs,
    eth_client: Arc<M>,
    block_streams: Vec<BlockStream>,
) -> eyre::Result<()>
where
    M: Middleware + Clone + 'static,
{
    launch_uopool(
        uopool_args.clone(),
        eth_client.clone(),
        block_streams,
        common_args.chain,
        common_args.entry_points.clone(),
        metrics_args.clone(),
    )
    .await?;

    launch_bundling(
        bundler_args.clone(),
        eth_client.clone(),
        common_args.chain,
        common_args.entry_points,
        format!("http://{:?}:{:?}", uopool_args.uopool_addr, uopool_args.uopool_port),
        metrics_args.clone(),
    )
    .await?;

    launch_rpc(
        rpc_args,
        format!("http://{:?}:{:?}", uopool_args.uopool_addr, uopool_args.uopool_port),
        format!("http://{:?}:{:?}", bundler_args.bundler_addr, bundler_args.bundler_port),
        metrics_args.clone(),
    )
    .await?;

    if metrics_args.enable_metrics {
        launch_metrics_exporter(metrics_args.listen_addr(), metrics_args.custom_label_value);
    }

    Ok(())
}

pub async fn launch_bundling<M>(
    args: BundlerArgs,
    eth_client: Arc<M>,
    chain: Option<NamedChain>,
    entry_points: Vec<Address>,
    uopool_grpc_listen_address: String,
    metrics_args: MetricArgs,
) -> eyre::Result<()>
where
    M: Middleware + Clone + 'static,
{
    info!("Starting bundling gRPC service...");

    let eth_client_version = check_connected_chain(eth_client.clone(), chain).await?;
    info!(
        "Bundling component connected to Ethereum execution client with version {}",
        eth_client_version,
    );

    let chain_id = eth_client.get_chainid().await?.as_u64();
    let chain_conn = Chain::from(chain_id);

    let wallet: Wallet;
    if args.send_bundle_mode == SendStrategy::Flashbots {
        wallet = Wallet::from_file(args.mnemonic_file.into(), chain_id, true)
            .map_err(|error| eyre::format_err!("Could not load mnemonic file: {}", error))?;
        info!("Wallet Signer {:?}", wallet.signer);
        info!("Flashbots Signer {:?}", wallet.flashbots_signer);
    } else {
        wallet = Wallet::from_file(args.mnemonic_file.into(), chain_id, false)
            .map_err(|error| eyre::format_err!("Could not load mnemonic file: {}", error))?;
        info!("{:?}", wallet.signer);
    }

    info!("Connecting to uopool gRPC service...");
    let uopool_grpc_client = UoPoolClient::connect(uopool_grpc_listen_address).await?;
    info!("Connected to uopool gRPC service");

    bundler_service_run(
        SocketAddr::new(args.bundler_addr, args.bundler_port),
        wallet,
        entry_points,
        eth_client,
        chain_conn,
        args.beneficiary,
        args.min_balance,
        args.bundle_interval,
        uopool_grpc_client,
        args.send_bundle_mode,
        match args.send_bundle_mode {
            SendStrategy::EthClient => None,
            SendStrategy::Flashbots => Some(vec![flashbots_relay_endpoints::FLASHBOTS.into()]),
        },
        metrics_args.enable_metrics,
    );
    info!("Started bundler gRPC service at {:?}:{:?}", args.bundler_addr, args.bundler_port);

    Ok(())
}

pub async fn launch_uopool<M>(
    args: UoPoolArgs,
    eth_client: Arc<M>,
    block_streams: Vec<BlockStream>,
    chain: Option<NamedChain>,
    entry_points: Vec<Address>,
    metrics_args: MetricArgs,
) -> eyre::Result<()>
where
    M: Middleware + Clone + 'static,
{
    info!("Starting uopool gRPC service...");

    let eth_client_version = check_connected_chain(eth_client.clone(), chain).await?;
    info!(
        "UoPool component connected to Ethereum execution client with version {}",
        eth_client_version
    );

    let chain_id = Chain::from(eth_client.get_chainid().await?.as_u64());

    let datadir = unwrap_path_or_home(args.datadir)?;

    let node_key_file = match args.p2p_opts.node_key.clone() {
        Some(key_file) => key_file,
        None => datadir.join(NODE_KEY_FILE_NAME),
    };
    let node_enr_file = match args.p2p_opts.node_enr.clone() {
        Some(enr_file) => enr_file,
        None => datadir.join(NODE_ENR_FILE_NAME),
    };

    let entrypoint_api = EntryPoint::new(
        eth_client.clone(),
        Address::from_str(entry_point::ADDRESS).expect("address should be valid"),
    );
    match (args.uopool_mode, args.storage_type) {
        (silius_primitives::UoPoolMode::Standard, StorageType::Memory) => {
            let validator = new_canonical(
                entrypoint_api,
                chain_id,
                args.max_verification_gas,
                args.min_priority_fee_per_gas,
            );
            let mempool = Mempool::new(
                Arc::new(RwLock::new(MetricsHandler::new(HashMap::<
                    UserOperationHash,
                    UserOperationSigned,
                >::default()))),
                Arc::new(RwLock::new(HashMap::<Address, HashSet<UserOperationHash>>::default())),
                Arc::new(RwLock::new(HashMap::<Address, HashSet<UserOperationHash>>::default())),
                Arc::new(RwLock::new(HashMap::<UserOperationHash, Vec<CodeHash>>::default())),
            );
            let mut reputation = Reputation::new(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                args.min_stake,
                MIN_UNSTAKE_DELAY.into(),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                Arc::new(RwLock::new(MetricsHandler::new(
                    HashMap::<Address, ReputationEntry>::default(),
                ))),
            );
            for whiteaddr in args.whitelist.iter() {
                reputation.add_whitelist(whiteaddr);
            }
            uopool_service_run(
                SocketAddr::new(args.uopool_addr, args.uopool_port),
                entry_points,
                eth_client,
                block_streams,
                chain_id,
                args.max_verification_gas,
                mempool,
                reputation,
                validator,
                args.p2p_opts.enable_p2p,
                node_key_file,
                node_enr_file,
                args.p2p_opts.to_config(),
                args.p2p_opts.bootnodes,
                metrics_args.enable_metrics,
            )
            .await?;
            info!("Started uopool gRPC service at {:?}:{:?}", args.uopool_addr, args.uopool_port);
        }
        (silius_primitives::UoPoolMode::Standard, StorageType::Database) => {
            let validator = new_canonical(
                entrypoint_api,
                chain_id,
                args.max_verification_gas,
                args.min_priority_fee_per_gas,
            );
            let env = Arc::new(
                init_env::<WriteMap>(datadir.join(DATABASE_FOLDER_NAME)).expect("Init mdbx failed"),
            );
            env.create_tables().expect("Create mdbx database tables failed");
            let mempool = Mempool::new(
                MetricsHandler::new(DatabaseTable::<WriteMap, UserOperations>::new(env.clone())),
                DatabaseTable::<WriteMap, UserOperationsBySender>::new(env.clone()),
                DatabaseTable::<WriteMap, UserOperationsByEntity>::new(env.clone()),
                DatabaseTable::<WriteMap, CodeHashes>::new(env.clone()),
            );
            let mut reputation = Reputation::new(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                args.min_stake,
                MIN_UNSTAKE_DELAY.into(),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                MetricsHandler::new(DatabaseTable::<WriteMap, EntitiesReputation>::new(
                    env.clone(),
                )),
            );
            for whiteaddr in args.whitelist.iter() {
                reputation.add_whitelist(whiteaddr);
            }
            uopool_service_run(
                SocketAddr::new(args.uopool_addr, args.uopool_port),
                entry_points,
                eth_client,
                block_streams,
                chain_id,
                args.max_verification_gas,
                mempool,
                reputation,
                validator,
                args.p2p_opts.enable_p2p,
                node_key_file,
                node_enr_file,
                args.p2p_opts.to_config(),
                args.p2p_opts.bootnodes,
                metrics_args.enable_metrics,
            )
            .await?;
            info!("Started uopool gRPC service at {:?}:{:?}", args.uopool_addr, args.uopool_port);
        }
        (silius_primitives::UoPoolMode::Unsafe, StorageType::Memory) => {
            let validator = new_canonical_unsafe(
                entrypoint_api,
                chain_id,
                args.max_verification_gas,
                args.min_priority_fee_per_gas,
            );
            let mempool = Mempool::new(
                Arc::new(RwLock::new(MetricsHandler::new(HashMap::<
                    UserOperationHash,
                    UserOperationSigned,
                >::default()))),
                Arc::new(RwLock::new(HashMap::<Address, HashSet<UserOperationHash>>::default())),
                Arc::new(RwLock::new(HashMap::<Address, HashSet<UserOperationHash>>::default())),
                Arc::new(RwLock::new(HashMap::<UserOperationHash, Vec<CodeHash>>::default())),
            );
            let mut reputation = Reputation::new(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                args.min_stake,
                MIN_UNSTAKE_DELAY.into(),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                Arc::new(RwLock::new(MetricsHandler::new(
                    HashMap::<Address, ReputationEntry>::default(),
                ))),
            );
            for whiteaddr in args.whitelist.iter() {
                reputation.add_whitelist(whiteaddr);
            }
            uopool_service_run(
                SocketAddr::new(args.uopool_addr, args.uopool_port),
                entry_points,
                eth_client,
                block_streams,
                chain_id,
                args.max_verification_gas,
                mempool,
                reputation,
                validator,
                args.p2p_opts.enable_p2p,
                node_key_file,
                node_enr_file,
                args.p2p_opts.to_config(),
                args.p2p_opts.bootnodes,
                metrics_args.enable_metrics,
            )
            .await?;
            info!("Started uopool gRPC service at {:?}:{:?}", args.uopool_addr, args.uopool_port);
        }
        (silius_primitives::UoPoolMode::Unsafe, StorageType::Database) => {
            let validator = new_canonical_unsafe(
                entrypoint_api,
                chain_id,
                args.max_verification_gas,
                args.min_priority_fee_per_gas,
            );
            let env = Arc::new(
                init_env::<WriteMap>(datadir.join(DATABASE_FOLDER_NAME)).expect("Init mdbx failed"),
            );
            env.create_tables().expect("Create mdbx database tables failed");
            let mempool = Mempool::new(
                MetricsHandler::new(DatabaseTable::<WriteMap, UserOperations>::new(env.clone())),
                DatabaseTable::<WriteMap, UserOperationsBySender>::new(env.clone()),
                DatabaseTable::<WriteMap, UserOperationsByEntity>::new(env.clone()),
                DatabaseTable::<WriteMap, CodeHashes>::new(env.clone()),
            );
            let mut reputation = Reputation::new(
                MIN_INCLUSION_RATE_DENOMINATOR,
                THROTTLING_SLACK,
                BAN_SLACK,
                args.min_stake,
                MIN_UNSTAKE_DELAY.into(),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                Arc::new(RwLock::new(HashSet::<Address>::default())),
                MetricsHandler::new(DatabaseTable::<WriteMap, EntitiesReputation>::new(
                    env.clone(),
                )),
            );
            for whiteaddr in args.whitelist.iter() {
                reputation.add_whitelist(whiteaddr);
            }
            uopool_service_run(
                SocketAddr::new(args.uopool_addr, args.uopool_port),
                entry_points,
                eth_client,
                block_streams,
                chain_id,
                args.max_verification_gas,
                mempool,
                reputation,
                validator,
                args.p2p_opts.enable_p2p,
                node_key_file,
                node_enr_file,
                args.p2p_opts.to_config(),
                args.p2p_opts.bootnodes,
                metrics_args.enable_metrics,
            )
            .await?;
            info!("Started uopool gRPC service at {:?}:{:?}", args.uopool_addr, args.uopool_port);
        }
    };

    Ok(())
}

pub async fn launch_rpc(
    args: RpcArgs,
    uopool_grpc_listen_address: String,
    bundler_grpc_listen_address: String,
    metrics_args: MetricArgs,
) -> eyre::Result<()> {
    if !args.is_enabled() {
        return Err(eyre::eyre!("No RPC protocol is enabled"));
    }

    info!("Starting bundler JSON-RPC server...");

    let mut server = JsonRpcServer::new(
        args.http,
        args.http_addr,
        args.http_port,
        args.ws,
        args.ws_addr,
        args.ws_port,
    )
    .with_cors(&args.http_corsdomain, JsonRpcServerType::Http)
    .with_cors(&args.ws_origins, JsonRpcServerType::Ws);

    if let Some(eth_client_proxy_address) = args.eth_client_proxy_address.clone() {
        server = server.with_proxy(eth_client_proxy_address);
    }

    if metrics_args.enable_metrics {
        info!("Enabling json rpc server metrics.");
        server = server.with_metrics()
    }

    let http_api: HashSet<String> = HashSet::from_iter(args.http_api.iter().cloned());
    let ws_api: HashSet<String> = HashSet::from_iter(args.ws_api.iter().cloned());

    if http_api.contains("web3") {
        server.add_methods(Web3ApiServerImpl {}.into_rpc(), JsonRpcServerType::Http)?;
    }
    if ws_api.contains("web3") {
        server.add_methods(Web3ApiServerImpl {}.into_rpc(), JsonRpcServerType::Ws)?;
    }

    info!("Connecting to uopool gRPC service...");
    let uopool_grpc_client = UoPoolClient::connect(uopool_grpc_listen_address).await?;
    info!("Connected to uopool gRPC service...");

    if args.is_api_method_enabled("eth") {
        if http_api.contains("eth") {
            server.add_methods(
                EthApiServerImpl { uopool_grpc_client: uopool_grpc_client.clone() }.into_rpc(),
                JsonRpcServerType::Http,
            )?;
        }
        if ws_api.contains("eth") {
            server.add_methods(
                EthApiServerImpl { uopool_grpc_client: uopool_grpc_client.clone() }.into_rpc(),
                JsonRpcServerType::Ws,
            )?;
        }
    }

    if args.is_api_method_enabled("debug") {
        info!("Connecting to bundling gRPC service...");
        let bundler_grpc_client = BundlerClient::connect(bundler_grpc_listen_address).await?;
        info!("Connected to bundling gRPC service...");

        if http_api.contains("debug") {
            server.add_methods(
                DebugApiServerImpl {
                    uopool_grpc_client: uopool_grpc_client.clone(),
                    bundler_grpc_client: bundler_grpc_client.clone(),
                }
                .into_rpc(),
                JsonRpcServerType::Http,
            )?;
        }

        if ws_api.contains("debug") {
            server.add_methods(
                DebugApiServerImpl { uopool_grpc_client, bundler_grpc_client }.into_rpc(),
                JsonRpcServerType::Ws,
            )?;
        }
    }

    tokio::spawn(async move {
        let (_http_handle, _ws_handle) = server.start().await?;

        info!(
            "Started bundler JSON-RPC server with http: {:?}:{:?}, ws: {:?}:{:?}",
            args.http_addr, args.http_port, args.ws_addr, args.ws_port,
        );
        pending::<eyre::Result<()>>().await
    });

    Ok(())
}

pub fn create_wallet(args: CreateWalletArgs) -> eyre::Result<()> {
    info!("Creating bundler wallet... Storing to: {:?}", args.output_path);

    let path = unwrap_path_or_home(args.output_path)?;

    if args.flashbots_key {
        let wallet = Wallet::build_random(path, args.chain_id, true)?;
        info!("Wallet signer {:?}", wallet.signer);
        info!("Flashbots signer {:?}", wallet.flashbots_signer);
    } else {
        let wallet = Wallet::build_random(path, args.chain_id, false)?;
        info!("Wallet signer {:?}", wallet.signer);
    }

    Ok(())
}

async fn check_connected_chain<M>(
    eth_client: Arc<M>,
    chain: Option<NamedChain>,
) -> eyre::Result<String>
where
    M: Middleware + Clone + 'static,
{
    if let Some(chain) = chain {
        match chain {
            NamedChain::Mainnet |
            NamedChain::Goerli |
            NamedChain::Sepolia |
            NamedChain::PolygonMumbai |
            NamedChain::Dev => {}
            _ => {
                warn!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                warn!("Chain {:?} is not officially supported yet! You could possibly meet a lot of problems with silius. Use at your own risk!!", chain);
                warn!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
            }
        }
        let chain: Chain = chain.into();

        let chain_id = eth_client.get_chainid().await?.as_u64();
        if chain.id() != chain_id {
            return Err(eyre::format_err!(
                "Tried to connect to the execution client of different chain ids: {} != {}",
                chain.id(),
                chain_id
            ));
        }
    }

    Ok(eth_client.client_version().await?)
}
