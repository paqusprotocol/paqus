use super::{Consensus, ConsensusConfig, ConsensusError, block_reward, tail_emission_start_height};
use crate::block::{Block, BlockError};
use crate::params::{
    BASE_FEE, BLOCK_REWARD, DIFFICULTY_ADJUSTMENT_INTERVAL, GENESIS_PREMINE, MAX_MINED_SUPPLY,
    MAX_UNIT_SUPPLY, TAIL_EMISSION, TAIL_EMISSION_START_HEIGHT,
};
use crate::transaction::{SignedTransaction, Transaction};
use crate::types::{Address, Amount, Hash, Height, Nonce, ProofOfWorkHash, PublicKey, Signature};

fn signed_transaction(nonce: u64) -> SignedTransaction {
    SignedTransaction::new(
        Transaction::new(
            Address([1; 20]),
            Address([2; 20]),
            Amount(10),
            Amount(BASE_FEE),
            Nonce(nonce),
        ),
        PublicKey([1; 2592]),
        Signature([1; 4627]),
    )
}

fn block(height: u64, previous_hash: Hash) -> Block {
    let transactions = if height == 0 && previous_hash == Hash([0; 64]) {
        vec![]
    } else {
        vec![signed_transaction(height)]
    };

    Block::new(
        Height(height),
        previous_hash,
        Address([9; 20]),
        1_700_000_000 + height,
        Nonce(0),
        transactions,
    )
}

#[test]
fn accepts_valid_config() {
    assert_eq!(
        Consensus::new(ConsensusConfig { difficulty: 1 })
            .unwrap()
            .config
            .difficulty,
        1
    );
}

#[test]
fn rejects_invalid_config() {
    assert_eq!(
        Consensus::new(ConsensusConfig { difficulty: 0 }),
        Err(ConsensusError::InvalidDifficulty)
    );
}

#[test]
fn validates_candidate_genesis_without_pow_when_difficulty_is_zero_for_test() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let genesis = block(0, Hash([0; 64]));

    assert_eq!(consensus.validate_candidate_block(&genesis, None), Ok(()));
}

#[test]
fn rejects_non_genesis_first_block() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let first = block(1, Hash([0; 64]));

    assert_eq!(
        consensus.validate_candidate_block(&first, None),
        Err(ConsensusError::InvalidHeight)
    );
}

#[test]
fn validates_next_block_linkage() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let genesis = block(0, Hash([0; 64]));
    let next = block(1, genesis.hash());

    assert_eq!(
        consensus.validate_candidate_block(&next, Some((genesis.height(), genesis.hash()))),
        Ok(())
    );
}

#[test]
fn rejects_next_block_timestamp_earlier_than_tip() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let genesis = block(0, Hash([0; 64]));
    let mut next = block(1, genesis.hash());
    next.header.timestamp = genesis.timestamp().saturating_sub(1);

    assert_eq!(
        consensus.validate_next_block_with_tip(&next, &genesis),
        Err(ConsensusError::InvalidTimestamp)
    );
}

#[test]
fn rejects_next_block_timestamp_too_far_in_future() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let genesis = block(0, Hash([0; 64]));
    let mut next = block(1, genesis.hash());
    let now = genesis.timestamp();
    next.header.timestamp = now + crate::params::MAX_FUTURE_TIME as u64 + 1;

    assert_eq!(
        consensus.validate_next_block_with_tip_at(&next, &genesis, now),
        Err(ConsensusError::InvalidBlock(BlockError::FutureTimestamp))
    );
}

#[test]
fn rejects_wrong_previous_hash() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let next = block(1, Hash([9; 64]));

    assert_eq!(
        consensus.validate_candidate_block(&next, Some((Height(0), Hash([1; 64])))),
        Err(ConsensusError::InvalidPreviousHash)
    );
}

#[test]
fn checks_proof_of_work_zero_byte_difficulty() {
    let consensus = Consensus::new(ConsensusConfig { difficulty: 2 }).unwrap();

    assert_eq!(
        consensus.validate_proof_of_work_hash(&ProofOfWorkHash([
            0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
            23, 24, 25, 26, 27, 28, 29, 30
        ])),
        Ok(())
    );
    assert_eq!(
        consensus.validate_proof_of_work_hash(&ProofOfWorkHash([
            0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
            23, 24, 25, 26, 27, 28, 29, 30
        ])),
        Err(ConsensusError::InsufficientProofOfWork)
    );
}

#[test]
fn proof_of_work_hash_is_argon2_based_and_deterministic() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let block = block(0, Hash([0; 64]));

    let hash = consensus.proof_of_work_hash(&block).unwrap();

    assert_eq!(hash.0.len(), 32);
    assert_eq!(hash, consensus.proof_of_work_hash(&block).unwrap());
    assert_ne!(hash.0.as_slice(), block.hash().0.as_slice());
    assert_eq!(consensus.validate_proof_of_work(&block), Ok(()));
}

#[test]
fn retargets_difficulty_from_block_timespan() {
    let consensus = Consensus::with_default_config();

    assert_eq!(
        consensus.retarget_difficulty(2, 0, 150, DIFFICULTY_ADJUSTMENT_INTERVAL),
        Ok(3)
    );
    assert_eq!(
        consensus.retarget_difficulty(2, 0, 450, DIFFICULTY_ADJUSTMENT_INTERVAL),
        Ok(1)
    );
    assert_eq!(
        consensus.retarget_difficulty(2, 0, 300, DIFFICULTY_ADJUSTMENT_INTERVAL),
        Ok(2)
    );
    assert_eq!(consensus.retarget_difficulty(2, 0, 10, 0), Ok(2));
}

#[test]
fn uses_block_reward_until_tail_emission_starts() {
    assert_eq!(block_reward(Height(0)), Amount(BLOCK_REWARD));
    assert_eq!(
        block_reward(Height(tail_emission_start_height().saturating_sub(1))),
        Amount(BLOCK_REWARD)
    );
    assert_eq!(
        block_reward(Height(tail_emission_start_height())),
        Amount(TAIL_EMISSION)
    );
    assert_eq!(tail_emission_start_height(), TAIL_EMISSION_START_HEIGHT);
}

#[test]
fn mined_supply_excludes_genesis_premine_from_max_unit_supply() {
    assert_eq!(
        MAX_MINED_SUPPLY,
        MAX_UNIT_SUPPLY.saturating_sub(GENESIS_PREMINE)
    );
}
