use super::{Ledger, LedgerError};
use crate::block::Block;
use crate::consensus::block_reward;
use crate::crypto::{address_from_public_key, generate_keypair, sign};
use crate::params::BASE_FEE;
use crate::state::Account;
use crate::transaction::{SignedTransaction, Transaction};
use crate::types::{Address, Amount, Hash, Height, Nonce};

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

fn signed_transaction(nonce: u64, to: Address, amount: u32) -> SignedTransaction {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = Transaction::new(from, to, Amount(amount), Amount(BASE_FEE), Nonce(nonce));
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());

    SignedTransaction::new(payload, keypair.public_key, signature)
}

fn signed_transaction_from(
    secret_key: &crate::types::SecretKey,
    public_key: crate::types::PublicKey,
    to: Address,
    amount: u32,
    nonce: u64,
) -> SignedTransaction {
    let from = address_from_public_key(&public_key);
    let payload = Transaction::new(from, to, Amount(amount), Amount(BASE_FEE), Nonce(nonce));
    let signature = sign(secret_key, &payload.signing_bytes());

    SignedTransaction::new(payload, public_key, signature)
}

fn funded_ledger(balance: u32) -> (Ledger, Address) {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.payload.from;
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

    ledger
        .create_account(address(1), Amount(crate::params::MAX_UNIT_SUPPLY))
        .unwrap();

    assert_eq!(
        ledger.total_supply(),
        Ok(Amount(crate::params::MAX_UNIT_SUPPLY))
    );
    assert_eq!(
        ledger.create_account(address(2), Amount(1)),
        Err(LedgerError::SupplyOverflow)
    );
    assert_eq!(ledger.balance(&address(2)), None);
    assert_eq!(ledger.validate_supply(), Ok(()));
}

#[test]
fn caps_minted_subsidy_at_remaining_supply() {
    let keypair = generate_keypair();
    let sender = address_from_public_key(&keypair.public_key);
    let receiver = address(2);
    let miner = address(9);
    let mut ledger = Ledger::new();
    ledger
        .create_account(sender, Amount(crate::params::MAX_UNIT_SUPPLY - 50))
        .unwrap();
    ledger.create_account(receiver, Amount(0)).unwrap();
    ledger.create_account(miner, Amount(0)).unwrap();
    let genesis = Block::new(
        Height(0),
        Hash([0; 64]),
        miner,
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    ledger.apply_block(genesis).unwrap();

    let transaction =
        signed_transaction_from(&keypair.secret_key, keypair.public_key, receiver, 1, 0);
    let mut block = Block::new(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner,
        1_700_000_001,
        Nonce(0),
        vec![transaction],
    );
    block.set_state_root(ledger.state_root_after_block(&block).unwrap());

    assert_eq!(ledger.apply_block(block), Ok(()));
    assert_eq!(
        ledger.total_supply(),
        Ok(Amount(crate::params::MAX_UNIT_SUPPLY))
    );
    assert_eq!(
        ledger.balance(&miner),
        Some(Amount(50 + crate::params::BASE_FEE))
    );
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
        Amount(BASE_FEE),
        Nonce(0),
    );

    assert_eq!(ledger.apply_transaction(&transaction), Ok(()));
    assert_eq!(ledger.balance(&address(1)), Some(Amount(88)));
    assert_eq!(ledger.balance(&address(2)), Some(Amount(15)));
    assert_eq!(ledger.account(&address(1)).unwrap().nonce, Nonce(1));
}

#[test]
fn rejects_transaction_when_account_is_missing() {
    let mut ledger = Ledger::new();
    ledger.create_account(address(1), Amount(100)).unwrap();

    let transaction = Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(BASE_FEE),
        Nonce(0),
    );

    assert_eq!(
        ledger.apply_transaction(&transaction),
        Err(LedgerError::AccountNotFound)
    );
}

#[test]
fn applies_genesis_block_and_tracks_tip() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.payload.from;
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
    assert_eq!(ledger.balance(&sender), Some(Amount(88)));
    assert_eq!(ledger.balance(&address(2)), Some(Amount(15)));
    assert_eq!(
        ledger.balance(&miner()),
        Some(Amount(block_reward(Height(1)).0 + crate::params::BASE_FEE))
    );
    assert_eq!(
        ledger.total_supply(),
        Ok(Amount(105 + block_reward(Height(1)).0))
    );
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
    let sender = signed.payload.from;
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

    assert_eq!(
        ledger.apply_block(next),
        Err(LedgerError::InvalidPreviousHash)
    );
}

#[test]
fn inserts_prebuilt_account() {
    let mut ledger = Ledger::new();
    let account = Account::with_nonce(address(1), Amount(100), Nonce(7));

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

    assert_eq!(
        ledger.apply_block(block),
        Err(LedgerError::InvalidState(
            crate::state::StateError::InvalidNonce
        ))
    );
    assert_eq!(ledger.balance(&sender), Some(Amount(100)));
    assert_eq!(ledger.balance(&receiver), Some(Amount(5)));
    assert_eq!(ledger.account(&sender).unwrap().nonce, Nonce(0));
    assert_eq!(ledger.balance(&miner()), Some(Amount(0)));
    assert_eq!(ledger.tip_height(), Some(Height(0)));
}

#[test]
fn rejects_block_with_wrong_state_root() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.payload.from;
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
        Err(LedgerError::InvalidBlock(
            crate::block::BlockError::InvalidStateRoot
        ))
    );
    assert_eq!(ledger.tip_height(), Some(Height(0)));
}

#[test]
fn calculates_deterministic_state_root_after_block() {
    let signed = signed_transaction(0, address(2), 10);
    let sender = signed.payload.from;
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
