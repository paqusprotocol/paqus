use crate::block::MAX_BLOCK_SIZE;
use crate::codec::{
    decode_block, decode_protocol_event, decode_qcash_transaction,
    decode_signed_protocol_transaction_at, decode_transaction,
};
use crate::event::MAX_PROTOCOL_EVENT_SIZE;
use crate::transaction::MAX_PROTOCOL_TRANSACTION_SIZE;

#[test]
fn consensus_decoders_reject_length_bombs_and_trailing_garbage() {
    let length_bombs: [&[u8]; 4] = [
        &[0xff, 0xff, 0xff, 0xff],
        &[0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff],
        &[5, 0, 0, 0, 0xff, 0xff, 0xff, 0xff],
        &[1, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff],
    ];
    for bytes in length_bombs {
        let _ = decode_transaction(bytes);
        let _ = decode_qcash_transaction(bytes);
        let _ = decode_protocol_event(bytes);
        let _ = decode_block(bytes);
        let _ = decode_signed_protocol_transaction_at(bytes, crate::block::Height(0), 0, ());
    }

    let mut transaction = crate::codec::transaction_bytes(&crate::transaction::Transaction::new(
        crate::crypto::Address([1; 20]),
        crate::crypto::Address([2; 20]),
        crate::consensus::supply::Amount(1),
        crate::consensus::supply::Amount(0),
        crate::block::Nonce(0),
    ));
    transaction.push(0);
    assert!(decode_transaction(&transaction).is_err());
}

#[test]
fn consensus_decoders_reject_oversized_input_before_deserialization() {
    assert!(decode_block(&vec![0; MAX_BLOCK_SIZE + 1]).is_err());
    assert!(decode_protocol_event(&vec![0; MAX_PROTOCOL_EVENT_SIZE + 1]).is_err());
    assert!(
        decode_signed_protocol_transaction_at(
            &vec![0; MAX_PROTOCOL_TRANSACTION_SIZE + 1],
            crate::block::Height(0),
            0,
            (),
        )
        .is_err()
    );
    assert!(
        decode_qcash_transaction(&vec![0; crate::transaction::qcash::MAX_QCASH_TX_SIZE + 1])
            .is_err()
    );
}
