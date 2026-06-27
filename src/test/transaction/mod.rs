use crate::crypto::{address_from_public_key, generate_keypair, sign};
use crate::params::MIN_FEE;
use crate::transaction::{SignedTransaction, Transaction, TransactionError};
use crate::types::{Address, Amount, Nonce, PublicKey, Signature};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn transaction() -> Transaction {
    Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(MIN_FEE),
        Nonce(7),
    )
}

#[test]
fn validates_basic_transaction_rules() {
    assert_eq!(transaction().validate(), Ok(()));

    let mut unsupported_version = transaction();
    unsupported_version.version = crate::params::TRANSACTION_VERSION + 1;
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
fn rejects_transaction_below_minimum_fee() {
    let mut transaction = transaction();
    transaction.fee = Amount(0);

    assert_eq!(transaction.validate(), Err(TransactionError::InvalidFee));
}

#[test]
fn validates_transaction_timestamp_window() {
    let now = 1_700_000_000;
    let valid = Transaction::new_at(
        address(1),
        address(2),
        Amount(10),
        Amount(MIN_FEE),
        Nonce(7),
        now,
    );
    assert_eq!(valid.validate_at(now), Ok(()));

    let expired = Transaction::new_at(
        address(1),
        address(2),
        Amount(10),
        Amount(MIN_FEE),
        Nonce(7),
        now - crate::params::MAX_TRANSACTION_AGE as u64 - 1,
    );
    assert_eq!(expired.validate_at(now), Err(TransactionError::Expired));

    let future = Transaction::new_at(
        address(1),
        address(2),
        Amount(10),
        Amount(MIN_FEE),
        Nonce(7),
        now + crate::params::MAX_TRANSACTION_FUTURE_TIME as u64 + 1,
    );
    assert_eq!(future.validate_at(now), Err(TransactionError::FromFuture));
}

#[test]
fn validates_signed_transaction_timestamp_window() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let now = 1_700_000_000;
    let payload = Transaction::new_at(from, address(2), Amount(10), Amount(MIN_FEE), Nonce(0), now);
    let signature = sign(&keypair.secret_key, &payload.signing_bytes());
    let signed = SignedTransaction::new(payload, keypair.public_key, signature);

    assert_eq!(signed.validate_signed_at(now), Ok(()));
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
    let signed = SignedTransaction::new(transaction(), PublicKey([1; 2592]), Signature([1; 4627]));

    assert_eq!(signed.validate(), Ok(()));
    assert_eq!(signed.transaction_hash(), signed.transaction.hash());
    assert!(signed.serialized_size() <= crate::params::MAX_TX_SIZE);

    let without_key =
        SignedTransaction::new(transaction(), PublicKey([0; 2592]), Signature([1; 4627]));
    assert_eq!(
        without_key.validate(),
        Err(TransactionError::EmptyPublicKey)
    );

    let without_signature =
        SignedTransaction::new(transaction(), PublicKey([1; 2592]), Signature([0; 4627]));
    assert_eq!(
        without_signature.validate(),
        Err(TransactionError::EmptySignature)
    );
}

#[test]
fn verifies_signed_transaction_signature_and_sender_address() {
    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let payload = Transaction::new(from, address(2), Amount(10), Amount(MIN_FEE), Nonce(0));
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
    let payload = Transaction::new(from, address(2), Amount(10), Amount(MIN_FEE), Nonce(0));
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
    let payload = Transaction::new(
        address(1),
        address(2),
        Amount(10),
        Amount(MIN_FEE),
        Nonce(0),
    );
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
    let payload = Transaction::new(from, address(2), Amount(10), Amount(MIN_FEE), Nonce(0));
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
