use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    UnsupportedVersion,
    MissingCoinbase,
    UnexpectedCoinbase,
    UnexpectedGenesisAllocation,
    TooManyTransactions,
    BlockTooLarge,
    InvalidTransaction,
    InvalidCoinbase,
    InvalidGenesisAllocation,
    InvalidMerkleRoot,
    InvalidStateRoot,
    FutureTimestamp,
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::UnsupportedVersion => f.write_str("block version is unsupported"),
            BlockError::MissingCoinbase => f.write_str("non-genesis block must contain coinbase"),
            BlockError::UnexpectedCoinbase => {
                f.write_str("genesis block must not contain coinbase")
            }
            BlockError::UnexpectedGenesisAllocation => {
                f.write_str("non-genesis block must not contain genesis allocations")
            }
            BlockError::TooManyTransactions => f.write_str("block contains too many transactions"),
            BlockError::BlockTooLarge => f.write_str("block serialized size exceeds limit"),
            BlockError::InvalidTransaction => f.write_str("block contains an invalid transaction"),
            BlockError::InvalidCoinbase => f.write_str("block coinbase is invalid"),
            BlockError::InvalidGenesisAllocation => {
                f.write_str("block genesis allocation is invalid")
            }
            BlockError::InvalidMerkleRoot => {
                f.write_str("block merkle root does not match transactions")
            }
            BlockError::InvalidStateRoot => f.write_str("block state root does not match ledger"),
            BlockError::FutureTimestamp => f.write_str("block timestamp is too far in the future"),
        }
    }
}

impl Error for BlockError {}
