use crate::block::BlockError;
use crate::state::{OffchainCoinError, StateError};
use crate::transaction::TransactionError;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    AccountNotFound,
    AccountAlreadyExists,
    InvalidBlock(BlockError),
    InvalidState(StateError),
    InvalidTransaction(TransactionError),
    InvalidSignature,
    InsufficientBalance,
    NonceMismatch,
    InvalidStateRoot,
    InvalidCoinbase,
    InvalidParent,
    InvalidBlockHeight,
    InvalidPreviousHash,
    InvalidTimestamp,
    DuplicateBlock,
    SupplyOverflow,
    InvalidOffchainCoin(OffchainCoinError),
    MissingEcashAccountJournal,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::AccountNotFound => f.write_str("account was not found"),
            LedgerError::AccountAlreadyExists => f.write_str("account already exists"),
            LedgerError::InvalidBlock(error) => write!(f, "invalid block: {error}"),
            LedgerError::InvalidState(error) => write!(f, "invalid state transition: {error}"),
            LedgerError::InvalidTransaction(error) => write!(f, "invalid transaction: {error}"),
            LedgerError::InvalidSignature => f.write_str("transaction signature is invalid"),
            LedgerError::InsufficientBalance => f.write_str("account balance is insufficient"),
            LedgerError::NonceMismatch => {
                f.write_str("transaction nonce does not match account nonce")
            }
            LedgerError::InvalidStateRoot => f.write_str("block state root does not match ledger"),
            LedgerError::InvalidCoinbase => f.write_str("block coinbase is invalid"),
            LedgerError::InvalidParent => f.write_str("block parent does not match ledger tip"),
            LedgerError::InvalidBlockHeight => {
                f.write_str("block height does not extend ledger tip")
            }
            LedgerError::InvalidPreviousHash => {
                f.write_str("block previous hash does not match ledger tip")
            }
            LedgerError::InvalidTimestamp => {
                f.write_str("block timestamp is earlier than ledger tip")
            }
            LedgerError::DuplicateBlock => f.write_str("block height already exists in ledger"),
            LedgerError::SupplyOverflow => {
                f.write_str("ledger total supply exceeds maximum supply")
            }
            LedgerError::InvalidOffchainCoin(error) => {
                write!(f, "invalid offchain coin state transition: {error}")
            }
            LedgerError::MissingEcashAccountJournal => {
                f.write_str("eCash account block journal was not found")
            }
        }
    }
}

impl From<OffchainCoinError> for LedgerError {
    fn from(error: OffchainCoinError) -> Self {
        Self::InvalidOffchainCoin(error)
    }
}

impl Error for LedgerError {}

impl From<BlockError> for LedgerError {
    fn from(error: BlockError) -> Self {
        match error {
            BlockError::InvalidStateRoot => Self::InvalidStateRoot,
            BlockError::InvalidCoinbase | BlockError::MissingCoinbase => Self::InvalidCoinbase,
            _ => Self::InvalidBlock(error),
        }
    }
}

impl From<StateError> for LedgerError {
    fn from(error: StateError) -> Self {
        match error {
            StateError::InsufficientBalance => Self::InsufficientBalance,
            StateError::InvalidNonce => Self::NonceMismatch,
            _ => Self::InvalidState(error),
        }
    }
}

impl From<TransactionError> for LedgerError {
    fn from(error: TransactionError) -> Self {
        match error {
            TransactionError::InvalidSignature
            | TransactionError::EmptySignature
            | TransactionError::EmptyPublicKey
            | TransactionError::SenderAddressMismatch => Self::InvalidSignature,
            _ => Self::InvalidTransaction(error),
        }
    }
}
