use crate::params::BASE_FEE;
use crate::state::{Account, StateError};
use crate::transaction::Transaction;
use crate::types::{Address, Amount, Nonce};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn transaction(nonce: u64) -> Transaction {
    Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(BASE_FEE),
        Nonce(nonce),
    )
}

#[test]
fn creates_account_with_zero_nonce() {
    let account = Account::new(address(1), Amount(100));

    assert_eq!(account.address, address(1));
    assert_eq!(account.balance, Amount(100));
    assert_eq!(account.nonce, Nonce(0));
}

#[test]
fn credits_and_debits_balance() {
    let mut account = Account::new(address(1), Amount(100));

    assert_eq!(account.credit(Amount(25)), Ok(()));
    assert_eq!(account.balance, Amount(125));

    assert_eq!(account.debit(Amount(50)), Ok(()));
    assert_eq!(account.balance, Amount(75));
}

#[test]
fn rejects_debit_when_balance_is_insufficient() {
    let mut account = Account::new(address(1), Amount(10));

    assert_eq!(
        account.debit(Amount(11)),
        Err(StateError::InsufficientBalance)
    );
}

#[test]
fn applies_outgoing_transaction_amount_fee_and_nonce() {
    let mut account = Account::new(address(1), Amount(100));

    assert_eq!(
        account.apply_outgoing_transaction(&transaction(0), crate::types::Height(0)),
        Ok(())
    );
    assert_eq!(account.balance, Amount(88));
    assert_eq!(account.nonce, Nonce(1));
}

#[test]
fn rejects_outgoing_transaction_with_wrong_nonce() {
    let mut account = Account::new(address(1), Amount(100));

    assert_eq!(
        account.apply_outgoing_transaction(&transaction(1), crate::types::Height(0)),
        Err(StateError::InvalidNonce)
    );
}

#[test]
fn rejects_outgoing_transaction_from_different_address() {
    let mut account = Account::new(address(9), Amount(100));

    assert_eq!(
        account.apply_outgoing_transaction(&transaction(0), crate::types::Height(0)),
        Err(StateError::AddressMismatch)
    );
}

#[test]
fn applies_incoming_transaction_amount_only() {
    let mut account = Account::new(address(2), Amount(5));

    assert_eq!(
        account.apply_incoming_transaction(&transaction(0), crate::types::Height(10)),
        Ok(())
    );
    assert_eq!(account.balance, Amount(15));
    assert_eq!(
        account.available_balance_at(crate::types::Height(0)),
        Amount(5)
    );
    assert_eq!(
        account.unspendable_balance_at(crate::types::Height(0)),
        Amount(10)
    );
    assert_eq!(
        account.available_balance_at(crate::types::Height(10)),
        Amount(15)
    );
    assert_eq!(account.nonce, Nonce(0));
}

#[test]
fn rejects_incoming_transaction_to_different_address() {
    let mut account = Account::new(address(9), Amount(5));

    assert_eq!(
        account.apply_incoming_transaction(&transaction(0), crate::types::Height(10)),
        Err(StateError::AddressMismatch)
    );
}

#[test]
fn rejects_debit_when_credit_is_not_mature() {
    let mut account = Account::new(address(1), Amount(0));
    account
        .credit_locked(
            Amount(100),
            crate::types::Height(10),
            crate::state::CreditSource::MiningReward,
        )
        .unwrap();

    assert_eq!(
        account.debit_at(Amount(1), crate::types::Height(9)),
        Err(StateError::InsufficientBalance)
    );
    assert_eq!(
        account.debit_at(Amount(1), crate::types::Height(10)),
        Ok(())
    );
    assert_eq!(account.balance, Amount(99));
}

#[test]
fn compacts_credits_with_same_unlock_policy() {
    let mut account = Account::new(address(1), Amount(0));
    account
        .credit_locked(
            Amount(10),
            crate::types::Height(5),
            crate::state::CreditSource::Transaction,
        )
        .unwrap();
    account
        .credit_locked(
            Amount(15),
            crate::types::Height(5),
            crate::state::CreditSource::Transaction,
        )
        .unwrap();

    assert_eq!(account.credits.len(), 1);
    assert_eq!(account.credits[0].amount, Amount(25));
}
