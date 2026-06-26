#![allow(clippy::module_inception)]

pub mod block;

pub use crate::error::BlockError;
pub use block::{Block, BlockHeader, CoinbaseTransaction, GenesisAllocation, MinerRevenue};
