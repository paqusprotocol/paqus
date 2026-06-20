pub mod block;
pub mod error;
#[cfg(test)]
mod test;

pub use block::{Block, BlockHeader, CoinbaseTransaction, GenesisAllocation, MinerRevenue};
pub use error::BlockError;
