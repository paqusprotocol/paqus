use crate::block::{Block, BlockError};
use crate::block::{Height, Nonce};
use crate::consensus::MAX_FUTURE_TIME;
use crate::consensus::supply::Amount;
use crate::crypto::Address;
use crate::crypto::Hash;
use crate::crypto::{address_from_public_key, generate_keypair, sign};
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
    block.header.version += 1;

    assert_eq!(block.validate(), Err(BlockError::UnsupportedVersion));
}

#[test]
fn validates_coinbase_only_blocks() {
    let block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![],
    );

    assert_eq!(block.validate(), Ok(()));
    assert!(block.coinbase.is_some());
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
    block.header.merkle_root = Hash([9; 64]).into();

    assert_eq!(block.validate(), Err(BlockError::InvalidMerkleRoot));
}

#[test]
fn rejects_transaction_with_invalid_signature() {
    let mut transaction = signed_transaction(1);
    transaction.witness.signature.0[0] ^= 0xff;
    let block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![transaction],
    );

    assert_eq!(block.validate(), Err(BlockError::InvalidTransaction));
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
    assert_eq!(revenue.fees, Amount(TEST_FEE * 2));
}

#[test]
fn rejects_fee_overflow() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let transaction = Transaction::new(
        from,
        Address([2; 20]),
        Amount(10),
        Amount(u64::MAX),
        Nonce(0),
    );
    let tx = SignedTransaction::new(
        transaction.clone(),
        keypair.public_key,
        sign(&keypair.secret_key, &transaction.signing_bytes()),
    );
    let block = Block::new(
        Height(1),
        Hash([1; 64]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![tx.clone(), tx],
    );

    assert_eq!(block.checked_total_fees(), Err(BlockError::FeeOverflow));
    assert_eq!(block.validate(), Err(BlockError::FeeOverflow));
}
