use crate::block::{Height, Nonce};
use crate::consensus::supply::Amount;
use crate::crypto::{Address, PublicKey, Signature};
use crate::crypto::{address_from_public_key, generate_keypair, sign};
use crate::transaction::{
    SignedProtocolTransaction, SignedTransaction, Transaction, TransactionError, TransferOutput,
    ValidityWindow,
};

const TEST_FEE: u64 = 2;

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn transaction() -> Transaction {
    Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(7),
    )
}

fn signed_payload(from: Address, to: Address, amount: u64, nonce: u64) -> Transaction {
    Transaction::new(from, to, Amount(amount), Amount(TEST_FEE), Nonce(nonce))
}

#[test]
fn validates_basic_transaction_rules() {
    assert_eq!(transaction().validate(), Ok(()));

    let mut unsupported_version = transaction();
    unsupported_version.version += 1;
    assert_eq!(
        unsupported_version.validate(),
        Err(TransactionError::UnsupportedVersion)
    );

    let mut zero_amount = transaction();
    zero_amount.amount = Amount(0);
    assert_eq!(zero_amount.validate(), Err(TransactionError::ZeroAmount));

    let mut same_addresses = transaction();
    same_addresses.to = same_addresses.from;
    assert_eq!(
        same_addresses.validate(),
        Err(TransactionError::SameSenderAndRecipient)
    );
}

#[test]
fn allows_zero_fee_at_core_validation_layer() {
    let mut transaction = transaction();
    transaction.fee = Amount(0);

    assert_eq!(transaction.validate(), Ok(()));
}

#[test]
fn batch_payment_signs_multiple_unique_outputs_once() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let transaction = Transaction::new(from, address(2), Amount(10), Amount(2), Nonce(0))
        .with_additional_outputs(vec![
            TransferOutput {
                to: address(3),
                amount: Amount(20),
            },
            TransferOutput {
                to: address(4),
                amount: Amount(30),
            },
        ]);
    assert_eq!(transaction.total_amount(), Ok(Amount(60)));
    let signature = sign(&keypair.secret_key, &transaction.signing_bytes());
    let signed = SignedTransaction::new(transaction, keypair.public_key, signature);
    assert_eq!(signed.validate_signed(), Ok(()));
}

#[test]
fn batch_payment_rejects_duplicate_recipient() {
    let transaction = transaction().with_additional_outputs(vec![TransferOutput {
        to: address(2),
        amount: Amount(1),
    }]);
    assert_eq!(
        transaction.validate(),
        Err(TransactionError::DuplicateRecipient)
    );
}

#[test]
fn validity_window_is_inclusive_and_height_bound() {
    assert_eq!(
        ValidityWindow::new(Height(12), Height(11)),
        Err(TransactionError::InvalidValidityWindow)
    );
    let window = ValidityWindow::new(Height(10), Height(12)).unwrap();
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = Transaction::new(from, address(2), Amount(10), Amount(TEST_FEE), Nonce(0))
        .with_validity_window(window);
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());
    let signed = SignedTransaction::new(payload, keypair.public_key, signature);

    assert_eq!(signed.validate_signed(), Ok(()));
    assert_eq!(
        signed.validate_signed_for_height(Height(9)),
        Err(TransactionError::NotYetValid)
    );
    assert_eq!(signed.validate_signed_for_height(Height(10)), Ok(()));
    assert_eq!(signed.validate_signed_for_height(Height(12)), Ok(()));
    assert_eq!(
        signed.validate_signed_for_height(Height(13)),
        Err(TransactionError::ValidityExpired)
    );
}

#[test]
fn treats_transaction_timestamp_as_signed_metadata() {
    let now = 1_700_000_000;
    let valid = Transaction::new_at(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(7),
        now,
    );
    assert_eq!(valid.validate_at(now), Ok(()));

    let old = Transaction::new_at(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(7),
        now - 2 * 24 * 60 * 60,
    );
    assert_eq!(old.validate_at(now), Ok(()));

    let future = Transaction::new_at(
        address(1),
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(7),
        now + 2 * crate::consensus::BLOCK_TIME as u64,
    );
    assert_eq!(future.validate_at(now), Ok(()));
}

#[test]
fn signed_transaction_timestamp_policy_is_outside_core_validation() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let now = 1_700_000_000;
    let payload = Transaction::new_at(
        from,
        address(2),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
        now,
    );
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());
    let signed = SignedTransaction::new(payload, keypair.public_key, signature);

    assert_eq!(signed.validate_signed_at(now), Ok(()));
}

#[test]
fn protocol_envelope_exposes_single_witness_public_key_and_address() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = Transaction::new(from, address(2), Amount(10), Amount(TEST_FEE), Nonce(0));
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());
    let envelope = SignedProtocolTransaction::from(SignedTransaction::new(
        payload,
        keypair.public_key,
        signature,
    ));

    assert_eq!(envelope.witness_public_keys(), vec![&keypair.public_key]);
    assert_eq!(
        envelope.single_witness_public_key(),
        Some(&keypair.public_key)
    );
    assert_eq!(envelope.witness_addresses(), vec![from]);
    assert_eq!(
        envelope.weight(),
        envelope.stripped_size() * crate::block::WITNESS_SCALE_FACTOR + envelope.witness_size()
    );
    assert!(envelope.virtual_size() < envelope.to_bytes().len());
}

#[test]
fn hashes_are_deterministic_and_change_with_payload() {
    let mut changed = transaction();
    changed.nonce = Nonce(8);

    assert_eq!(transaction().hash(), transaction().hash());
    assert_ne!(transaction().hash(), changed.hash());
}

#[test]
fn signed_transaction_requires_signature_material() {
    let signed = SignedTransaction::new(
        signed_payload(address(1), address(2), 10, 7),
        PublicKey([1; 2592]),
        Signature([1; 4627]),
    );

    assert_eq!(signed.validate(), Ok(()));
    assert_eq!(signed.transaction_hash(), signed.transaction.hash());
    assert!(signed.serialized_size() <= crate::transaction::MAX_TX_SIZE);

    let without_key = SignedTransaction::new(
        signed_payload(address(1), address(2), 10, 7),
        PublicKey([0; 2592]),
        Signature([1; 4627]),
    );
    assert_eq!(
        without_key.validate(),
        Err(TransactionError::EmptyPublicKey)
    );

    let without_signature = SignedTransaction::new(
        signed_payload(address(1), address(2), 10, 7),
        PublicKey([1; 2592]),
        Signature([0; 4627]),
    );
    assert_eq!(
        without_signature.validate(),
        Err(TransactionError::EmptySignature)
    );
}

#[test]
fn verifies_signed_transaction_signature_and_sender_address() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = signed_payload(from, address(2), 10, 0);
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());
    let signed = SignedTransaction::new(payload, keypair.public_key, signature);

    assert_eq!(signed.sender_address(), from);
    assert_eq!(signed.verify_signature(), Ok(()));
    assert_eq!(signed.validate_signed(), Ok(()));
}

#[test]
fn rejects_signature_without_transaction_domain() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = signed_payload(from, address(2), 10, 0);
    let signature = sign(&keypair.secret_key, &payload.to_bytes());
    let signed = SignedTransaction::new(payload, keypair.public_key, signature);

    assert_eq!(
        signed.verify_signature(),
        Err(TransactionError::InvalidSignature)
    );
}

#[test]
fn rejects_signed_transaction_with_wrong_sender_address() {
    let keypair = generate_keypair();
    let payload = signed_payload(address(1), address(2), 10, 0);
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());
    let signed = SignedTransaction::new(payload, keypair.public_key, signature);

    assert_eq!(
        signed.validate_signed(),
        Err(TransactionError::SenderAddressMismatch)
    );
}

#[test]
fn rejects_signed_transaction_with_invalid_signature() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = signed_payload(from, address(2), 10, 0);
    let mut signed = SignedTransaction::new(
        payload.clone(),
        keypair.public_key,
        sign(&keypair.secret_key, &payload.signing_bytes()),
    );
    signed.witness.signature.0[0] ^= 0xff;

    assert_eq!(
        signed.validate_signed(),
        Err(TransactionError::InvalidSignature)
    );
}

#[test]
fn validates_signed_ecash_withdraw() {
    use crate::ecash::{CashDenomination, WithdrawCashMetadata, cash_coin_commitment};
    use crate::transaction::{EcashTransaction, SignedEcashTransaction};

    let keypair = generate_keypair();
    let signer = address_from_public_key(&keypair.public_key);
    let commitments: Vec<[u8; 32]> = (0..10)
        .map(|index| cash_coin_commitment(&[index; 32]))
        .collect();
    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(1_000 * crate::consensus::supply::XPQ),
        &[CashDenomination::OneHundred; 10],
        &commitments,
    )
    .unwrap();
    let transaction = EcashTransaction::withdraw(
        signer,
        Amount(1_000 * crate::consensus::supply::XPQ),
        Amount(TEST_FEE),
        Nonce(0),
        metadata,
    );
    let signature = sign(&keypair.secret_key, &transaction.signing_bytes());
    let signed = SignedEcashTransaction::new(transaction, keypair.public_key, signature);

    assert_eq!(signed.validate_signed(), Ok(()));
}

#[test]
fn rejects_ecash_withdraw_when_outputs_do_not_match_amount() {
    use crate::ecash::{CashDenomination, WithdrawCashMetadata, cash_coin_commitment};
    use crate::transaction::EcashTransaction;

    let metadata = WithdrawCashMetadata::with_denominations(
        Amount(100 * crate::consensus::supply::XPQ),
        &[CashDenomination::OneHundred],
        &[cash_coin_commitment(&[1; 32])],
    )
    .unwrap();
    let transaction = EcashTransaction::withdraw(
        address(1),
        Amount(50 * crate::consensus::supply::XPQ),
        Amount(TEST_FEE),
        Nonce(0),
        metadata,
    );

    assert_eq!(
        transaction.validate(),
        Err(TransactionError::InvalidEcashMetadata)
    );
}
