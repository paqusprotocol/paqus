use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use static_assertions::const_assert;

use crate::{block::BlockHeight, consensus::BLOCKS_PER_YEAR};

pub const UNIT: u64 = 1;
pub const XPQ: u64 = 100_000;
pub const DECIMALS: u8 = 5;

const_assert!(XPQ == 100_000);

pub const BLOCK_REWARD: u64 = 25 * XPQ; // 25 XPQ
pub const TAIL_EMISSION: u64 = 74_700; // 0.747 XPQ
pub const TAIL_EMISSION_START_YEARS: u64 = 5;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Amount(pub u64);

pub type Balance = Amount;
pub type Fee = Amount;

pub const TAIL_EMISSION_START_HEIGHT: u64 = TAIL_EMISSION_START_YEARS * BLOCKS_PER_YEAR;

pub fn block_reward(height: BlockHeight) -> Amount {
    if height.0 < TAIL_EMISSION_START_HEIGHT {
        Amount(BLOCK_REWARD)
    } else {
        Amount(TAIL_EMISSION)
    }
}

pub fn tail_emission_start_height() -> u64 {
    TAIL_EMISSION_START_HEIGHT
}
