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
use crate::ecash::{CashDenomination, WithdrawCashMetadata, cash_coin_commitment};
use crate::event::{ProtocolEvent, ProtocolEventKind};
use crate::transaction::{
    EcashTransaction, SignedEcashTransaction, SignedProtocolTransaction, SignedTransaction,
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

    let ecash_seed = [2; 32];
    let ecash_key = public_key_from_seed(&ecash_seed);
    let ecash = EcashTransaction::withdraw(
        address_from_public_key(&ecash_key),
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
    let signed_ecash = SignedEcashTransaction::new(
        ecash.clone(),
        ecash_key,
        sign_from_seed(&ecash_seed, &ecash.signing_bytes()),
    );

    vec![signed_transfer.into(), signed_ecash.into()]
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
            SignedProtocolTransaction::Ecash(tx) => tx.clone(),
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
                7309,
                "68054e012f0830bddf44502219f7617be3a1382501277d8db0a4c6fc1110d257".into(),
                "d80da8dcced3cbc6eeddce84d9e6420390e20a260fb8f4573e7890cb32b6f964".into(),
                "ad96ece725f117a9eb5e99523c4f011b9746b134c7f365db72ead8e100ca7456".into(),
            ),
            (
                TransactionFamily::Ecash,
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
            14873,
            "c681ad3b2e7aef1a1972a66ff9a1f98bd795d9024f886603c3c5d908cc6f89cf".into(),
            "00645a305af69f71da9b6d7990ec909a661a7b3b47a86edd3cb70b2fa06f801b".into(),
            "c05c0046170398f3cac3ce6e5f0834e204757b13bc2f4fcb1afe214c67c778db".into(),
            "b7af85f1dac8c0effa474fb63200185d0dcd45c99951227f3b609a0bd1fc44c5".into(),
        )
    );
    assert_eq!(
        event_vector,
        (
            "012a0000000000000000645a305af69f71da9b6d7990ec909a661a7b3b47a86edd3cb70b2fa06f801b01d80da8dcced3cbc6eeddce84d9e6420390e20a260fb8f4573e7890cb32b6f9640000000000589a5fa09aa6e8f47096c82a566389a6d725f983212121212121212121212121212121212121212165000000000000000200000000000000".into(),
            "a9c0c299f92bdb02e7ed3fe64ec03511d2d062380f11cb89a38d1e99c6569438".into(),
        )
    );
}
