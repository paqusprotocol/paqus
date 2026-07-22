use crate::block::Block;
use crate::block::{BlockHeight, Height};
use crate::crypto::{BlockHash, HASH_SIZE, Hash};
use crate::error::LedgerError;
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
                    return Err(LedgerError::InvalidParent);
                }

                if block.timestamp()
                    <= self
                        .timestamp_at(tip_height)
                        .ok_or(LedgerError::InvalidParent)?
                {
                    return Err(LedgerError::InvalidTimestamp);
                }
            }
            _ => return Err(LedgerError::InvalidParent),
        }

        Ok(())
    }

    pub fn remove_tip(&mut self, expected_hash: BlockHash) -> Result<Block, LedgerError> {
        if self.tip_hash != Some(expected_hash) {
            return Err(LedgerError::InvalidParent);
        }
        let height = self.tip_height.ok_or(LedgerError::InvalidBlockHeight)?;
        let block = self
            .blocks
            .remove(&height)
            .ok_or(LedgerError::InvalidBlockHeight)?;
        let previous_height = height.0.checked_sub(1).map(Height);
        self.tip_height = previous_height;
        self.tip_hash =
            previous_height.and_then(|previous| self.blocks.get(&previous).map(Block::hash));
        Ok(block)
    }

    fn timestamp_at(&self, height: BlockHeight) -> Option<u64> {
        self.blocks.get(&height).map(Block::timestamp)
    }
}
