use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    UnsupportedVersion,
    EmptyTransactions,
    TooManyTransactions,
    BlockTooLarge,
    InvalidTransaction,
    InvalidMerkleRoot,
    InvalidStateRoot,
    FutureTimestamp,
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::UnsupportedVersion => f.write_str("block version is unsupported"),
            BlockError::EmptyTransactions => {
                f.write_str("block must contain at least one transaction")
            }
            BlockError::TooManyTransactions => f.write_str("block contains too many transactions"),
            BlockError::BlockTooLarge => f.write_str("block serialized size exceeds limit"),
            BlockError::InvalidTransaction => f.write_str("block contains an invalid transaction"),
            BlockError::InvalidMerkleRoot => {
                f.write_str("block merkle root does not match transactions")
            }
            BlockError::InvalidStateRoot => f.write_str("block state root does not match ledger"),
            BlockError::FutureTimestamp => f.write_str("block timestamp is too far in the future"),
        }
    }
}

impl Error for BlockError {}
