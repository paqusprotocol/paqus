pub mod block;
pub mod error;
#[cfg(test)]
mod test;

pub use block::{Block, BlockHeader, MinerRevenue};
pub use error::BlockError;
