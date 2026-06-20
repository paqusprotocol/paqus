use super::{Block, BlockError};
use crate::params::{BASE_FEE, MAX_FUTURE_TIME};
use crate::transaction::{SignedTransaction, Transaction};
use crate::types::{Address, Amount, Hash, Height, Nonce, PublicKey, Signature};

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

fn miner() -> Address {
    Address([9; 20])
}

#[test]
fn validates_block_with_matching_merkle_root() {
    let block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1), signed_transaction(2)],
    );

    assert_eq!(block.validate(), Ok(()));
    assert_eq!(block.header.merkle_root, block.calculate_merkle_root());
}

#[test]
fn rejects_unsupported_block_version() {
    let mut block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1)],
    );
    block.header.version = crate::params::BLOCK_VERSION + 1;

    assert_eq!(block.validate(), Err(BlockError::UnsupportedVersion));
}

#[test]
fn rejects_empty_blocks() {
    let block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![],
    );

    assert_eq!(block.validate(), Err(BlockError::EmptyTransactions));
}

#[test]
fn allows_empty_genesis_block() {
    let block = Block::new(
        Height(0),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![],
    );

    assert_eq!(block.validate(), Ok(()));
    assert_eq!(block.calculate_merkle_root(), Hash([0; 64]));
}

#[test]
fn rejects_tampered_merkle_root() {
    let mut block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1)],
    );
    block.header.merkle_root = Hash([9; 64]);

    assert_eq!(block.validate(), Err(BlockError::InvalidMerkleRoot));
}

#[test]
fn refreshes_merkle_root_after_push() {
    let mut block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1)],
    );
    let old_root = block.header.merkle_root;

    block.push_transaction(signed_transaction(2));

    assert_eq!(block.transaction_count(), 2);
    assert_ne!(block.header.merkle_root, old_root);
    assert_eq!(block.validate(), Ok(()));
}

#[test]
fn rejects_future_timestamp() {
    let block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000 + MAX_FUTURE_TIME as u64 + 1,
        Nonce(42),
        vec![signed_transaction(1)],
    );

    assert_eq!(
        block.validate_at(1_700_000_000),
        Err(BlockError::FutureTimestamp)
    );
}

#[test]
fn reports_miner_revenue_from_subsidy_and_fees() {
    let block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1), signed_transaction(2)],
    );
    let revenue = block.miner_revenue(Amount(2_500));

    assert_eq!(revenue.subsidy, Amount(2_500));
    assert_eq!(revenue.fees, Amount(BASE_FEE * 2));
}
