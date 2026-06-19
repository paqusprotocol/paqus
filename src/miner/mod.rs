pub mod miner;
#[cfg(test)]
mod test;

pub use miner::{MiningConfig, MiningResult, mine_candidate_block};
