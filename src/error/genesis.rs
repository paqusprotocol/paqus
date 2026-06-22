use crate::ledger::LedgerError;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenesisError {
    InvalidPremine,
    Ledger(LedgerError),
}

impl fmt::Display for GenesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenesisError::InvalidPremine => f.write_str("genesis premine amount is invalid"),
            GenesisError::Ledger(error) => write!(f, "genesis ledger error: {error}"),
        }
    }
}

impl Error for GenesisError {}

impl From<LedgerError> for GenesisError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}
