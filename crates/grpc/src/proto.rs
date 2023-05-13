// Code adapted from: https://github.com/ledgerwatch/interfaces/blob/master/src/lib.rs#L1
pub mod types {

    use std::str::FromStr;

    use aa_bundler_primitives::UserOperationHash;
    use arrayref::array_ref;
    use ethers::types::{Address, Bloom, Bytes};
    use prost::bytes::Buf;

    tonic::include_proto!("types");

    impl From<ethers::types::H128> for H128 {
        fn from(value: ethers::types::H128) -> Self {
            Self {
                hi: u64::from_be_bytes(*array_ref!(value, 0, 8)),
                lo: u64::from_be_bytes(*array_ref!(value, 8, 8)),
            }
        }
    }

    impl From<ethers::types::H160> for H160 {
        fn from(value: ethers::types::H160) -> Self {
            Self {
                hi: Some(ethers::types::H128::from_slice(&value[..16]).into()),
                lo: u32::from_be_bytes(*array_ref!(value, 16, 4)),
            }
        }
    }

    impl From<ethers::types::H256> for H256 {
        fn from(value: ethers::types::H256) -> Self {
            Self {
                hi: Some(ethers::types::H128::from_slice(&value[..16]).into()),
                lo: Some(ethers::types::H128::from_slice(&value[16..]).into()),
            }
        }
    }

    impl From<H128> for ethers::types::H128 {
        fn from(value: H128) -> Self {
            let mut v = [0; Self::len_bytes()];
            v[..8].copy_from_slice(&value.hi.to_be_bytes());
            v[8..].copy_from_slice(&value.lo.to_be_bytes());

            v.into()
        }
    }

    impl From<H160> for ethers::types::H160 {
        fn from(value: H160) -> Self {
            type H = ethers::types::H128;

            let mut v = [0; Self::len_bytes()];
            v[..H::len_bytes()]
                .copy_from_slice(H::from(value.hi.unwrap_or_default()).as_fixed_bytes());
            v[H::len_bytes()..].copy_from_slice(&value.lo.to_be_bytes());

            v.into()
        }
    }

    impl From<H256> for ethers::types::H256 {
        fn from(value: H256) -> Self {
            type H = ethers::types::H128;

            let mut v = [0; Self::len_bytes()];
            v[..H::len_bytes()]
                .copy_from_slice(H::from(value.hi.unwrap_or_default()).as_fixed_bytes());
            v[H::len_bytes()..]
                .copy_from_slice(H::from(value.lo.unwrap_or_default()).as_fixed_bytes());

            v.into()
        }
    }

    impl From<H256> for UserOperationHash {
        fn from(value: H256) -> Self {
            Self::from(ethers::types::H256::from(value))
        }
    }

    impl From<PbU256> for ethers::types::U256 {
        fn from(value: PbU256) -> Self {
            ethers::types::U256::from_big_endian(value.data.chunk())
        }
    }

    impl From<ethers::types::U256> for PbU256 {
        fn from(value: ethers::types::U256) -> Self {
            let mut bytes: [u8; 32] = [0; 32];
            value.to_big_endian(bytes.as_mut());
            PbU256 {
                data: prost::bytes::Bytes::copy_from_slice(&bytes),
            }
        }
    }

    impl From<aa_bundler_primitives::UserOperation> for UserOperation {
        fn from(user_operation: aa_bundler_primitives::UserOperation) -> Self {
            Self {
                sender: Some(user_operation.sender.into()),
                nonce: Some(user_operation.nonce.into()),
                init_code: prost::bytes::Bytes::copy_from_slice(user_operation.init_code.as_ref()),
                call_data: prost::bytes::Bytes::copy_from_slice(user_operation.call_data.as_ref()),
                call_gas_limit: Some(user_operation.call_gas_limit.into()),
                verification_gas_limit: Some(user_operation.verification_gas_limit.into()),
                pre_verification_gas: Some(user_operation.pre_verification_gas.into()),
                max_fee_per_gas: Some(user_operation.max_fee_per_gas.into()),
                max_priority_fee_per_gas: Some(user_operation.max_priority_fee_per_gas.into()),
                paymaster_and_data: prost::bytes::Bytes::copy_from_slice(
                    user_operation.paymaster_and_data.as_ref(),
                ),
                signature: prost::bytes::Bytes::copy_from_slice(user_operation.signature.as_ref()),
            }
        }
    }

    impl From<UserOperation> for aa_bundler_primitives::UserOperation {
        fn from(user_operation: UserOperation) -> Self {
            Self {
                sender: {
                    if let Some(sender) = user_operation.sender {
                        sender.into()
                    } else {
                        Address::zero()
                    }
                },
                nonce: user_operation.nonce.map(|n| n.into()).unwrap_or_default(),
                init_code: Bytes::from(user_operation.init_code),
                call_data: Bytes::from(user_operation.call_data),
                call_gas_limit: user_operation
                    .call_gas_limit
                    .map(|n| n.into())
                    .unwrap_or_default(),
                verification_gas_limit: user_operation
                    .verification_gas_limit
                    .map(|n| n.into())
                    .unwrap_or_default(),
                pre_verification_gas: user_operation
                    .pre_verification_gas
                    .map(|n| n.into())
                    .unwrap_or_default(),
                max_fee_per_gas: user_operation
                    .max_fee_per_gas
                    .map(|n| n.into())
                    .unwrap_or_default(),
                max_priority_fee_per_gas: user_operation
                    .max_priority_fee_per_gas
                    .map(|n| n.into())
                    .unwrap_or_default(),
                paymaster_and_data: Bytes::from(user_operation.paymaster_and_data),
                signature: Bytes::from(user_operation.signature),
            }
        }
    }

    impl From<aa_bundler_primitives::ReputationEntry> for ReputationEntry {
        fn from(reputation_entry: aa_bundler_primitives::ReputationEntry) -> Self {
            Self {
                address: Some(reputation_entry.address.into()),
                uo_seen: reputation_entry.uo_seen,
                uo_included: reputation_entry.uo_included,
                status: match reputation_entry.status {
                    aa_bundler_primitives::ReputationStatus::OK => ReputationStatus::Ok,
                    aa_bundler_primitives::ReputationStatus::THROTTLED => {
                        ReputationStatus::Throttled
                    }
                    aa_bundler_primitives::ReputationStatus::BANNED => ReputationStatus::Banned,
                } as i32,
            }
        }
    }

    impl From<ReputationEntry> for aa_bundler_primitives::ReputationEntry {
        fn from(reputation_entry: ReputationEntry) -> Self {
            Self {
                address: {
                    if let Some(address) = reputation_entry.address {
                        address.into()
                    } else {
                        Address::zero()
                    }
                },
                uo_seen: reputation_entry.uo_seen,
                uo_included: reputation_entry.uo_included,
                status: match reputation_entry.status {
                    _ if reputation_entry.status == ReputationStatus::Ok as i32 => {
                        aa_bundler_primitives::ReputationStatus::OK
                    }
                    _ if reputation_entry.status == ReputationStatus::Throttled as i32 => {
                        aa_bundler_primitives::ReputationStatus::THROTTLED
                    }
                    _ if reputation_entry.status == ReputationStatus::Banned as i32 => {
                        aa_bundler_primitives::ReputationStatus::BANNED
                    }
                    _ => aa_bundler_primitives::ReputationStatus::OK,
                },
            }
        }
    }

    impl From<ethers::types::TransactionReceipt> for TransactionReceipt {
        fn from(value: ethers::types::TransactionReceipt) -> Self {
            Self {
                transaction_hash: Some(value.transaction_hash.into()),
                transaction_index: value.transaction_index.as_u64(),
                block_hash: value.block_hash.map(|hash| hash.into()),
                block_number: value.block_number.unwrap_or_default().as_u64(),
                from: Some(value.from.into()),
                to: value.to.map(|address| address.into()),
                cumulative_gas_used: Some(value.cumulative_gas_used.into()),
                gas_used: value.gas_used.map(|gas| gas.into()),
                contract_address: value.contract_address.map(|address| address.into()),
                logs: value.logs.into_iter().map(|log| log.into()).collect(),
                logs_bloom: format!("{:}", value.logs_bloom),
                status: value.status.unwrap_or_default().as_u64(),
                root: value.root.map(|root| root.into()),
                effective_gas_price: value
                    .effective_gas_price
                    .map(|effective_gas_price| effective_gas_price.into()),
            }
        }
    }

    impl From<TransactionReceipt> for ethers::types::TransactionReceipt {
        fn from(value: TransactionReceipt) -> Self {
            Self {
                transaction_hash: value.transaction_hash.unwrap_or_default().into(),
                transaction_index: value.transaction_index.into(),
                block_hash: value.block_hash.map(|h| h.into()),
                block_number: Some(value.block_number.into()),
                from: value.from.unwrap_or_default().into(),
                to: value.to.map(|v| v.into()),
                cumulative_gas_used: value
                    .cumulative_gas_used
                    .map(|g| g.into())
                    .unwrap_or_default(),
                gas_used: value.gas_used.map(|g| g.into()),
                contract_address: value.contract_address.map(|a| a.into()),
                logs: value.logs.into_iter().map(|l| l.into()).collect(),
                status: Some(value.status.into()),
                root: value.root.map(|r| r.into()),
                logs_bloom: Bloom::from_str(&value.logs_bloom).unwrap_or_default(),
                transaction_type: None, // default type for eth now
                effective_gas_price: value.effective_gas_price.map(|g| g.into()),
                other: Default::default(),
            }
        }
    }

    impl From<ethers::types::Log> for Log {
        fn from(value: ethers::types::Log) -> Self {
            Self {
                address: Some(value.address.into()),
                topics: value.topics.into_iter().map(|topic| topic.into()).collect(),
                data: value.data.0,
                block_number: value
                    .block_number
                    .map(|block_number| block_number.as_u64())
                    .unwrap_or(0),
                transaction_hash: value
                    .transaction_hash
                    .map(|transaction_hash| transaction_hash.into()),
                transaction_index: value
                    .transaction_index
                    .map(|transaction_index| transaction_index.as_u64())
                    .unwrap_or(0),
                block_hash: value.block_hash.map(|block_hash| block_hash.into()),
                log_index: value.log_index.map(|log_index| log_index.into()),
                removed: value.removed.unwrap_or(false),
            }
        }
    }

    impl From<Log> for ethers::types::Log {
        fn from(value: Log) -> Self {
            Self {
                address: value.address.unwrap_or_default().into(),
                topics: value.topics.into_iter().map(|t| t.into()).collect(),
                data: value.data.into(),
                block_hash: value.block_hash.map(|b| b.into()),
                block_number: Some(value.block_number.into()),
                transaction_hash: value.transaction_hash.map(|t| t.into()),
                transaction_index: Some(value.transaction_index.into()),
                log_index: value.log_index.map(|l| l.into()),
                transaction_log_index: Some(value.transaction_index.into()),
                log_type: None,
                removed: Some(value.removed),
            }
        }
    }

    impl From<UserOperationHash> for H256 {
        fn from(value: UserOperationHash) -> Self {
            Self::from(value.0)
        }
    }
}

pub mod uopool {
    tonic::include_proto!("uopool");
}

pub mod bundler {
    use aa_bundler_primitives::Mode as GrpcMode;

    tonic::include_proto!("bundler");

    impl From<Mode> for GrpcMode {
        fn from(value: Mode) -> Self {
            match value {
                Mode::Auto => Self::Auto,
                Mode::Manual => Self::Manual,
            }
        }
    }

    impl From<GrpcMode> for Mode {
        fn from(value: GrpcMode) -> Self {
            match value {
                GrpcMode::Auto => Self::Auto,
                GrpcMode::Manual => Self::Manual,
            }
        }
    }
}
