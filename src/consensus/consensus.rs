use crate::block::Block;
use crate::params::{
    BLOCK_TIME, DIFFICULTY_ADJUSTMENT_INTERVAL, DIFFICULTY_START, HASH_SIZE, MAX_DIFFICULTY,
    MAX_DIFFICULTY_ADJUSTMENT_BITS, MIN_DIFFICULTY, PROOF_OF_WORK_HASH_SIZE,
};
use crate::types::{BlockHash, BlockHeight, Hash, Height, PreviousHash, ProofOfWorkHash};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::error::ConsensusError;

const ARGON2_POW_SALT: &[u8] = b"paquscore-proof-of-work";
const ARGON2_POW_MEMORY_KIB: u32 = 512 * 1024; // 512MiB
const ARGON2_POW_TIME_COST: u32 = 1;
const ARGON2_POW_PARALLELISM: u32 = 2;
const ARGON2_POW_OUTPUT_LEN: usize = 32;

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
        if config.difficulty < MIN_DIFFICULTY || config.difficulty > MAX_DIFFICULTY {
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

        let hash = argon2_proof_of_work_hash(block)?;
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
        if !(MIN_DIFFICULTY..=MAX_DIFFICULTY).contains(&difficulty) {
            return Err(ConsensusError::InvalidDifficulty);
        }

        if hash_meets_difficulty(hash, difficulty) {
            Ok(())
        } else {
            Err(ConsensusError::InsufficientProofOfWork)
        }
    }

    pub fn proof_of_work_hash(&self, block: &Block) -> Result<ProofOfWorkHash, ConsensusError> {
        argon2_proof_of_work_hash(block)
    }

    pub fn retarget_difficulty(
        &self,
        current_difficulty: u32,
        first_timestamp: u64,
        last_timestamp: u64,
        block_count: u64,
    ) -> Result<u32, ConsensusError> {
        if !(MIN_DIFFICULTY..=MAX_DIFFICULTY).contains(&current_difficulty) {
            return Err(ConsensusError::InvalidDifficulty);
        }

        if block_count < DIFFICULTY_ADJUSTMENT_INTERVAL {
            return Ok(current_difficulty);
        }

        let target_timespan = BLOCK_TIME as u64 * block_count;
        let min_timespan = target_timespan / crate::params::DIFFICULTY_TIMESPAN_CLAMP_FACTOR;
        let max_timespan =
            target_timespan.saturating_mul(crate::params::DIFFICULTY_TIMESPAN_CLAMP_FACTOR);
        let actual_timespan = last_timestamp
            .saturating_sub(first_timestamp)
            .clamp(min_timespan.max(1), max_timespan.max(1));

        let adjustment = difficulty_adjustment_bits(target_timespan, actual_timespan);
        let next = if adjustment >= 0 {
            current_difficulty.saturating_add(adjustment as u32)
        } else {
            current_difficulty.saturating_sub(adjustment.unsigned_abs())
        };

        Ok(next.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY))
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

fn argon2_proof_of_work_hash(block: &Block) -> Result<ProofOfWorkHash, ConsensusError> {
    let params = Params::new(
        ARGON2_POW_MEMORY_KIB,
        ARGON2_POW_TIME_COST,
        ARGON2_POW_PARALLELISM,
        Some(ARGON2_POW_OUTPUT_LEN),
    )
    .map_err(|_| ConsensusError::InvalidProofOfWorkParameters)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let header_bytes =
        borsh::to_vec(&block.header).expect("block header serialization should not fail");
    let mut output = [0_u8; PROOF_OF_WORK_HASH_SIZE];

    argon2
        .hash_password_into(&header_bytes, ARGON2_POW_SALT, &mut output)
        .map_err(|_| ConsensusError::InvalidProofOfWorkParameters)?;

    Ok(ProofOfWorkHash(output))
}

fn hash_meets_difficulty(hash: &ProofOfWorkHash, difficulty: u32) -> bool {
    let full_zero_bytes = (difficulty / 8) as usize;
    let remaining_zero_bits = (difficulty % 8) as u8;

    if full_zero_bytes > hash.0.len() {
        return false;
    }

    if !hash.0.iter().take(full_zero_bytes).all(|byte| *byte == 0) {
        return false;
    }

    if remaining_zero_bits == 0 {
        return true;
    }

    let Some(next_byte) = hash.0.get(full_zero_bytes) else {
        return false;
    };
    let mask = 0xff << (8 - remaining_zero_bits);
    next_byte & mask == 0
}

#[allow(dead_code)]
fn _previous_hash_type_marker(hash: PreviousHash) -> PreviousHash {
    hash
}
