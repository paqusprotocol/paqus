use super::{
    SparseStateTree, calculate_state_root, create_account_state_proof, verify_account_state_proof,
};
use crate::consensus::supply::Amount;
use crate::crypto::Address;
use crate::state::Account;
use std::collections::BTreeMap;

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

#[test]
fn verifies_account_state_proof_against_state_root() {
    let mut accounts = BTreeMap::new();
    accounts.insert(address(1), Account::new(address(1), Amount(100)));
    accounts.insert(address(2), Account::new(address(2), Amount(50)));
    accounts.insert(address(3), Account::new(address(3), Amount(25)));
    let root = calculate_state_root(&accounts);
    let proof = create_account_state_proof(&accounts, &address(2)).unwrap();

    assert!(verify_account_state_proof(root, &proof));
}

#[test]
fn sparse_state_tree_incremental_updates_match_full_root() {
    let mut accounts = BTreeMap::new();
    let mut tree = SparseStateTree::new();

    for byte in [3, 1, 2] {
        let account = Account::new(address(byte), Amount(byte as u64 * 10));
        tree.update_account(&account);
        accounts.insert(account.address, account);

        assert_eq!(tree.root(), calculate_state_root(&accounts));
    }

    let updated = Account::trusted_with_nonce(address(2), Amount(99), crate::block::Nonce(7));
    tree.update_account(&updated);
    accounts.insert(updated.address, updated);

    assert_eq!(tree.root(), calculate_state_root(&accounts));
}

#[test]
fn rejects_tampered_account_state_proof() {
    let mut accounts = BTreeMap::new();
    accounts.insert(address(1), Account::new(address(1), Amount(100)));
    accounts.insert(address(2), Account::new(address(2), Amount(50)));
    let root = calculate_state_root(&accounts);
    let mut proof = create_account_state_proof(&accounts, &address(2)).unwrap();

    proof.account.balance = Amount(51);

    assert!(!verify_account_state_proof(root, &proof));
}

#[test]
fn missing_account_has_no_state_proof() {
    let mut accounts = BTreeMap::new();
    accounts.insert(address(1), Account::new(address(1), Amount(100)));

    assert!(create_account_state_proof(&accounts, &address(9)).is_none());
}
