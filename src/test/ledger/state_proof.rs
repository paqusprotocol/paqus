use super::{calculate_state_root, create_account_state_proof, verify_account_state_proof};
use crate::state::Account;
use crate::types::{Address, Amount};
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
