use crate::block::BlockError;
use crate::state::StateError;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    AccountNotFound,
    AccountAlreadyExists,
    InvalidBlock(BlockError),
    InvalidState(StateError),
    InvalidBlockHeight,
    InvalidPreviousHash,
    InvalidTimestamp,
    DuplicateBlock,
    SupplyOverflow,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::AccountNotFound => f.write_str("account was not found"),
            LedgerError::AccountAlreadyExists => f.write_str("account already exists"),
            LedgerError::InvalidBlock(error) => write!(f, "invalid block: {error}"),
            LedgerError::InvalidState(error) => write!(f, "invalid state transition: {error}"),
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
        }
    }
}

impl Error for LedgerError {}

impl From<BlockError> for LedgerError {
    fn from(error: BlockError) -> Self {
        Self::InvalidBlock(error)
    }
}

impl From<StateError> for LedgerError {
    fn from(error: StateError) -> Self {
        Self::InvalidState(error)
    }
}
