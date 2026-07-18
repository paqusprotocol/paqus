use crate::crypto::HASH_SIZE;
use crate::ledger::LedgerError;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenesisError {
    Ledger(LedgerError),
    HashMismatch {
        expected: [u8; HASH_SIZE],
        found: [u8; HASH_SIZE],
    },
}

impl fmt::Display for GenesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenesisError::Ledger(error) => write!(f, "genesis ledger error: {error}"),
            GenesisError::HashMismatch { expected, found } => write!(
                f,
                "canonical genesis hash mismatch: expected {expected:02x?}, found {found:02x?}"
            ),
        }
    }
}

impl Error for GenesisError {}

impl From<LedgerError> for GenesisError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}
