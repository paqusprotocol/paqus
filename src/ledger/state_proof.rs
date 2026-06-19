use crate::params::HASH_SIZE;
use crate::state::Account;
use crate::types::{Address, Hash, StateRoot};
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Sha3_512};
use std::collections::BTreeMap;

const STATE_LEAF_DOMAIN: &[u8] = b"PAQUS_STATE_LEAF_V1";
const STATE_PARENT_DOMAIN: &[u8] = b"PAQUS_STATE_PARENT_V1";

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

pub fn calculate_state_root(accounts: &BTreeMap<Address, Account>) -> StateRoot {
    let mut leaves: Vec<Hash> = accounts.values().map(account_leaf_hash).collect();
    merkle_root(&mut leaves)
}

pub fn create_account_state_proof(
    accounts: &BTreeMap<Address, Account>,
    address: &Address,
) -> Option<AccountStateProof> {
    let account = accounts.get(address)?.clone();
    let mut index = accounts.keys().position(|candidate| candidate == address)?;
    let mut level: Vec<Hash> = accounts.values().map(account_leaf_hash).collect();
    let mut siblings = Vec::new();

    while level.len() > 1 {
        if level.len() % 2 == 1 {
            let last = *level.last().expect("level is not empty");
            level.push(last);
        }

        let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
        let side = if index % 2 == 0 {
            ProofSide::Right
        } else {
            ProofSide::Left
        };
        siblings.push(StateProofNode {
            side,
            hash: level[sibling_index],
        });

        level = level
            .chunks(2)
            .map(|pair| parent_hash(pair[0], pair[1]))
            .collect();
        index /= 2;
    }

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
    for sibling in &proof.siblings {
        current = match sibling.side {
            ProofSide::Left => parent_hash(sibling.hash, current),
            ProofSide::Right => parent_hash(current, sibling.hash),
        };
    }

    current == root
}

fn merkle_root(leaves: &mut Vec<Hash>) -> StateRoot {
    if leaves.is_empty() {
        return Hash([0; HASH_SIZE]);
    }

    while leaves.len() > 1 {
        if leaves.len() % 2 == 1 {
            let last = *leaves.last().expect("leaves is not empty");
            leaves.push(last);
        }

        *leaves = leaves
            .chunks(2)
            .map(|pair| parent_hash(pair[0], pair[1]))
            .collect();
    }

    leaves[0]
}

fn account_leaf_hash(account: &Account) -> Hash {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(STATE_LEAF_DOMAIN);
    bytes
        .extend_from_slice(&borsh::to_vec(account).expect("account serialization should not fail"));
    hash_bytes(&bytes)
}

fn parent_hash(left: Hash, right: Hash) -> Hash {
    let mut bytes = Vec::with_capacity(STATE_PARENT_DOMAIN.len() + HASH_SIZE * 2);
    bytes.extend_from_slice(STATE_PARENT_DOMAIN);
    bytes.extend_from_slice(&left.0);
    bytes.extend_from_slice(&right.0);
    hash_bytes(&bytes)
}

fn hash_bytes(bytes: &[u8]) -> Hash {
    let digest = Sha3_512::digest(bytes);
    let mut hash = [0_u8; HASH_SIZE];
    hash.copy_from_slice(&digest);
    Hash(hash)
}
