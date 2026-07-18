#![allow(clippy::module_inception)]

pub mod consensus;
pub mod supply;

pub use crate::error::ConsensusError;
pub use consensus::{
    ASERT_HALF_LIFE, BLOCK_TIME, BLOCKS_PER_DAY, BLOCKS_PER_YEAR, Consensus, ConsensusConfig,
    DIFFICULTY_ADJUSTMENT_INTERVAL, DIFFICULTY_START, MAX_FUTURE_TIME, MIN_DIFFICULTY,
};
pub use supply::{TAIL_EMISSION_START_HEIGHT, block_reward, tail_emission_start_height};
