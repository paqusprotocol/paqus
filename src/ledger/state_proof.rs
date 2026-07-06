use crate::codec::{HashDomain, canonical_bytes, domain_hash};
use crate::crypto::ADDRESS_SIZE;
use crate::crypto::Address;
use crate::crypto::{HASH_SIZE, Hash, StateRoot};
use crate::state::Account;
use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::BTreeMap;

const ADDRESS_BITS: usize = ADDRESS_SIZE * 8;

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofSide {
    Left,
    Right,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateProofNode {
    pub side: ProofSide,
    pub hash: Hash,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct AccountStateProof {
    pub address: Address,
    pub account: Account,
    pub siblings: Vec<StateProofNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparseStateTree {
    nodes: BTreeMap<(usize, [u8; ADDRESS_SIZE]), Hash>,
    root: StateRoot,
}

impl Default for SparseStateTree {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            root: StateRoot::ZERO,
        }
    }
}

impl SparseStateTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_accounts(accounts: &BTreeMap<Address, Account>) -> Self {
        let mut tree = Self::new();
        for account in accounts.values() {
            tree.update_account(account);
        }
        tree
    }

    pub fn root(&self) -> StateRoot {
        self.root
    }

    pub fn update_account(&mut self, account: &Account) {
        self.nodes.insert(
            (ADDRESS_BITS, account.address.0),
            account_leaf_hash(account),
        );

        for depth in (0..ADDRESS_BITS).rev() {
            let parent_prefix = address_prefix(&account.address, depth);
            let left_prefix = child_prefix(parent_prefix, depth, false);
            let right_prefix = child_prefix(parent_prefix, depth, true);
            let left = self
                .nodes
                .get(&(depth + 1, left_prefix))
                .copied()
                .unwrap_or_else(|| empty_subtree_hash(depth + 1));
            let right = self
                .nodes
                .get(&(depth + 1, right_prefix))
                .copied()
                .unwrap_or_else(|| empty_subtree_hash(depth + 1));
            self.nodes
                .insert((depth, parent_prefix), parent_hash(left, right));
        }

        self.root = self
            .nodes
            .get(&(0, [0; ADDRESS_SIZE]))
            .copied()
            .map(|hash| StateRoot(hash.0))
            .unwrap_or(StateRoot::ZERO);
    }
}

pub fn calculate_state_root(accounts: &BTreeMap<Address, Account>) -> StateRoot {
    if accounts.is_empty() {
        return StateRoot::ZERO;
    }

    SparseStateTree::from_accounts(accounts).root()
}

pub fn create_account_state_proof(
    accounts: &BTreeMap<Address, Account>,
    address: &Address,
) -> Option<AccountStateProof> {
    let account = accounts.get(address)?.clone();
    let siblings = sparse_proof(accounts, address);

    Some(AccountStateProof {
        address: *address,
        account,
        siblings,
    })
}

pub fn verify_account_state_proof(root: StateRoot, proof: &AccountStateProof) -> bool {
    if proof.account.address != proof.address {
        return false;
    }

    let mut current = account_leaf_hash(&proof.account);
    for sibling in proof.siblings.iter().rev() {
        current = match sibling.side {
            ProofSide::Left => parent_hash(sibling.hash, current),
            ProofSide::Right => parent_hash(current, sibling.hash),
        };
    }

    current == root
}

fn sparse_root(accounts: &[(&Address, &Account)], depth: usize) -> Hash {
    if accounts.is_empty() {
        return empty_subtree_hash(depth);
    }
    if depth == ADDRESS_BITS {
        return account_leaf_hash(accounts[0].1);
    }

    let split = accounts.partition_point(|(address, _)| !address_bit(address, depth));
    parent_hash(
        sparse_root(&accounts[..split], depth + 1),
        sparse_root(&accounts[split..], depth + 1),
    )
}

fn sparse_proof(accounts: &BTreeMap<Address, Account>, address: &Address) -> Vec<StateProofNode> {
    let nodes: Vec<(&Address, &Account)> = accounts.iter().collect();
    let mut current = nodes.as_slice();
    let mut siblings = Vec::with_capacity(ADDRESS_BITS);

    for depth in 0..ADDRESS_BITS {
        let split = current.partition_point(|(candidate, _)| !address_bit(candidate, depth));
        let bit = address_bit(address, depth);
        let (same, sibling, side) = if bit {
            (&current[split..], &current[..split], ProofSide::Left)
        } else {
            (&current[..split], &current[split..], ProofSide::Right)
        };
        siblings.push(StateProofNode {
            side,
            hash: sparse_root(sibling, depth + 1),
        });
        current = same;
    }

    siblings
}

fn account_leaf_hash(account: &Account) -> Hash {
    domain_hash(HashDomain::AccountState, &canonical_bytes(account))
}

fn empty_subtree_hash(depth: usize) -> Hash {
    let mut bytes = Vec::with_capacity(8);
    bytes.extend_from_slice(&(depth as u64).to_le_bytes());
    domain_hash(HashDomain::StateNode, &bytes)
}

fn address_bit(address: &Address, depth: usize) -> bool {
    let byte = address.0[depth / 8];
    let mask = 0x80_u8 >> (depth % 8);
    byte & mask != 0
}

fn address_prefix(address: &Address, depth: usize) -> [u8; ADDRESS_SIZE] {
    let mut prefix = address.0;
    clear_bits_from(&mut prefix, depth);
    prefix
}

fn child_prefix(
    mut parent_prefix: [u8; ADDRESS_SIZE],
    depth: usize,
    right: bool,
) -> [u8; ADDRESS_SIZE] {
    clear_bits_from(&mut parent_prefix, depth);
    if right {
        let byte = depth / 8;
        let mask = 0x80_u8 >> (depth % 8);
        parent_prefix[byte] |= mask;
    }
    clear_bits_from(&mut parent_prefix, depth + 1);
    parent_prefix
}

fn clear_bits_from(bytes: &mut [u8; ADDRESS_SIZE], depth: usize) {
    if depth >= ADDRESS_BITS {
        return;
    }

    let byte_index = depth / 8;
    let bit_index = depth % 8;
    if bit_index == 0 {
        bytes[byte_index] = 0;
    } else {
        let keep_mask = 0xff_u8 << (8 - bit_index);
        bytes[byte_index] &= keep_mask;
    }
    for byte in &mut bytes[(byte_index + 1)..] {
        *byte = 0;
    }
}

fn parent_hash(left: Hash, right: Hash) -> Hash {
    let mut bytes = Vec::with_capacity(HASH_SIZE * 2);
    bytes.extend_from_slice(&left.0);
    bytes.extend_from_slice(&right.0);
    domain_hash(HashDomain::StateNode, &bytes)
}
