use crate::{
    metrics::LabelValue,
    utils::{
        parse_address, parse_duration, parse_enr, parse_label_value, parse_send_bundle_mode,
        parse_u256, parse_uopool_mode,
    },
};
use alloy_chains::NamedChain;
use clap::{Parser, ValueEnum};
use discv5::Enr;
use ethers::types::{Address, U256};
use expanded_pathbuf::ExpandedPathBuf;
use silius_p2p::config::{Config, ListenAddr};
use silius_primitives::{
    bundler::SendStrategy,
    constants::{
        bundler::BUNDLE_INTERVAL,
        grpc::{BUNDLER_PORT, MEMPOOL_PORT},
        rpc::{HTTP_PORT, WS_PORT},
    },
    UoPoolMode,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

#[derive(ValueEnum, Debug, Clone)]
pub enum StorageType {
    Database,
    Memory,
}

/// Bundler CLI args
#[derive(Debug, Clone, Parser, PartialEq)]
pub struct BundlerArgs {
    /// Bundler gRPC address to listen on.
    #[clap(long = "bundler.addr", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub bundler_addr: IpAddr,

    /// Bundler gRPC port to listen on.
    #[clap(long = "bundler.port", default_value_t = BUNDLER_PORT)]
    pub bundler_port: u16,

    /// Path to the mnemonic file.
    #[clap(long)]
    pub mnemonic_file: PathBuf,

    /// The bundler beneficiary address.
    #[clap(long, value_parser=parse_address)]
    pub beneficiary: Address,

    /// The minimum balance required for the beneficiary address.
    ///
    /// By default, this option is set to `100000000000000000`.
    #[clap(long, default_value = "100000000000000000", value_parser=parse_u256)]
    pub min_balance: U256,

    /// The bundle interval in seconds.
    ///
    /// By default the interval time is set to 10
    #[clap(long, default_value_t = BUNDLE_INTERVAL)]
    pub bundle_interval: u64,

    /// Sets the send bundle mode.
    ///
    /// By default, this option is set to `eth-client`.
    #[clap(long, default_value = "eth-client", value_parser=parse_send_bundle_mode)]
    pub send_bundle_mode: SendStrategy,
}

/// UoPool CLI args
#[derive(Debug, Clone, Parser)]
pub struct UoPoolArgs {
    /// UoPool gRPC address to listen on.
    #[clap(long = "uopool.addr", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub uopool_addr: IpAddr,

    /// UoPool gRPC port to listen on.
    #[clap(long = "uopool.port", default_value_t = MEMPOOL_PORT)]
    pub uopool_port: u16,

    /// Data directory (primarily for database).
    #[clap(long)]
    pub datadir: Option<ExpandedPathBuf>,

    /// The storage type which is used for mempool and repution
    /// Currently, silius support `databse` and `memory` type
    #[clap(value_enum, default_value_t = StorageType::Database)]
    pub storage_type: StorageType,

    /// Max allowed verification gas.
    #[clap(long, default_value="5000000", value_parser=parse_u256)]
    pub max_verification_gas: U256,

    /// Minimum stake required for entities.
    #[clap(long, value_parser=parse_u256, default_value = "1")]
    pub min_stake: U256,

    /// Minimum priority fee per gas.
    #[clap(long, value_parser=parse_u256, default_value = "0")]
    pub min_priority_fee_per_gas: U256,

    /// Addresses of whitelisted entities.
    #[clap(long, value_delimiter=',', value_parser = parse_address)]
    pub whitelist: Vec<Address>,

    /// User operation mempool mode
    #[clap(long, default_value = "standard", value_parser=parse_uopool_mode)]
    pub uopool_mode: UoPoolMode,

    /// P2P configuration
    #[clap(flatten)]
    pub p2p_opts: P2PArgs,
}

/// Common CLI args for bundler and uopool
#[derive(Debug, Clone, Parser, PartialEq)]
pub struct BundlerAndUoPoolArgs {
    /// Ethereum execution client RPC endpoint.
    #[clap(long, default_value = "http://127.0.0.1:8545")]
    pub eth_client_address: String,

    /// Chain information.
    #[clap(long)]
    pub chain: Option<NamedChain>,

    /// Entry point addresses.
    #[clap(long, value_delimiter=',', value_parser=parse_address)]
    pub entry_points: Vec<Address>,

    /// Poll interval event filters and pending transactions in milliseconds.
    #[clap(long, default_value = "500", value_parser= parse_duration)]
    pub poll_interval: Duration,

    #[clap(flatten)]
    pub metrics: MetricArgs,
}

/// RPC CLI args
#[derive(Debug, Clone, Parser, PartialEq)]
pub struct RpcArgs {
    /// Enables or disables the HTTP RPC.
    ///
    /// By default, this option is set to false.
    /// - To enable: `--http`.
    /// - To disable: no `--http` flag.
    #[clap(long)]
    pub http: bool,

    /// Sets the HTTP RPC address to listen on.
    ///
    /// By default, this option is set to `127.0.0.1`
    #[clap(long = "http.addr", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub http_addr: IpAddr,

    /// Sets the HTTP RPC port to listen on.
    ///
    /// By default, this option is set to `3000`
    #[clap(long = "http.port", default_value_t = HTTP_PORT)]
    pub http_port: u16,

    /// Configures the HTTP RPC API modules.
    #[clap(long = "http.api", value_delimiter=',', default_value = "eth", value_parser = ["eth", "debug", "web3"])]
    pub http_api: Vec<String>,

    /// Configures the allowed CORS domains.
    ///
    /// By default, this option is set to `*`.
    #[clap(long = "http.corsdomain", value_delimiter = ',', default_value = "*")]
    pub http_corsdomain: Vec<String>,

    /// Enables or disables the WebSocket RPC.
    ///
    /// By default, this option is set to false.
    /// - To enable: `--ws`
    /// - To disable: no `--ws` flag.
    #[clap(long)]
    pub ws: bool,

    /// Sets the WS RPC address to listen on.
    ///
    /// By default, this option is set to `127.0.0.1`
    #[clap(long = "ws.addr", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub ws_addr: IpAddr,

    /// Sets the WS RPC port to listen on.
    ///
    /// By default, this option is set to `3001`
    #[clap(long = "ws.port", default_value_t = WS_PORT)]
    pub ws_port: u16,

    /// Configures the WS RPC API modules.
    #[clap(long = "ws.api", value_delimiter=',', default_value = "eth", value_parser = ["eth", "debug", "web3"])]
    pub ws_api: Vec<String>,

    /// Configures the allowed WS origins.
    ///
    /// By default, this option is set to `*`.
    #[clap(long = "ws.origins", value_delimiter = ',', default_value = "*")]
    pub ws_origins: Vec<String>,

    /// Ethereum execution client proxy HTTP RPC endpoint
    #[clap(long)]
    pub eth_client_proxy_address: Option<String>,
}

impl RpcArgs {
    /// Checks if either HTTP or WebSocket RPC is enabled.
    ///
    /// # Returns
    /// * `bool` - Returns `true` if either HTTP or WebSocket RPC is enabled, otherwise `false`.
    pub fn is_enabled(&self) -> bool {
        self.http || self.ws
    }

    /// Checks if the given API method is enabled.
    ///
    /// # Arguments
    /// * `method: &str` - The API method to check.
    ///
    /// # Returns
    /// * `bool` - Returns `true` if the given API method is enabled, otherwise `false`.
    pub fn is_api_method_enabled(&self, method: &str) -> bool {
        self.http_api.contains(&method.into()) || self.ws_api.contains(&method.into())
    }
}

/// Create wallet CLI args
#[derive(Debug, Clone, Parser)]
pub struct CreateWalletArgs {
    /// The path where the wallet will be stored.
    #[clap(long, short)]
    pub output_path: Option<ExpandedPathBuf>,

    /// The chain id.
    #[clap(long, default_value = "1")]
    pub chain_id: u64,

    /// Whether to create a Flashbots key.
    #[clap(long, default_value_t = false)]
    pub flashbots_key: bool,
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct P2PArgs {
    /// enable p2p
    #[clap(long)]
    pub enable_p2p: bool,

    /// Sets the p2p listen address.
    #[clap(long = "p2p.addr", default_value = "0.0.0.0")]
    pub p2p_listen_address: Ipv4Addr,

    /// The ipv4 address to broadcast to peers about which address we are listening on.
    #[clap(long = "p2p.baddr")]
    pub p2p_broadcast_address: Option<Ipv4Addr>,

    /// The udp4 port to broadcast to peers in order to reach back for discovery.
    #[clap(long = "discovery.port", default_value = "9000")]
    pub udp4_port: u16,

    /// The tcp4 port to boardcast to peers in order to reach back for discovery.
    #[clap(long = "p2p.port", default_value = "9000")]
    pub tcp4_port: u16,

    /// The initial bootnodes to connect to for the p2p network
    #[clap(long, value_delimiter = ',', value_parser=parse_enr)]
    pub bootnodes: Vec<Enr>,

    /// The path to the file where the p2p private key is stored.
    #[clap(long = "nodekey")]
    pub node_key: Option<PathBuf>,

    /// The path to the file where the p2p enr is stored.
    #[clap(long = "nodeenr")]
    pub node_enr: Option<PathBuf>,
}

impl P2PArgs {
    /// Convert the P2POpts to [silius_p2p::config::Config]
    pub fn to_config(&self) -> Config {
        // TODO: support ipv6
        Config {
            listen_addr: silius_p2p::config::ListenAddress::Ipv4(ListenAddr {
                addr: self.p2p_listen_address,
                udp_port: self.udp4_port,
                tcp_port: self.tcp4_port,
            }),
            ipv4_addr: self.p2p_broadcast_address,
            ipv6_addr: None,
            enr_udp4_port: Some(self.udp4_port),
            enr_tcp4_port: Some(self.tcp4_port),
            enr_udp6_port: None,
            enr_tcp6_port: None,
        }
    }
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct MetricArgs {
    #[clap(long)]
    pub enable_metrics: bool,
    #[clap(long, value_parser=parse_label_value)]
    pub custom_label_value: Option<LabelValue>,
    #[clap(long = "metrics.addr", default_value = "127.0.0.1")]
    pub listen_address: Ipv4Addr,
    #[clap(long = "metrics.port", default_value = "3030")]
    pub port: u16,
}

impl MetricArgs {
    pub fn listen_addr(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(self.listen_address), self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use discv5::enr::{CombinedKey, Enr as EnrBuilder};
    use std::{
        net::{IpAddr, Ipv4Addr},
        str::FromStr,
    };

    #[test]
    fn bundler_args() {
        let args = vec![
            "bundlerargs",
            "--mnemonic-file",
            "~/.silius/0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990",
            "--beneficiary",
            "0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990",
            "--min-balance",
            "100000000000000000",
            "--bundler.addr",
            "127.0.0.1",
            "--bundler.port",
            "3002",
            "--bundle-interval",
            "10",
        ];
        assert_eq!(
            BundlerArgs {
                mnemonic_file: PathBuf::from(
                    "~/.silius/0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990"
                ),
                beneficiary: Address::from_str("0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990")
                    .unwrap(),
                min_balance: U256::from(100000000000000000_u64),
                bundle_interval: 10,
                send_bundle_mode: SendStrategy::EthClient,
                bundler_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                bundler_port: 3002,
            },
            BundlerArgs::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn bundler_and_uopool_args() {
        let args = vec![
            "bundleranduopoolargs",
            "--eth-client-address",
            "http://127.0.0.1:8545",
            "--chain",
            "holesky",
            "--entry-points",
            "0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990",
            "--poll-interval",
            "5000",
        ];
        assert_eq!(
            BundlerAndUoPoolArgs {
                eth_client_address: String::from("http://127.0.0.1:8545"),
                chain: Some(NamedChain::Holesky),
                entry_points: vec![
                    Address::from_str("0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990").unwrap()
                ],
                poll_interval: Duration::from_millis(5000),
                metrics: MetricArgs {
                    enable_metrics: false,
                    custom_label_value: None,
                    listen_address: Ipv4Addr::new(127, 0, 0, 1),
                    port: 3030
                }
            },
            BundlerAndUoPoolArgs::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_args_when_http_and_ws_flag() {
        let args = vec![
            "rpcargs",
            "--http",
            "--http.addr",
            "127.0.0.1",
            "--http.port",
            "3000",
            "--http.api",
            "eth,debug,web3",
            "--http.corsdomain",
            "127.0.0.1:4321",
            "--ws",
            "--ws.addr",
            "127.0.0.1",
            "--ws.port",
            "3001",
            "--ws.api",
            "eth,debug,web3",
            "--ws.origins",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcArgs {
                http: true,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                http_corsdomain: vec![String::from("127.0.0.1:4321")],
                ws: true,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                ws_origins: vec![String::from("127.0.0.1:4321")],
                eth_client_proxy_address: None,
            },
            RpcArgs::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_args_when_http_is_true_ws_is_false() {
        let args = vec![
            "rpcargs",
            "--http",
            "--http.addr",
            "127.0.0.1",
            "--http.port",
            "3000",
            "--http.api",
            "eth,debug,web3",
            "--http.corsdomain",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcArgs {
                http: true,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                http_corsdomain: vec![String::from("127.0.0.1:4321")],
                ws: false,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"),],
                ws_origins: vec![String::from("*")],
                eth_client_proxy_address: None,
            },
            RpcArgs::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_args_when_http_is_false_ws_is_true() {
        let args = vec![
            "rpcargs",
            "--ws",
            "--ws.addr",
            "127.0.0.1",
            "--ws.port",
            "3001",
            "--ws.api",
            "eth,debug,web3",
            "--ws.origins",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcArgs {
                http: false,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"),],
                http_corsdomain: vec![String::from("*")],
                ws: true,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                ws_origins: vec![String::from("127.0.0.1:4321")],
                eth_client_proxy_address: None,
            },
            RpcArgs::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn rpc_args_when_no_http_and_ws_flag() {
        let args = vec![
            "rpcargs",
            "--http.addr",
            "127.0.0.1",
            "--http.port",
            "3000",
            "--http.api",
            "eth,debug,web3",
            "--http.corsdomain",
            "127.0.0.1:4321",
        ];
        assert_eq!(
            RpcArgs {
                http: false,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                http_corsdomain: vec![String::from("127.0.0.1:4321")],
                ws: false,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"),],
                ws_origins: vec![String::from("*")],
                eth_client_proxy_address: None,
            },
            RpcArgs::try_parse_from(args).unwrap()
        );
    }

    #[test]
    fn is_enabled_return_true_when_only_http() {
        assert_eq!(
            RpcArgs {
                http: true,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                http_corsdomain: vec![String::from("127.0.0.1:4321")],
                ws: false,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"),],
                ws_origins: vec![String::from("*")],
                eth_client_proxy_address: None,
            }
            .is_enabled(),
            true
        );
    }

    #[test]
    fn is_enabled_return_true_when_only_ws() {
        assert_eq!(
            RpcArgs {
                http: false,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"),],
                http_corsdomain: vec![String::from("*")],
                ws: true,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                ws_origins: vec![String::from("127.0.0.1:4321")],
                eth_client_proxy_address: None,
            }
            .is_enabled(),
            true
        );
    }

    #[test]
    fn is_enabled_return_true_when_http_and_ws_are_true() {
        assert_eq!(
            RpcArgs {
                http: true,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                http_corsdomain: vec![String::from("127.0.0.1:4321")],
                ws: true,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                ws_origins: vec![String::from("127.0.0.1:4321")],
                eth_client_proxy_address: None,
            }
            .is_enabled(),
            true
        );
    }

    #[test]
    fn is_enabled_return_false_when_http_and_ws_are_false() {
        assert_eq!(
            RpcArgs {
                http: false,
                http_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                http_port: 3000,
                http_api: vec![String::from("eth"), String::from("debug"), String::from("web3")],
                http_corsdomain: vec![String::from("127.0.0.1:4321")],
                ws: false,
                ws_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                ws_port: 3001,
                ws_api: vec![String::from("eth"),],
                ws_origins: vec![String::from("*")],
                eth_client_proxy_address: None,
            }
            .is_enabled(),
            false
        );
    }

    #[test]
    fn p2p_opts() {
        let key = CombinedKey::secp256k1_from_bytes([1; 32].as_mut()).unwrap();
        let enr = EnrBuilder::builder()
            .ip4(Ipv4Addr::new(8, 8, 8, 8))
            .tcp4(4337)
            .udp4(4337)
            .build(&key)
            .unwrap();
        let binding = enr.clone().to_base64();
        let args = vec![
            "p2popts",
            "--enable-p2p",
            "--p2p.addr",
            "0.0.0.0",
            "--p2p.baddr",
            "127.0.0.1",
            "--discovery.port",
            "4337",
            "--p2p.port",
            "4337",
            "--bootnodes",
            &binding,
            "--nodekey",
            "~/.silius/p2p/node-key",
            "--nodeenr",
            "~/.silius/p2p/node-enr",
        ];
        assert_eq!(
            P2PArgs {
                enable_p2p: true,
                p2p_listen_address: Ipv4Addr::new(0, 0, 0, 0),
                p2p_broadcast_address: Some(Ipv4Addr::new(127, 0, 0, 1)),
                tcp4_port: 4337,
                udp4_port: 4337,
                bootnodes: vec![enr],
                node_key: Some(PathBuf::from("~/.silius/p2p/node-key")),
                node_enr: Some(PathBuf::from("~/.silius/p2p/node-enr"))
            },
            P2PArgs::try_parse_from(args).unwrap()
        )
    }

    #[test]
    fn metrics_args() {
        let args = vec![
            "metricsargs",
            "--enable-metrics",
            "--metrics.addr",
            "127.0.0.1",
            "--metrics.port",
            "9090",
            "--custom-label-value",
            "custom=value",
        ];
        assert_eq!(
            MetricArgs {
                enable_metrics: true,
                listen_address: Ipv4Addr::new(127, 0, 0, 1),
                port: 9090,
                custom_label_value: Some(LabelValue::new(
                    String::from("custom"),
                    String::from("value")
                ))
            },
            MetricArgs::try_parse_from(args).unwrap()
        )
    }
}
