use crate::types::{BlockHash, Height};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub height: Height,
    pub hash: BlockHash,
}

pub const HARDCODED_CHECKPOINTS: &[Checkpoint] = &[];

pub fn checkpoint_for_height(height: Height) -> Option<Checkpoint> {
    HARDCODED_CHECKPOINTS
        .iter()
        .copied()
        .find(|checkpoint| checkpoint.height == height)
}

pub fn validate_checkpoint(height: Height, hash: BlockHash) -> bool {
    checkpoint_for_height(height).is_none_or(|checkpoint| checkpoint.hash == hash)
}
