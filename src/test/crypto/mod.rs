pub use crate::crypto::*;

use crate::crypto::{
    CryptoError, derive_public_key, generate_keypair, sha3_512_proof_of_work_hash, sign, verify,
    verify_result,
};

mod address;

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
    let message = b"core";
    let signature = sign(&keypair.secret_key, message);

    assert_eq!(signature.0.len(), 4627);
    assert!(verify(&keypair.public_key, message, &signature));
    assert!(!verify(&keypair.public_key, b"tampered", &signature));
    assert_eq!(
        verify_result(&keypair.public_key, b"tampered", &signature),
        Err(CryptoError::VerificationFailed)
    );
}

#[test]
fn sha3_512_proof_of_work_matches_known_empty_input_vector() {
    let hash = sha3_512_proof_of_work_hash(b"");
    let expected = hex::decode(
        "a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a6\
         15b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26",
    )
    .unwrap();

    assert_eq!(hash.0.as_slice(), expected.as_slice());
}
