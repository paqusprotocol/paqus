use crate::block::Block;
use crate::params::HASH_SIZE;
use crate::types::{BlockHash, BlockHeight, Hash, Height};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockNode {
    pub block: Block,
    pub hash: BlockHash,
    pub parent: BlockHash,
    pub height: BlockHeight,
    pub work: u128,
    pub cumulative_work: u128,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForkChoice {
    nodes: BTreeMap<BlockHash, BlockNode>,
    best_tip: Option<BlockHash>,
}

impl ForkChoice {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_block(&mut self, block: Block) -> Result<BlockHash, ForkChoiceError> {
        let hash = block.hash();
        if self.nodes.contains_key(&hash) {
            return Err(ForkChoiceError::DuplicateBlock);
        }

        let parent = BlockHash(block.previous_hash().0);
        let parent_work = if block.height() == Height(0) {
            if parent != Hash([0; HASH_SIZE]) {
                return Err(ForkChoiceError::MissingParent);
            }
            0
        } else {
            let parent_node = self
                .nodes
                .get(&parent)
                .ok_or(ForkChoiceError::MissingParent)?;
            if block.height().0 != parent_node.height.0.saturating_add(1) {
                return Err(ForkChoiceError::InvalidHeight);
            }
            parent_node.cumulative_work
        };

        let work = block_work(block.difficulty());
        let cumulative_work = parent_work.saturating_add(work);
        let node = BlockNode {
            height: block.height(),
            parent,
            hash,
            work,
            cumulative_work,
            block,
        };

        self.nodes.insert(hash, node);
        self.update_best_tip(hash);
        Ok(hash)
    }

    pub fn best_tip(&self) -> Option<&BlockNode> {
        self.best_tip.and_then(|hash| self.nodes.get(&hash))
    }

    pub fn get(&self, hash: &BlockHash) -> Option<&BlockNode> {
        self.nodes.get(hash)
    }

    pub fn ancestor_hashes(&self, hash: BlockHash) -> Vec<BlockHash> {
        let mut hashes = Vec::new();
        let mut current = hash;

        while let Some(node) = self.nodes.get(&current) {
            hashes.push(current);
            if node.height.0 == 0 {
                break;
            }
            current = node.parent;
        }

        hashes
    }

    pub fn branch_from_ancestor(&self, ancestor: BlockHash, tip: BlockHash) -> Option<Vec<Block>> {
        let mut blocks = Vec::new();
        let mut current = tip;

        while current != ancestor {
            let node = self.nodes.get(&current)?;
            blocks.push(node.block.clone());
            current = node.parent;
        }

        blocks.reverse();
        Some(blocks)
    }

    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.nodes.contains_key(hash)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn update_best_tip(&mut self, candidate_hash: BlockHash) {
        let Some(candidate) = self.nodes.get(&candidate_hash) else {
            return;
        };

        let should_update = match self.best_tip.and_then(|hash| self.nodes.get(&hash)) {
            None => true,
            Some(best) => {
                candidate.cumulative_work > best.cumulative_work
                    || (candidate.cumulative_work == best.cumulative_work
                        && candidate.hash < best.hash)
            }
        };

        if should_update {
            self.best_tip = Some(candidate_hash);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkChoiceError {
    DuplicateBlock,
    InvalidHeight,
    MissingParent,
}

pub fn block_work(difficulty: u32) -> u128 {
    1_u128.checked_shl(difficulty.min(127)).unwrap_or(u128::MAX)
}
