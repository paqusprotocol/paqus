use crate::block::Block;
use crate::block::{BlockHeight, Height};
use crate::crypto::{
    BlockHash, HASH_SIZE, Hash, PreviousHash, ProofOfWorkHash, argon2_proof_of_work_hash,
    hash_meets_difficulty,
};

use crate::error::ConsensusError;

const SECOND: u32 = 1;
const MINUTE: u32 = 60 * SECOND;
const HOUR: u32 = 60 * MINUTE;
const DAY: u32 = 24 * HOUR;
pub const BLOCK_TIME: u32 = 5 * MINUTE;
pub const BLOCKS_PER_DAY: u64 = DAY as u64 / BLOCK_TIME as u64;
pub const BLOCKS_PER_YEAR: u64 = 365 * BLOCKS_PER_DAY;
pub const MIN_DIFFICULTY: u32 = 1;
pub const DIFFICULTY_START: u32 = 1;
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;
pub const DIFFICULTY_TIMESPAN_CLAMP_FACTOR: u64 = 16;
pub const MAX_DIFFICULTY_ADJUSTMENT_BITS: u32 = 4;
pub const MAX_FUTURE_TIME: u32 = 2 * MINUTE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsensusConfig {
    pub difficulty: u32,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            difficulty: DIFFICULTY_START,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Consensus {
    pub config: ConsensusConfig,
}

impl Consensus {
    pub fn new(config: ConsensusConfig) -> Result<Self, ConsensusError> {
        if config.difficulty < MIN_DIFFICULTY {
            return Err(ConsensusError::InvalidDifficulty);
        }

        Ok(Self { config })
    }

    pub fn with_default_config() -> Self {
        Self::new(ConsensusConfig::default()).expect("default consensus config should be valid")
    }

    pub fn validate_genesis_block(&self, block: &Block) -> Result<(), ConsensusError> {
        self.validate_genesis_block_at(block, block.timestamp())
    }

    pub fn validate_genesis_block_at(&self, block: &Block, now: u64) -> Result<(), ConsensusError> {
        block.validate_at(now)?;

        if block.height() != Height(0) || block.previous_hash() != Hash([0; HASH_SIZE]) {
            return Err(ConsensusError::InvalidHeight);
        }

        self.validate_proof_of_work(block)
    }

    pub fn validate_next_block(
        &self,
        block: &Block,
        tip_height: BlockHeight,
        tip_hash: BlockHash,
    ) -> Result<(), ConsensusError> {
        self.validate_next_block_at(block, tip_height, tip_hash, block.timestamp())
    }

    pub fn validate_next_block_at(
        &self,
        block: &Block,
        tip_height: BlockHeight,
        tip_hash: BlockHash,
        now: u64,
    ) -> Result<(), ConsensusError> {
        block.validate_at(now)?;
        self.validate_next_block_linkage(block, tip_height, tip_hash)?;
        self.validate_proof_of_work(block)
    }

    pub fn validate_next_block_with_tip(
        &self,
        block: &Block,
        tip: &Block,
    ) -> Result<(), ConsensusError> {
        self.validate_next_block_with_tip_at(block, tip, block.timestamp())
    }

    pub fn validate_next_block_with_tip_at(
        &self,
        block: &Block,
        tip: &Block,
        now: u64,
    ) -> Result<(), ConsensusError> {
        block.validate_at(now)?;
        self.validate_next_block_linkage(block, tip.height(), tip.hash())?;
        if block.timestamp() < tip.timestamp() {
            return Err(ConsensusError::InvalidTimestamp);
        }
        self.validate_proof_of_work(block)
    }

    fn validate_next_block_linkage(
        &self,
        block: &Block,
        tip_height: BlockHeight,
        tip_hash: BlockHash,
    ) -> Result<(), ConsensusError> {
        if block.height().0 != tip_height.0.saturating_add(1) {
            return Err(ConsensusError::InvalidHeight);
        }

        if block.previous_hash() != tip_hash {
            return Err(ConsensusError::InvalidPreviousHash);
        }

        Ok(())
    }

    pub fn validate_candidate_block(
        &self,
        block: &Block,
        tip: Option<(BlockHeight, BlockHash)>,
    ) -> Result<(), ConsensusError> {
        match tip {
            Some((tip_height, tip_hash)) => self.validate_next_block(block, tip_height, tip_hash),
            None => self.validate_genesis_block(block),
        }
    }

    pub fn validate_proof_of_work(&self, block: &Block) -> Result<(), ConsensusError> {
        if self.config.difficulty == 0 {
            return Ok(());
        }

        if block.difficulty() != self.config.difficulty {
            return Err(ConsensusError::UnexpectedDifficulty);
        }

        let hash = proof_of_work_hash(block)?;
        self.validate_proof_of_work_hash_with_difficulty(&hash, block.difficulty())
    }

    pub fn validate_proof_of_work_hash(
        &self,
        hash: &ProofOfWorkHash,
    ) -> Result<(), ConsensusError> {
        self.validate_proof_of_work_hash_with_difficulty(hash, self.config.difficulty)
    }

    pub fn validate_proof_of_work_hash_with_difficulty(
        &self,
        hash: &ProofOfWorkHash,
        difficulty: u32,
    ) -> Result<(), ConsensusError> {
        if difficulty < MIN_DIFFICULTY {
            return Err(ConsensusError::InvalidDifficulty);
        }

        if hash_meets_difficulty(hash, difficulty) {
            Ok(())
        } else {
            Err(ConsensusError::InsufficientProofOfWork)
        }
    }

    pub fn proof_of_work_hash(&self, block: &Block) -> Result<ProofOfWorkHash, ConsensusError> {
        proof_of_work_hash(block)
    }

    pub fn retarget_difficulty(
        &self,
        current_difficulty: u32,
        first_timestamp: u64,
        last_timestamp: u64,
        block_count: u64,
    ) -> Result<u32, ConsensusError> {
        if current_difficulty < MIN_DIFFICULTY {
            return Err(ConsensusError::InvalidDifficulty);
        }

        if block_count < DIFFICULTY_ADJUSTMENT_INTERVAL {
            return Ok(current_difficulty);
        }

        let target_timespan = BLOCK_TIME as u64 * block_count;
        let min_timespan = target_timespan / DIFFICULTY_TIMESPAN_CLAMP_FACTOR;
        let max_timespan = target_timespan.saturating_mul(DIFFICULTY_TIMESPAN_CLAMP_FACTOR);
        let actual_timespan = last_timestamp
            .saturating_sub(first_timestamp)
            .clamp(min_timespan.max(1), max_timespan.max(1));

        let adjustment = difficulty_adjustment_bits(target_timespan, actual_timespan);
        let next = if adjustment >= 0 {
            current_difficulty.saturating_add(adjustment as u32)
        } else {
            current_difficulty.saturating_sub(adjustment.unsigned_abs())
        };

        Ok(next.max(MIN_DIFFICULTY))
    }
}

fn difficulty_adjustment_bits(target_timespan: u64, actual_timespan: u64) -> i32 {
    if target_timespan == 0 || actual_timespan == 0 {
        return MAX_DIFFICULTY_ADJUSTMENT_BITS as i32;
    }

    let mut adjustment = 0_i32;
    let mut fast_threshold = target_timespan / 2;
    while fast_threshold > 0
        && actual_timespan <= fast_threshold
        && adjustment < MAX_DIFFICULTY_ADJUSTMENT_BITS as i32
    {
        adjustment += 1;
        fast_threshold /= 2;
    }

    if adjustment > 0 {
        return adjustment;
    }

    let mut slow_threshold = target_timespan.saturating_mul(2);
    while actual_timespan >= slow_threshold && adjustment > -(MAX_DIFFICULTY_ADJUSTMENT_BITS as i32)
    {
        adjustment -= 1;
        let Some(next_threshold) = slow_threshold.checked_mul(2) else {
            break;
        };
        slow_threshold = next_threshold;
    }

    adjustment
}

fn proof_of_work_hash(block: &Block) -> Result<ProofOfWorkHash, ConsensusError> {
    let header_bytes =
        borsh::to_vec(&block.header).expect("block header serialization should not fail");
    argon2_proof_of_work_hash(&header_bytes)
        .map_err(|_| ConsensusError::InvalidProofOfWorkParameters)
}

#[allow(dead_code)]
fn _previous_hash_type_marker(hash: PreviousHash) -> PreviousHash {
    hash
}
