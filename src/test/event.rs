use crate::block::{Block, CoinbaseTransaction, Height, Nonce};
use crate::codec::{decode_protocol_event, protocol_event_bytes};
use crate::consensus::block_reward;
use crate::consensus::supply::Amount;
use crate::crypto::{Address, HASH_SIZE, Hash, address_from_public_key, generate_keypair, sign};
use crate::event::ProtocolEventKind;
use crate::ledger::Ledger;
use crate::transaction::{SignedTransaction, Transaction};

#[test]
fn successful_block_emits_canonical_events_and_rollback_removes_them() {
    let owner = generate_keypair();
    let sender = address_from_public_key(&owner.public_key);
    let recipient = Address([2; 20]);
    let miner = Address([9; 20]);
    let mut ledger = Ledger::new();
    ledger.create_account(sender, Amount(100)).unwrap();
    ledger.create_account(recipient, Amount(0)).unwrap();
    let genesis = Block::new(
        Height(0),
        Hash([0; HASH_SIZE]),
        miner,
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    ledger.apply_block(genesis).unwrap();

    let transaction = Transaction::new(sender, recipient, Amount(10), Amount(2), Nonce(0));
    let signature = sign(&owner.secret_key, &transaction.signing_bytes());
    let signed = SignedTransaction::new(transaction.clone(), owner.public_key, signature);
    let mut block = Block::with_coinbase(
        Height(1),
        ledger.tip_hash().unwrap(),
        miner,
        crate::consensus::DIFFICULTY_START,
        1_700_000_001,
        Nonce(0),
        Some(CoinbaseTransaction::new(
            miner,
            block_reward(Height(1)),
            Amount(2),
        )),
        vec![signed],
    );
    block.set_state_root(ledger.state_root_after_block(&block).unwrap());
    let block_hash = block.hash();
    ledger.apply_block(block).unwrap();

    let events = ledger.events_for_block(&block_hash);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_index, 0);
    assert_eq!(events[0].transaction_hash, Some(transaction.hash()));
    assert_eq!(
        events[0].kind,
        ProtocolEventKind::Transfer {
            from: sender,
            to: recipient,
            amount: Amount(10),
            fee: Amount(2),
        }
    );
    assert_eq!(events[1].event_index, 1);
    assert_eq!(events[1].transaction_hash, None);
    assert!(matches!(
        events[1].kind,
        ProtocolEventKind::CoinbasePaid { .. }
    ));
    assert_ne!(events[0].id(), events[1].id());
    assert_eq!(
        decode_protocol_event(&protocol_event_bytes(&events[0])).unwrap(),
        events[0]
    );
    assert_eq!(ledger.event(events[0].id()), Some(&events[0]));

    ledger.rollback_block(block_hash).unwrap();
    assert!(ledger.events_for_block(&block_hash).is_empty());
}

#[test]
fn rejected_block_does_not_emit_events() {
    let miner = Address([9; 20]);
    let mut ledger = Ledger::new();
    let genesis = Block::new(
        Height(0),
        Hash([0; HASH_SIZE]),
        miner,
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    ledger.apply_block(genesis).unwrap();
    let invalid = Block::new(
        Height(2),
        ledger.tip_hash().unwrap(),
        miner,
        1_700_000_001,
        Nonce(0),
        vec![],
    );
    let hash = invalid.hash();
    assert!(ledger.apply_block(invalid).is_err());
    assert!(ledger.events_for_block(&hash).is_empty());
}
