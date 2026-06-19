use crate::block::Block;
use crate::ledger::error::LedgerError;
use crate::params::HASH_SIZE;
use crate::types::{BlockHash, BlockHeight, Hash, Height};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chain {
    pub blocks: BTreeMap<BlockHeight, Block>,
    pub tip_height: Option<BlockHeight>,
    pub tip_hash: Option<BlockHash>,
}

impl Chain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_block(&mut self, block: Block) -> Result<(), LedgerError> {
        self.validate_next_block(&block)?;

        let height = block.height();
        let hash = block.hash();
        self.blocks.insert(height, block);
        self.tip_height = Some(height);
        self.tip_hash = Some(hash);

        Ok(())
    }

    pub fn block(&self, height: &BlockHeight) -> Option<&Block> {
        self.blocks.get(height)
    }

    pub fn has_blocks(&self) -> bool {
        self.tip_height.is_some()
    }

    pub fn tip_height(&self) -> Option<BlockHeight> {
        self.tip_height
    }

    pub fn tip_hash(&self) -> Option<BlockHash> {
        self.tip_hash
    }

    pub fn validate_next_block(&self, block: &Block) -> Result<(), LedgerError> {
        if self.blocks.contains_key(&block.height()) {
            return Err(LedgerError::DuplicateBlock);
        }

        match (self.tip_height, self.tip_hash) {
            (None, None) => {
                if block.height() != Height(0) || block.previous_hash() != Hash([0; HASH_SIZE]) {
                    return Err(LedgerError::InvalidBlockHeight);
                }
            }
            (Some(tip_height), Some(tip_hash)) => {
                if block.height().0 != tip_height.0.saturating_add(1) {
                    return Err(LedgerError::InvalidBlockHeight);
                }

                if block.previous_hash() != tip_hash {
                    return Err(LedgerError::InvalidPreviousHash);
                }

                let tip_block = self
                    .blocks
                    .get(&tip_height)
                    .ok_or(LedgerError::InvalidPreviousHash)?;
                if block.timestamp() < tip_block.timestamp() {
                    return Err(LedgerError::InvalidTimestamp);
                }
            }
            _ => return Err(LedgerError::InvalidPreviousHash),
        }

        Ok(())
    }
}
