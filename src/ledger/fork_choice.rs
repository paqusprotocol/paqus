use crate::block::Block;
use crate::block::{BlockHeight, Height};
use crate::consensus::MIN_DIFFICULTY;
use crate::crypto::{BlockHash, HASH_SIZE, Hash};
use std::collections::BTreeMap;
use std::ops::Add;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Work([u64; 4]);

impl Work {
    pub const ZERO: Self = Self([0, 0, 0, 0]);
    pub const MAX: Self = Self([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);

    pub fn pow2(exponent: u32) -> Self {
        if exponent >= 256 {
            return Self::MAX;
        }

        let limb_from_low = (exponent / 64) as usize;
        let bit = exponent % 64;
        let mut limbs = [0; 4];
        limbs[3 - limb_from_low] = 1_u64 << bit;
        Self(limbs)
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        let mut result = [0; 4];
        let mut carry = 0_u128;

        for index in (0..4).rev() {
            let sum = self.0[index] as u128 + rhs.0[index] as u128 + carry;
            result[index] = sum as u64;
            carry = sum >> 64;
        }

        if carry > 0 { Self::MAX } else { Self(result) }
    }
}

impl Add for Work {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.saturating_add(rhs)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockNode {
    pub block: Block,
    pub hash: BlockHash,
    pub parent: BlockHash,
    pub height: BlockHeight,
    pub work: Work,
    pub cumulative_work: Work,
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

        if block.difficulty() < MIN_DIFFICULTY {
            return Err(ForkChoiceError::InvalidDifficulty);
        }

        let parent = BlockHash(block.previous_hash().0);
        let parent_work = if block.height() == Height(0) {
            if parent != Hash([0; HASH_SIZE]) {
                return Err(ForkChoiceError::MissingParent);
            }
            Work::ZERO
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
    InvalidDifficulty,
    InvalidHeight,
    MissingParent,
}

pub fn block_work(difficulty: u32) -> Work {
    Work::pow2(difficulty)
}
