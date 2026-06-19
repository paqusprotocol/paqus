use crate::params::{BLOCK_REWARD, TAIL_EMISSION, TAIL_EMISSION_START_HEIGHT};
use crate::types::{Amount, BlockHeight};

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
