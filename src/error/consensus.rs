use crate::block::BlockError;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusError {
    InvalidBlock(BlockError),
    InvalidDifficulty,
    UnexpectedDifficulty,
    InvalidProofOfWorkParameters,
    InvalidHeight,
    InvalidPreviousHash,
    InvalidTimestamp,
    InsufficientProofOfWork,
}

impl fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusError::InvalidBlock(error) => write!(f, "invalid block: {error}"),
            ConsensusError::InvalidDifficulty => f.write_str("difficulty is outside allowed range"),
            ConsensusError::UnexpectedDifficulty => {
                f.write_str("block difficulty does not match expected difficulty")
            }
            ConsensusError::InvalidProofOfWorkParameters => {
                f.write_str("proof-of-work parameters are invalid")
            }
            ConsensusError::InvalidHeight => f.write_str("block height does not extend tip"),
            ConsensusError::InvalidPreviousHash => {
                f.write_str("block previous hash does not match tip")
            }
            ConsensusError::InvalidTimestamp => f.write_str("block timestamp is earlier than tip"),
            ConsensusError::InsufficientProofOfWork => {
                f.write_str("block hash does not satisfy proof-of-work difficulty")
            }
        }
    }
}

impl Error for ConsensusError {}

impl From<BlockError> for ConsensusError {
    fn from(error: BlockError) -> Self {
        Self::InvalidBlock(error)
    }
}
