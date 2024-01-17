//! Silius RPC crate provides an interface for handling RPC methods according to the ERC-4337 spec.
#![allow(dead_code)]

pub mod codes;
mod debug;
pub mod debug_api;
mod error;
mod eth;
pub mod eth_api;
pub mod metrics;
pub mod middleware;
mod rpc;
mod web3;
pub mod web3_api;

pub use rpc::{JsonRpcServer, JsonRpcServerType};
