use super::{CryptoError, derive_public_key, generate_keypair, sign, verify, verify_result};

#[test]
fn generates_keypair_with_expected_sizes() {
    let keypair = generate_keypair();

    assert_eq!(keypair.public_key.0.len(), 2592);
    assert_eq!(keypair.secret_key.0.len(), 4896);
}

#[test]
fn derives_public_key_from_secret_key() {
    let keypair = generate_keypair();

    assert_eq!(derive_public_key(&keypair.secret_key), keypair.public_key);
}

#[test]
fn signs_and_verifies_message() {
    let keypair = generate_keypair();
    let message = b"paqus-core";
    let signature = sign(&keypair.secret_key, message);

    assert_eq!(signature.0.len(), 4627);
    assert!(verify(&keypair.public_key, message, &signature));
    assert!(!verify(&keypair.public_key, b"tampered", &signature));
    assert_eq!(
        verify_result(&keypair.public_key, b"tampered", &signature),
        Err(CryptoError::VerificationFailed)
    );
}
