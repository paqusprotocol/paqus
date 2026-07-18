use crate::block::{Block, BlockError};
use crate::block::{Height, Nonce};
use crate::consensus::supply::Amount;
use crate::consensus::supply::{BLOCK_REWARD, TAIL_EMISSION};
use crate::consensus::{
    ASERT_HALF_LIFE, BLOCK_TIME, Consensus, ConsensusConfig, ConsensusError,
    TAIL_EMISSION_START_HEIGHT, block_reward, tail_emission_start_height,
};
use crate::crypto::Address;
use crate::crypto::{
    BlockHash, Hash, PreviousHash, ProofOfWorkHash, address_from_public_key, generate_keypair, sign,
};
use crate::transaction::{SignedTransaction, Transaction};

const TEST_FEE: u64 = 2;

fn signed_transaction(nonce: u64) -> SignedTransaction {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let transaction = Transaction::new(
        from,
        Address([2; 20]),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(nonce),
    );
    let signature = sign(&keypair.secret_key, &transaction.signing_bytes());

    SignedTransaction::new(transaction, keypair.public_key, signature)
}

fn block(height: u64, previous_hash: impl Into<PreviousHash>) -> Block {
    let previous_hash = previous_hash.into();
    let transactions = if height == 0 && previous_hash == Hash([0; crate::crypto::HASH_SIZE]) {
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
fn accepts_config_above_pow_hash_bit_width() {
    assert_eq!(
        Consensus::new(ConsensusConfig { difficulty: 257 })
            .unwrap()
            .config
            .difficulty,
        257
    );
}

#[test]
fn validates_candidate_genesis_without_pow_when_difficulty_is_zero_for_test() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let genesis = block(0, Hash([0; crate::crypto::HASH_SIZE]));

    assert_eq!(consensus.validate_candidate_block(&genesis, None), Ok(()));
}

#[test]
fn rejects_non_genesis_first_block() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let first = block(1, Hash([0; crate::crypto::HASH_SIZE]));

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
    let genesis = block(0, Hash([0; crate::crypto::HASH_SIZE]));
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
    let genesis = block(0, Hash([0; crate::crypto::HASH_SIZE]));
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
    let genesis = block(0, Hash([0; crate::crypto::HASH_SIZE]));
    let mut next = block(1, genesis.hash());
    let now = genesis.timestamp();
    next.header.timestamp = now + crate::consensus::MAX_FUTURE_TIME as u64 + 1;

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
    let next = block(1, Hash([9; crate::crypto::HASH_SIZE]));

    assert_eq!(
        consensus.validate_candidate_block(
            &next,
            Some((Height(0), BlockHash([1; crate::crypto::HASH_SIZE])))
        ),
        Err(ConsensusError::InvalidPreviousHash)
    );
}

#[test]
fn rejects_block_difficulty_mismatch() {
    let consensus = Consensus::new(ConsensusConfig { difficulty: 2 }).unwrap();
    let block = block(0, Hash([0; crate::crypto::HASH_SIZE]));

    assert_eq!(
        consensus.validate_proof_of_work(&block),
        Err(ConsensusError::UnexpectedDifficulty)
    );
}

#[test]
fn checks_proof_of_work_zero_bit_difficulty() {
    let consensus = Consensus::new(ConsensusConfig { difficulty: 9 }).unwrap();
    let mut valid = [0_u8; crate::crypto::PROOF_OF_WORK_HASH_SIZE];
    valid[1] = 0b0111_1111;
    let mut invalid = valid;
    invalid[1] = 0b1000_0000;

    assert_eq!(
        consensus.validate_proof_of_work_hash(&ProofOfWorkHash(valid)),
        Ok(())
    );
    assert_eq!(
        consensus.validate_proof_of_work_hash(&ProofOfWorkHash(invalid)),
        Err(ConsensusError::InsufficientProofOfWork)
    );
}

#[test]
fn treats_difficulty_above_hash_bit_width_as_unmet_pow() {
    let consensus = Consensus::with_default_config();

    assert_eq!(
        consensus.validate_proof_of_work_hash_with_difficulty(&ProofOfWorkHash([0; 64]), 512),
        Ok(())
    );
    assert_eq!(
        consensus.validate_proof_of_work_hash_with_difficulty(&ProofOfWorkHash([0; 64]), 513),
        Err(ConsensusError::InsufficientProofOfWork)
    );
}

#[test]
fn proof_of_work_hash_is_sha3_512_based_and_deterministic() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let block = block(0, Hash([0; crate::crypto::HASH_SIZE]));

    let hash = consensus.proof_of_work_hash(&block).unwrap();

    assert_eq!(hash.0.len(), 64);
    assert_eq!(hash, consensus.proof_of_work_hash(&block).unwrap());
    assert_ne!(hash.0.as_slice(), block.hash().0.as_slice());
    assert_eq!(consensus.validate_proof_of_work(&block), Ok(()));
}

#[test]
fn asert_keeps_difficulty_on_schedule() {
    let consensus = Consensus::with_default_config();
    let blocks = ASERT_HALF_LIFE / BLOCK_TIME as u64;

    assert_eq!(
        consensus.asert_difficulty(
            10,
            1_700_000_000,
            Height(0),
            1_700_000_000 + ASERT_HALF_LIFE,
            Height(blocks),
        ),
        Ok(10)
    );
}

#[test]
fn asert_adjusts_from_anchor_for_hashrate_swings() {
    let consensus = Consensus::with_default_config();
    let blocks = ASERT_HALF_LIFE / BLOCK_TIME as u64;

    assert_eq!(
        consensus.asert_difficulty(10, 0, Height(0), 0, Height(blocks)),
        Ok(11)
    );
    assert_eq!(
        consensus.asert_difficulty(10, 0, Height(0), ASERT_HALF_LIFE * 2, Height(blocks),),
        Ok(9)
    );
}

#[test]
fn asert_clamps_to_minimum_and_rejects_invalid_anchor() {
    let consensus = Consensus::with_default_config();

    assert_eq!(
        consensus.asert_difficulty(1, 0, Height(0), ASERT_HALF_LIFE * 10, Height(1)),
        Ok(crate::consensus::MIN_DIFFICULTY)
    );
    assert_eq!(
        consensus.asert_difficulty(0, 0, Height(0), 0, Height(0)),
        Err(ConsensusError::InvalidDifficulty)
    );
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
