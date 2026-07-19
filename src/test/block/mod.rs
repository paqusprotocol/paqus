use crate::block::{Block, BlockError};
use crate::block::{Height, Nonce};
use crate::codec::{block_bytes, decode_block};
use crate::consensus::MAX_FUTURE_TIME;
use crate::consensus::supply::Amount;
use crate::crypto::Address;
use crate::crypto::Hash;
use crate::crypto::{address_from_public_key, generate_keypair, sign};
use crate::transaction::{
    QCashTransaction, SignedQCashTransaction, SignedTransaction, Transaction,
};

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

fn signed_transaction_for_keypair(
    keypair: &crate::crypto::KeyPair,
    nonce: u64,
) -> SignedTransaction {
    let transaction = Transaction::new(
        address_from_public_key(&keypair.public_key),
        Address([(nonce as u8).saturating_add(2); 20]),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(nonce),
    );
    let signature = sign(&keypair.secret_key, &transaction.signing_bytes());
    SignedTransaction::new(transaction, keypair.public_key, signature)
}

#[test]
fn block_wire_deduplicates_repeated_witness_public_keys() {
    let keypair = generate_keypair();
    let transactions = vec![
        signed_transaction_for_keypair(&keypair, 1),
        signed_transaction_for_keypair(&keypair, 2),
        signed_transaction_for_keypair(&keypair, 3),
    ];
    let standalone_size = transactions
        .iter()
        .map(SignedTransaction::serialized_size)
        .sum::<usize>();
    let block = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(42),
        transactions,
    );
    let encoded = block_bytes(&block);
    assert_eq!(decode_block(&encoded).unwrap(), block);
    assert!(standalone_size.saturating_sub(encoded.len()) > crate::crypto::PUBLIC_KEY_SIZE);
}

fn miner() -> Address {
    Address([9; 20])
}

#[test]
fn validates_block_with_matching_merkle_root() {
    let block = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
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
        Hash([0; crate::crypto::HASH_SIZE]),
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
        Hash([0; crate::crypto::HASH_SIZE]),
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
        Hash([0; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![],
    );

    assert_eq!(block.validate(), Ok(()));
    assert_eq!(
        block.calculate_merkle_root(),
        Hash([0; crate::crypto::HASH_SIZE])
    );
}

#[test]
fn rejects_tampered_merkle_root() {
    let mut block = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1)],
    );
    block.header.merkle_root = Hash([9; crate::crypto::HASH_SIZE]).into();

    assert_eq!(block.validate(), Err(BlockError::InvalidMerkleRoot));
}

#[test]
fn rejects_tampered_witness_root() {
    let mut block = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1)],
    );
    block.header.witness_root.0[0] ^= 0xff;

    assert_eq!(block.validate(), Err(BlockError::InvalidWitnessRoot));
}

#[test]
fn segwit_keeps_txid_stable_and_commits_witness_variants() {
    let transaction = signed_transaction(1);
    let mut alternate_witness = transaction.clone();
    alternate_witness.witness.signature.0[0] ^= 0xff;

    assert_eq!(transaction.txid(), alternate_witness.txid());
    assert_ne!(transaction.wtxid(), alternate_witness.wtxid());

    let original = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![transaction],
    );
    let alternate = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![alternate_witness],
    );

    assert_eq!(original.header.merkle_root, alternate.header.merkle_root);
    assert_ne!(original.header.witness_root, alternate.header.witness_root);
    assert_ne!(original.hash(), alternate.hash());
}

#[test]
fn segwit_wire_format_roundtrips_and_rejects_section_length_mismatch() {
    let block = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(42),
        vec![signed_transaction(1)],
    );
    let bytes = block_bytes(&block);

    assert_eq!(
        block.serialized_size(),
        block.stripped_size() + block.witness_size()
    );
    assert_eq!(decode_block(&bytes).unwrap(), block);

    let mut mismatched = bytes;
    let first_witness_length = block.stripped_size();
    mismatched[first_witness_length..first_witness_length + 4]
        .copy_from_slice(&0_u32.to_le_bytes());
    assert!(decode_block(&mismatched).is_err());
}

#[test]
fn rejects_transaction_with_invalid_signature() {
    let mut transaction = signed_transaction(1);
    transaction.witness.signature.0[0] ^= 0xff;
    let block = Block::new(
        Height(1),
        Hash([0; crate::crypto::HASH_SIZE]),
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
        Hash([0; crate::crypto::HASH_SIZE]),
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
        Hash([0; crate::crypto::HASH_SIZE]),
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
        Hash([0; crate::crypto::HASH_SIZE]),
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
        Hash([1; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![tx.clone(), tx],
    );

    assert_eq!(block.checked_total_fees(), Err(BlockError::FeeOverflow));
    assert_eq!(block.validate(), Err(BlockError::FeeOverflow));
}

#[test]
fn block_commits_transfer_qcash_and_witnesses() {
    use crate::consensus::supply::XPQ;
    use crate::qcash::{CashDenomination, WithdrawCashMetadata, cash_coin_commitment};

    let keypair = generate_keypair();
    let signer = address_from_public_key(&keypair.public_key);
    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(XPQ),
        &[CashDenomination::One],
        &[cash_coin_commitment(&[90; 32])],
    )
    .unwrap();
    let transaction =
        QCashTransaction::withdraw(signer, Amount(XPQ), Amount(3), Nonce(0), metadata);
    let signature = sign(&keypair.secret_key, &transaction.signing_bytes());
    let qcash = SignedQCashTransaction::new(transaction, keypair.public_key, signature);
    let mut block = Block::with_all_transactions(
        Height(1),
        Hash([1; crate::crypto::HASH_SIZE]),
        miner(),
        1,
        1_700_000_000,
        Nonce(1),
        vec![signed_transaction(0)],
        vec![qcash],
    )
    .unwrap();

    assert_eq!(block.header.version, crate::block::BLOCK_VERSION);
    assert_eq!(block.transaction_count(), 2);
    assert_eq!(block.checked_total_fees(), Ok(Amount(TEST_FEE + 3)));
    assert_eq!(block.coinbase.as_ref().unwrap().fees, Amount(TEST_FEE + 3));
    assert_eq!(block.validate(), Ok(()));

    block.header.merkle_root.0[0] ^= 0xff;
    assert_eq!(block.validate(), Err(BlockError::InvalidMerkleRoot));
}

#[test]
fn uses_the_segwit_format_immediately_after_genesis() {
    let block = Block::new(
        Height(1),
        Hash([1; crate::crypto::HASH_SIZE]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    assert_eq!(block.header.version, crate::block::BLOCK_VERSION);
    assert_eq!(block.validate(), Ok(()));

    assert_eq!(
        Block::with_all_transactions(
            Height(0),
            Hash([1; crate::crypto::HASH_SIZE]),
            miner(),
            1,
            1_700_000_000,
            Nonce(0),
            vec![],
            vec![],
        ),
        Err(BlockError::InvalidTransaction)
    );
}
