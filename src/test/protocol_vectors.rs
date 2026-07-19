use crate::block::{Block, Height, Nonce};
use crate::codec::{
    block_bytes, decode_block, decode_protocol_event, decode_signed_protocol_transaction_at,
    protocol_event_bytes,
};
use crate::consensus::supply::{Amount, XPQ};
use crate::crypto::{
    Address, Hash, StateRoot, address_from_public_key, hash_bytes, public_key_from_seed,
    sign_from_seed,
};
use crate::event::{ProtocolEvent, ProtocolEventKind};
use crate::qcash::{CashDenomination, WithdrawCashMetadata, cash_coin_commitment};
use crate::transaction::{
    QCashTransaction, SignedProtocolTransaction, SignedQCashTransaction, SignedTransaction,
    Transaction, TransactionFamily, ValidityWindow,
};

fn protocol_vector_transactions() -> Vec<SignedProtocolTransaction> {
    let transfer_seed = [1; 32];
    let transfer_key = public_key_from_seed(&transfer_seed);
    let transfer = Transaction::new_at(
        address_from_public_key(&transfer_key),
        Address([0x21; 20]),
        Amount(101),
        Amount(2),
        Nonce(3),
        1_700_000_042,
    )
    .with_validity_window(ValidityWindow::new(Height(40), Height(80)).unwrap());
    let signed_transfer = SignedTransaction::new(
        transfer.clone(),
        transfer_key,
        sign_from_seed(&transfer_seed, &transfer.signing_bytes()),
    );

    let qcash_seed = [2; 32];
    let qcash_key = public_key_from_seed(&qcash_seed);
    let qcash = QCashTransaction::withdraw(
        address_from_public_key(&qcash_key),
        Amount(XPQ),
        Amount(3),
        Nonce(4),
        WithdrawCashMetadata::with_denominations(
            Amount(XPQ),
            &[CashDenomination::One],
            &[cash_coin_commitment(&[0x31; 32])],
        )
        .unwrap(),
    )
    .with_timestamp(1_700_000_042)
    .with_validity_window(ValidityWindow::new(Height(40), Height(80)).unwrap());
    let signed_qcash = SignedQCashTransaction::new(
        qcash.clone(),
        qcash_key,
        sign_from_seed(&qcash_seed, &qcash.signing_bytes()),
    );

    vec![signed_transfer.into(), signed_qcash.into()]
}

#[test]
fn canonical_protocol_envelope_block_and_event_vectors_are_stable() {
    let transactions = protocol_vector_transactions();
    let computed: Vec<_> = transactions
        .iter()
        .map(|transaction| {
            let bytes = transaction.to_bytes();
            assert_eq!(
                decode_signed_protocol_transaction_at(&bytes, Height(42), 1_700_000_042, (),)
                    .unwrap(),
                *transaction
            );
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(
                decode_signed_protocol_transaction_at(&trailing, Height(42), 1_700_000_042, (),)
                    .is_err()
            );
            (
                transaction.family(),
                bytes.len(),
                hex::encode(hash_bytes(&bytes).0),
                hex::encode(transaction.hash().0),
                hex::encode(transaction.wtxid().0),
            )
        })
        .collect();

    let mut block = Block::with_all_transactions(
        Height(42),
        Hash([0x55; 32]),
        Address([0x99; 20]),
        7,
        1_700_000_042,
        Nonce(9),
        vec![match &transactions[0] {
            SignedProtocolTransaction::Transfer(tx) => tx.clone(),
            _ => unreachable!(),
        }],
        vec![match &transactions[1] {
            SignedProtocolTransaction::QCash(tx) => tx.clone(),
            _ => unreachable!(),
        }],
    )
    .unwrap();
    let fees = block.checked_total_fees().unwrap();
    block.coinbase.as_mut().unwrap().fees = fees;
    block.refresh_merkle_root();
    block.set_state_root(StateRoot([0x77; 32]));
    let bytes = block_bytes(&block);
    assert_eq!(decode_block(&bytes).unwrap(), block);
    let mut trailing_block = bytes.clone();
    trailing_block.push(0);
    assert!(decode_block(&trailing_block).is_err());
    let block_vector = (
        bytes.len(),
        hex::encode(hash_bytes(&bytes).0),
        hex::encode(block.hash().0),
        hex::encode(block.header.merkle_root.0),
        hex::encode(block.header.witness_root.0),
    );

    let transfer = match &transactions[0] {
        SignedProtocolTransaction::Transfer(tx) => tx,
        _ => unreachable!(),
    };
    let event = ProtocolEvent::new(
        Height(42),
        block.hash(),
        Some(transfer.hash()),
        0,
        ProtocolEventKind::Transfer {
            from: transfer.transaction.from,
            to: transfer.transaction.to,
            amount: transfer.transaction.amount,
            fee: transfer.transaction.fee,
        },
    );
    let event_vector = (
        hex::encode(protocol_event_bytes(&event)),
        hex::encode(event.id().0),
    );
    assert_eq!(
        decode_protocol_event(&protocol_event_bytes(&event)).unwrap(),
        event
    );
    let mut trailing_event = protocol_event_bytes(&event);
    trailing_event.push(0);
    assert!(decode_protocol_event(&trailing_event).is_err());

    assert_eq!(
        computed,
        vec![
            (
                TransactionFamily::Transfer,
                7313,
                "7fff03c4e006af533e22d6c5aa211214baf560e957890de4f59d9cbd517de04d".into(),
                "11ccb47f1c2d204d83a8ecc954089f773988e8a135b8e8062f12c58f038bb820".into(),
                "14849b0c9ecd9e5583425efc2e62fab458af202afbe2a09079ee702c1ea63214".into(),
            ),
            (
                TransactionFamily::QCash,
                7332,
                "5b8fe8c217ea846185a4257824c7da1a0c82335b746373ef8d1b6714cf13f041".into(),
                "78bf83e96dc4448a42437951ea3d0b2259b5da5f0516808a763dca69f0f5db21".into(),
                "76329a85c265ec9e5c7e84ecd0d5af45a3b15ec4c7ac1a0b0ef9639495254f58".into(),
            ),
        ]
    );
    assert_eq!(
        block_vector,
        (
            14889,
            "ad93514fa47195f302d2a45e83123f9e91fa4b3c50e8d467b81e95e07f9866ca".into(),
            "f97d7ac8e332d51cdbdc2bf73c88aa9573989bc52b6bc431e0b8ab3244c88176".into(),
            "5f62ce32107212464263f1b42827505d56f33f6f5e828d9a4d8167e3206b5d7d".into(),
            "c961bb2f0d4659d2fe0fa4f3ed04dd97b178b99558c103f3abba616c5d02457a".into(),
        )
    );
    assert_eq!(
        event_vector,
        (
            "012a00000000000000f97d7ac8e332d51cdbdc2bf73c88aa9573989bc52b6bc431e0b8ab3244c881760111ccb47f1c2d204d83a8ecc954089f773988e8a135b8e8062f12c58f038bb8200000000000589a5fa09aa6e8f47096c82a566389a6d725f983212121212121212121212121212121212121212165000000000000000200000000000000".into(),
            "9b8ee72c78a3e556315a8410318fdb669720d3371433b84c02bf465244454253".into(),
        )
    );
}
