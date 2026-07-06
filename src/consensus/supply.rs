use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use static_assertions::const_assert;

use crate::block::BlockHeight;

pub const UNIT: u64 = 1;
pub const XPQ: u64 = 100_000_000;
pub const DECIMALS: u8 = 8;

pub const MAX_XPQ_SUPPLY: u64 = 42_000_000;
pub const MAX_UNIT_SUPPLY: u64 = MAX_XPQ_SUPPLY * XPQ;
pub const MAX_MINED_SUPPLY: u64 = MAX_UNIT_SUPPLY;
const_assert!(MAX_XPQ_SUPPLY == 42_000_000);
const_assert!(XPQ == 100_000_000);
const_assert!(MAX_UNIT_SUPPLY == 4_200_000_000_000_000);
const_assert!(MAX_UNIT_SUPPLY <= u64::MAX);

pub const BLOCK_REWARD: u64 = 5_000_000_000; // 50 XPQ
pub const TAIL_EMISSION: u64 = 100_000_000; // 1 XPQ
pub const TAIL_EMISSION_START_YEARS: u64 = 4;

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

pub const TAIL_EMISSION_START_HEIGHT: u64 = TAIL_EMISSION_START_YEARS * super::BLOCKS_PER_YEAR;

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
