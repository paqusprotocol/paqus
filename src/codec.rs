use crate::block::{Block, BlockHeader};
use crate::crypto::{BlockHash, HASH_SIZE, StateRoot, TransactionHash};
pub use crate::crypto::{HashDomain, domain_hash, hash_bytes};
use crate::error::CodecError;
use crate::transaction::{SignedTransaction, Transaction};
use borsh::{BorshDeserialize, BorshSerialize};

pub fn canonical_bytes<T: BorshSerialize>(value: &T) -> Vec<u8> {
    borsh::to_vec(value).expect("canonical serialization should not fail")
}

/// Canonically deserializes bytes without applying domain validation.
pub fn canonical_deserialize<T: BorshDeserialize>(bytes: &[u8]) -> Result<T, CodecError> {
    T::try_from_slice(bytes).map_err(|_| CodecError::DecodeFailed)
}

/// Alias for canonical deserialization. This does not imply consensus validity.
pub fn canonical_decode<T: BorshDeserialize>(bytes: &[u8]) -> Result<T, CodecError> {
    canonical_deserialize(bytes)
}

pub fn transaction_bytes(transaction: &Transaction) -> Vec<u8> {
    canonical_bytes(transaction)
}

pub fn signed_transaction_bytes(transaction: &SignedTransaction) -> Vec<u8> {
    canonical_bytes(transaction)
}

pub fn block_header_bytes(header: &BlockHeader) -> Vec<u8> {
    canonical_bytes(header)
}

pub fn block_bytes(block: &Block) -> Vec<u8> {
    canonical_bytes(block)
}

pub fn state_root_bytes(state_root: &StateRoot) -> [u8; HASH_SIZE] {
    state_root.0
}

pub fn transaction_hash(transaction: &Transaction) -> TransactionHash {
    TransactionHash(domain_hash(HashDomain::Transaction, &transaction_bytes(transaction)).0)
}

pub fn signed_transaction_hash(transaction: &SignedTransaction) -> TransactionHash {
    TransactionHash(
        domain_hash(
            HashDomain::SignedTransaction,
            &signed_transaction_bytes(transaction),
        )
        .0,
    )
}

pub fn block_header_hash(header: &BlockHeader) -> BlockHash {
    BlockHash(domain_hash(HashDomain::BlockHeader, &block_header_bytes(header)).0)
}

pub fn decode_transaction(bytes: &[u8]) -> Result<Transaction, CodecError> {
    let transaction: Transaction = canonical_deserialize(bytes)?;
    transaction
        .validate()
        .map_err(|_| CodecError::InvalidTransaction)?;
    Ok(transaction)
}

/// Decodes a signed transaction and verifies its signature and sender address.
pub fn decode_signed_transaction(bytes: &[u8]) -> Result<SignedTransaction, CodecError> {
    let transaction: SignedTransaction = canonical_deserialize(bytes)?;
    transaction
        .validate_signed()
        .map_err(|_| CodecError::InvalidTransaction)?;
    Ok(transaction)
}

/// Decodes a structurally valid block.
///
/// This validates block-local rules, including transaction signatures, merkle root, size, and
/// timestamp bounds. It does not validate proof of work, parent linkage, ledger state root, fork
/// choice, or coinbase subsidy against a ledger.
pub fn decode_block(bytes: &[u8]) -> Result<Block, CodecError> {
    let block: Block = canonical_deserialize(bytes)?;
    block.validate().map_err(|_| CodecError::InvalidBlock)?;
    Ok(block)
}
