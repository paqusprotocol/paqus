use super::{Ledger, LedgerError, validate_transaction_against_state};
use crate::block::{Block, CoinbaseTransaction};
use crate::block::{Height, Nonce};
use crate::consensus::block_reward;
use crate::consensus::supply::Amount;
use crate::crypto::Address;
use crate::crypto::Hash;
use crate::crypto::{address_from_public_key, generate_keypair, sign};
use crate::state::{Account, CreditSource};
use crate::transaction::{SignedTransaction, Transaction};

const TEST_FEE: u64 = 2;

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn miner() -> Address {
    Address([9; 20])
}

fn empty_genesis() -> Block {
    Block::new(
        Height(0),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![],
    )
}

fn signed_transaction(nonce: u64, to: Address, amount: u64) -> SignedTransaction {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = Transaction::new(from, to, Amount(amount), Amount(TEST_FEE), Nonce(nonce));
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());

    SignedTransaction::new(payload, keypair.public_key, signature)
}

fn signed_transaction_from(
    secret_key: &crate::crypto::SecretKey,
    public_key: crate::crypto::PublicKey,
    to: Address,
    amount: u64,
    nonce: u64,
) -> SignedTransaction {
    let from = address_from_public_key(&public_key);
    let payload = Transaction::new(from, to, Amount(amount), Amount(TEST_FEE), Nonce(nonce));
    let signature = sign(secret_key, &payload.signing_bytes());

    SignedTransaction::new(payload, public_key, signature)
}

fn funded_ledger(balance: u64) -> (Ledger, Address) {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.transaction.from;
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(balance)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();
    (ledger, sender)
}

#[test]
fn creates_accounts_and_reads_balances() {
    let mut ledger = Ledger::new();

    assert_eq!(ledger.create_account(address(1), Amount(100)), Ok(()));
    assert_eq!(ledger.balance(&address(1)), Some(Amount(100)));
    assert_eq!(
        ledger.create_account(address(1), Amount(100)),
        Err(LedgerError::AccountAlreadyExists)
    );
}

#[test]
fn tracks_total_supply_and_rejects_supply_overflow() {
    let mut ledger = Ledger::new();

    ledger.create_account(address(1), Amount(u64::MAX)).unwrap();

    assert_eq!(ledger.total_supply(), Ok(Amount(u64::MAX)));
    assert_eq!(
        ledger.create_account(address(2), Amount(1)),
        Err(LedgerError::SupplyOverflow)
    );
    assert_eq!(ledger.balance(&address(2)), None);
    assert_eq!(ledger.validate_supply(), Ok(()));
}

#[test]
fn rejects_inexact_coinbase_subsidy() {
    let keypair = generate_keypair();
    let sender = address_from_public_key(&keypair.public_key);
    let receiver = address(2);
    let miner = address(9);
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(receiver, Amount(0)).unwrap();
    ledger.create_account(miner, Amount(0)).unwrap();
    ledger.apply_block(empty_genesis()).unwrap();

    let transaction =
        signed_transaction_from(&keypair.secret_key, keypair.public_key, receiver, 1, 0);
    let block = Block::with_coinbase(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner,
        crate::consensus::DIFFICULTY_START,
        1_700_000_001,
        Nonce(0),
        Some(CoinbaseTransaction::new(
            miner,
            Amount(block_reward(Height(1)).0 - 1),
            Amount(TEST_FEE),
        )),
        vec![transaction],
    );
    assert_eq!(ledger.apply_block(block), Err(LedgerError::InvalidCoinbase));
}

#[test]
fn applies_transaction_to_sender_and_receiver_accounts() {
    let mut ledger = Ledger::new();
    ledger.create_account(address(1), Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();

    let transaction = Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );

    assert_eq!(ledger.apply_transaction(&transaction), Ok(()));
    assert_eq!(ledger.balance(&address(1)), Some(Amount(90 - TEST_FEE)));
    assert_eq!(ledger.balance(&address(2)), Some(Amount(15)));
    assert_eq!(ledger.account(&address(1)).unwrap().nonce, Nonce(1));
}

#[test]
fn signed_transaction_apply_does_not_bypass_locked_credits() {
    let keypair = generate_keypair();
    let sender = address_from_public_key(&keypair.public_key);
    let receiver = address(2);
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(0)).unwrap();
    ledger
        .account_mut(&sender)
        .unwrap()
        .credit_locked(Amount(100), Height(10), CreditSource::MiningReward)
        .unwrap();

    let transaction =
        signed_transaction_from(&keypair.secret_key, keypair.public_key, receiver, 10, 0);

    assert_eq!(
        ledger.apply_signed_transaction(&transaction),
        Err(LedgerError::InsufficientBalance)
    );
    assert_eq!(ledger.balance(&sender), Some(Amount(100)));
    assert_eq!(ledger.balance(&receiver), None);
}

#[test]
fn transaction_outputs_mature_after_confirmation_depth() {
    let mut ledger = Ledger::new();
    ledger.create_account(address(1), Amount(100)).unwrap();

    let transaction = Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );

    assert_eq!(ledger.apply_transaction_at(&transaction, Height(7)), Ok(()));
    let receiver = ledger.account(&address(2)).unwrap();
    let immature_height = Height(7 + crate::ledger::CONFIRMATION_DEPTH.saturating_sub(1) as u64);
    assert_eq!(receiver.available_balance_at(immature_height), Amount(0));
    assert_eq!(
        receiver.available_balance_at(Height(7 + crate::ledger::CONFIRMATION_DEPTH as u64)),
        Amount(10)
    );
}

#[test]
fn validates_transaction_against_account_state_without_ledger_runtime() {
    let mut ledger = Ledger::new();
    ledger.create_account(address(1), Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();
    let accounts = ledger.accounts.clone();

    let transaction = Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );

    assert_eq!(
        validate_transaction_against_state(&accounts, &transaction, Height(1)),
        Ok(())
    );
    assert_eq!(accounts.get(&address(1)).unwrap().balance, Amount(100));
    assert_eq!(accounts.get(&address(2)).unwrap().balance, Amount(5));
}

#[test]
fn creates_receiver_account_when_missing() {
    let mut ledger = Ledger::new();
    ledger.create_account(address(1), Amount(100)).unwrap();

    let transaction = Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );

    assert_eq!(ledger.apply_transaction(&transaction), Ok(()));
    assert_eq!(ledger.balance(&address(1)), Some(Amount(90 - TEST_FEE)));
    assert_eq!(ledger.balance(&address(2)), Some(Amount(10)));
}

#[test]
fn applies_genesis_block_and_tracks_tip() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.transaction.from;
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();
    ledger.create_account(miner(), Amount(0)).unwrap();
    let genesis = Block::new(
        Height(0),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    ledger.apply_block(genesis).unwrap();

    let mut block = Block::new(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner(),
        1_700_000_001,
        Nonce(0),
        vec![signed],
    );
    block.set_state_root(ledger.state_root_after_block(&block).unwrap());
    let block_hash = block.hash();

    assert_eq!(ledger.apply_block(block), Ok(()));
    assert_eq!(ledger.tip_height(), Some(Height(1)));
    assert_eq!(ledger.tip_hash(), Some(block_hash));
    assert_eq!(ledger.balance(&sender), Some(Amount(90 - TEST_FEE)));
    assert_eq!(ledger.balance(&address(2)), Some(Amount(15)));
    assert_eq!(
        ledger.balance(&miner()),
        Some(Amount(block_reward(Height(1)).0 + TEST_FEE))
    );
    assert_eq!(
        ledger.total_supply(),
        Ok(Amount(105 + block_reward(Height(1)).0))
    );
}

#[test]
fn validates_and_executes_block_without_mutating_original_ledger() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.transaction.from;
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();
    ledger.create_account(miner(), Amount(0)).unwrap();
    ledger.apply_block(empty_genesis()).unwrap();

    let mut block = Block::new(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner(),
        1_700_000_001,
        Nonce(0),
        vec![signed.clone()],
    );
    let expected_state_root = ledger.state_root_after_block(&block).unwrap();
    block.set_state_root(expected_state_root);

    assert_eq!(ledger.validate_block(&block), Ok(expected_state_root));

    let original_tip = ledger.tip_hash();
    let original_sender_balance = ledger.balance(&sender);
    let (executed, result) = ledger.execute_block(&block).unwrap();

    assert_eq!(ledger.tip_hash(), original_tip);
    assert_eq!(ledger.balance(&sender), original_sender_balance);
    assert_eq!(executed.tip_height(), Some(Height(1)));
    assert_eq!(executed.balance(&sender), Some(Amount(90 - TEST_FEE)));
    assert_eq!(result.height, Height(1));
    assert_eq!(result.state_root_before, ledger.state_root());
    assert_eq!(result.state_root_after, expected_state_root);
    assert_eq!(result.transactions.len(), 1);
    assert_eq!(result.transactions[0].transaction_hash, signed.hash());
    assert_eq!(result.transactions[0].fee, Amount(TEST_FEE));
}

#[test]
fn rejects_non_genesis_first_block() {
    let (mut ledger, _sender) = funded_ledger(100);

    let block = Block::new(
        Height(1),
        Hash([0; 64]),
        miner(),
        1_700_000_000,
        Nonce(0),
        vec![signed_transaction(0, address(2), 10)],
    );

    assert_eq!(
        ledger.apply_block(block),
        Err(LedgerError::InvalidBlockHeight)
    );
}

#[test]
fn rejects_block_with_wrong_previous_hash() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.transaction.from;
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();

    ledger.apply_block(empty_genesis()).unwrap();

    let next = Block::new(
        Height(1),
        Hash([9; 64]),
        miner(),
        1_700_000_010,
        Nonce(0),
        vec![signed_transaction(1, address(2), 10)],
    );

    assert_eq!(ledger.apply_block(next), Err(LedgerError::InvalidParent));
}

#[test]
fn inserts_prebuilt_account() {
    let mut ledger = Ledger::new();
    let account = Account::trusted_with_nonce(address(1), Amount(100), Nonce(7));

    assert_eq!(ledger.insert_account(account), Ok(()));
    assert_eq!(ledger.account(&address(1)).unwrap().nonce, Nonce(7));
}

#[test]
fn apply_block_is_atomic_when_later_transaction_fails() {
    let keypair = generate_keypair();
    let sender = address_from_public_key(&keypair.public_key);
    let receiver = address(2);
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(receiver, Amount(5)).unwrap();
    ledger.create_account(miner(), Amount(0)).unwrap();
    ledger.apply_block(empty_genesis()).unwrap();

    let valid = signed_transaction_from(&keypair.secret_key, keypair.public_key, receiver, 10, 0);
    let invalid_nonce =
        signed_transaction_from(&keypair.secret_key, keypair.public_key, receiver, 10, 7);
    let block = Block::new(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner(),
        1_700_000_001,
        Nonce(0),
        vec![valid, invalid_nonce],
    );

    assert_eq!(ledger.apply_block(block), Err(LedgerError::NonceMismatch));
    assert_eq!(ledger.balance(&sender), Some(Amount(100)));
    assert_eq!(ledger.balance(&receiver), Some(Amount(5)));
    assert_eq!(ledger.account(&sender).unwrap().nonce, Nonce(0));
    assert_eq!(ledger.balance(&miner()), Some(Amount(0)));
    assert_eq!(ledger.tip_height(), Some(Height(0)));
}

#[test]
fn rejects_block_with_wrong_state_root() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.transaction.from;
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();
    ledger.create_account(miner(), Amount(0)).unwrap();
    ledger.apply_block(empty_genesis()).unwrap();

    let mut block = Block::new(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner(),
        1_700_000_001,
        Nonce(0),
        vec![signed],
    );
    block.set_state_root(Hash([7; 64]));

    assert_eq!(
        ledger.apply_block(block),
        Err(LedgerError::InvalidStateRoot)
    );
    assert_eq!(ledger.tip_height(), Some(Height(0)));
}

#[test]
fn rejects_non_genesis_block_with_zero_state_root() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.transaction.from;
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();
    ledger.create_account(miner(), Amount(0)).unwrap();
    ledger.apply_block(empty_genesis()).unwrap();

    let block = Block::new(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner(),
        1_700_000_001,
        Nonce(0),
        vec![signed],
    );

    assert_eq!(
        ledger.apply_block(block),
        Err(LedgerError::InvalidStateRoot)
    );
    assert_eq!(ledger.tip_height(), Some(Height(0)));
}

#[test]
fn calculates_deterministic_state_root_after_block() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.transaction.from;
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(address(2), Amount(5)).unwrap();
    ledger.create_account(miner(), Amount(0)).unwrap();
    ledger.apply_block(empty_genesis()).unwrap();

    let block = Block::new(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner(),
        1_700_000_001,
        Nonce(0),
        vec![signed],
    );
    let expected = ledger.state_root_after_block(&block).unwrap();
    let mut block_with_root = block.clone();
    block_with_root.set_state_root(expected);

    assert_eq!(ledger.apply_block(block_with_root), Ok(()));
    assert_eq!(ledger.state_root(), expected);
}
