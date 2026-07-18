use crate::block::{Block, BlockHeader, BlockHeight, MAX_BLOCK_SIZE};
use crate::crypto::{BlockHash, HASH_SIZE, StateRoot, TransactionHash, WitnessTransactionHash};
pub use crate::crypto::{HashDomain, domain_hash, hash_bytes};
use crate::error::CodecError;
use crate::event::{MAX_PROTOCOL_EVENT_SIZE, ProtocolEvent};
use crate::transaction::{
    EcashTransaction, SignedEcashTransaction, SignedProtocolTransaction, SignedTransaction,
    Transaction, TransactionFamily,
};
use borsh::{BorshDeserialize, BorshSerialize};

/// Frozen consensus encoding profile. Changing any on-chain Borsh layout requires a new version.
pub const CANONICAL_ENCODING_VERSION: u8 = 1;
pub const CANONICAL_ENCODING_PROFILE: &str = "paqus-borsh-le";

/// Consensus-critical serialization. Do not replace or wrap this format under encoding version 1.
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

pub fn signed_protocol_transaction_bytes(transaction: &SignedProtocolTransaction) -> Vec<u8> {
    canonical_bytes(transaction)
}

pub fn signed_protocol_transaction_hash(
    transaction: &SignedProtocolTransaction,
) -> WitnessTransactionHash {
    WitnessTransactionHash(
        domain_hash(
            HashDomain::WitnessTransaction,
            &signed_protocol_transaction_bytes(transaction),
        )
        .0,
    )
}

/// Hashes canonical signed-family bytes with their unified envelope tag.
pub fn family_witness_transaction_hash(
    family: TransactionFamily,
    signed_family_bytes: &[u8],
) -> WitnessTransactionHash {
    let mut bytes = Vec::with_capacity(1 + signed_family_bytes.len());
    bytes.push(family.envelope_tag());
    bytes.extend_from_slice(signed_family_bytes);
    WitnessTransactionHash(domain_hash(HashDomain::WitnessTransaction, &bytes).0)
}

pub fn protocol_event_bytes(event: &ProtocolEvent) -> Vec<u8> {
    canonical_bytes(event)
}

pub fn decode_protocol_event(bytes: &[u8]) -> Result<ProtocolEvent, CodecError> {
    if bytes.len() > MAX_PROTOCOL_EVENT_SIZE {
        return Err(CodecError::DecodeFailed);
    }
    let event: ProtocolEvent = canonical_deserialize(bytes)?;
    if !event.validate() {
        return Err(CodecError::DecodeFailed);
    }
    Ok(event)
}

pub fn block_header_bytes(header: &BlockHeader) -> Vec<u8> {
    canonical_bytes(header)
}

pub fn block_bytes(block: &Block) -> Vec<u8> {
    canonical_bytes(block)
}

/// Canonical header and payload sections without the trailing witness sections.
pub fn stripped_block_bytes(block: &Block) -> Vec<u8> {
    let mut bytes = Vec::new();
    crate::block::serialize_stripped_block(block, &mut bytes)
        .expect("canonical stripped block serialization should not fail");
    bytes
}

pub fn state_root_bytes(state_root: &StateRoot) -> [u8; HASH_SIZE] {
    state_root.0
}

pub fn transaction_hash(transaction: &Transaction) -> TransactionHash {
    TransactionHash(domain_hash(HashDomain::Transaction, &transaction_bytes(transaction)).0)
}

pub fn signed_transaction_hash(transaction: &SignedTransaction) -> WitnessTransactionHash {
    family_witness_transaction_hash(
        TransactionFamily::Transfer,
        &signed_transaction_bytes(transaction),
    )
}

pub fn block_header_hash(header: &BlockHeader) -> BlockHash {
    BlockHash(domain_hash(HashDomain::BlockHeader, &block_header_bytes(header)).0)
}

pub fn decode_transaction(bytes: &[u8]) -> Result<Transaction, CodecError> {
    if bytes.len() > crate::transaction::MAX_TX_SIZE {
        return Err(CodecError::InvalidTransaction);
    }
    let transaction: Transaction = canonical_deserialize(bytes)?;
    transaction
        .validate()
        .map_err(|_| CodecError::InvalidTransaction)?;
    Ok(transaction)
}

/// Decodes a signed transaction and verifies its signature and sender address.
pub fn decode_signed_transaction(bytes: &[u8]) -> Result<SignedTransaction, CodecError> {
    if bytes.len() > crate::transaction::MAX_TX_SIZE {
        return Err(CodecError::InvalidTransaction);
    }
    let transaction: SignedTransaction = canonical_deserialize(bytes)?;
    transaction
        .validate_signed()
        .map_err(|_| CodecError::InvalidTransaction)?;
    Ok(transaction)
}

pub fn decode_ecash_transaction(bytes: &[u8]) -> Result<EcashTransaction, CodecError> {
    if bytes.len() > crate::transaction::ecash::MAX_ECASH_TX_SIZE {
        return Err(CodecError::InvalidTransaction);
    }
    let transaction: EcashTransaction = canonical_deserialize(bytes)?;
    transaction
        .validate()
        .map_err(|_| CodecError::InvalidTransaction)?;
    Ok(transaction)
}

pub fn decode_signed_ecash_transaction(bytes: &[u8]) -> Result<SignedEcashTransaction, CodecError> {
    if bytes.len() > crate::transaction::ecash::MAX_ECASH_TX_SIZE {
        return Err(CodecError::InvalidTransaction);
    }
    let transaction: SignedEcashTransaction = canonical_deserialize(bytes)?;
    transaction
        .validate_signed()
        .map_err(|_| CodecError::InvalidTransaction)?;
    Ok(transaction)
}

/// Decodes and validates a unified envelope in its block context.
///
/// Authorization signatures are state-dependent, so the caller supplies the
/// active policy resolver for the decoded account.
pub fn decode_signed_protocol_transaction_at<F>(
    bytes: &[u8],
    height: BlockHeight,
    block_timestamp: u64,
    _policy_for: F,
) -> Result<SignedProtocolTransaction, CodecError> {
    if bytes.len() > crate::transaction::MAX_PROTOCOL_TRANSACTION_SIZE {
        return Err(CodecError::InvalidTransaction);
    }
    let transaction: SignedProtocolTransaction = canonical_deserialize(bytes)?;
    let valid = match &transaction {
        SignedProtocolTransaction::Transfer(transaction) => {
            transaction.validate_signed_at_height(block_timestamp, height)
        }
        SignedProtocolTransaction::Ecash(transaction) => {
            transaction.validate_signed_for_height(height)
        }
    };
    valid.map_err(|_| CodecError::InvalidTransaction)?;
    Ok(transaction)
}

/// Decodes a structurally valid block.
///
/// This validates block-local rules, including transaction signatures, merkle root, size, and
/// timestamp bounds. It does not validate proof of work, parent linkage, ledger state root, fork
/// choice, or coinbase subsidy against a ledger.
pub fn decode_block(bytes: &[u8]) -> Result<Block, CodecError> {
    if bytes.len() > MAX_BLOCK_SIZE {
        return Err(CodecError::InvalidBlock);
    }
    let block: Block = canonical_deserialize(bytes)?;
    block.validate().map_err(|_| CodecError::InvalidBlock)?;
    Ok(block)
}
