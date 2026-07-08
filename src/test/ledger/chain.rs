use super::{Chain, LedgerError};
use crate::block::Block;
use crate::block::{Height, Nonce};
use crate::consensus::supply::Amount;
use crate::crypto::{Address, PublicKey, Signature};
use crate::crypto::{Hash, PreviousHash};
use crate::transaction::{SignedTransaction, Transaction};

fn signed_transaction(nonce: u64) -> SignedTransaction {
    SignedTransaction::new(
        Transaction::new(
            Address([1; 20]),
            Address([2; 20]),
            Amount(10),
            Amount(1),
            Nonce(nonce),
        ),
        PublicKey([1; 2592]),
        Signature([1; 4627]),
    )
}

fn block(height: u64, previous_hash: impl Into<PreviousHash>) -> Block {
    Block::new(
        Height(height),
        previous_hash,
        Address([9; 20]),
        1_700_000_000 + height,
        Nonce(0),
        vec![signed_transaction(height)],
    )
}

fn block_at(height: u64, previous_hash: impl Into<PreviousHash>, timestamp: u64) -> Block {
    Block::new(
        Height(height),
        previous_hash,
        Address([9; 20]),
        timestamp,
        Nonce(0),
        vec![signed_transaction(height)],
    )
}

#[test]
fn inserts_genesis_and_tracks_tip() {
    let mut chain = Chain::new();
    let genesis = block(0, Hash([0; crate::crypto::HASH_SIZE]));
    let genesis_hash = genesis.hash();

    assert_eq!(chain.insert_block(genesis), Ok(()));
    assert_eq!(chain.tip_height(), Some(Height(0)));
    assert_eq!(chain.tip_hash(), Some(genesis_hash));
    assert!(chain.has_blocks());
}

#[test]
fn rejects_duplicate_height() {
    let mut chain = Chain::new();
    chain
        .insert_block(block(0, Hash([0; crate::crypto::HASH_SIZE])))
        .unwrap();

    assert_eq!(
        chain.insert_block(block(0, Hash([0; crate::crypto::HASH_SIZE]))),
        Err(LedgerError::DuplicateBlock)
    );
}

#[test]
fn rejects_wrong_link() {
    let mut chain = Chain::new();
    chain
        .insert_block(block(0, Hash([0; crate::crypto::HASH_SIZE])))
        .unwrap();

    assert_eq!(
        chain.insert_block(block(1, Hash([9; crate::crypto::HASH_SIZE]))),
        Err(LedgerError::InvalidParent)
    );
}

#[test]
fn rejects_timestamp_earlier_than_tip() {
    let mut chain = Chain::new();
    let genesis = block_at(0, Hash([0; crate::crypto::HASH_SIZE]), 1_700_000_010);
    let genesis_hash = genesis.hash();
    chain.insert_block(genesis).unwrap();

    assert_eq!(
        chain.insert_block(block_at(1, genesis_hash, 1_700_000_000)),
        Err(LedgerError::InvalidTimestamp)
    );
}
