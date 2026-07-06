use super::{
    CryptoError, address_from_public_key, address_from_string, address_to_string, generate_keypair,
    try_address_from_public_key, wallet_address_from_public_key,
};
use crate::crypto::{Address, PublicKey};

#[test]
fn derives_address_without_prefix_from_public_key() {
    let public_key = PublicKey([7; 2592]);
    let address = address_from_public_key(&public_key);

    assert_eq!(address.0.len(), 20);
    assert_eq!(address, address_from_public_key(&public_key));
}

#[test]
fn generated_keypair_can_produce_address() {
    let keypair = generate_keypair();
    let address = address_from_public_key(&keypair.public_key);

    assert_ne!(address.0, [0; 20]);
}

#[test]
fn rejects_empty_public_key() {
    let public_key = PublicKey([0; 2592]);

    assert_eq!(
        try_address_from_public_key(&public_key),
        Err(CryptoError::InvalidPublicKey)
    );
}

#[test]
fn formats_wallet_address_as_uppercase_bech32() {
    let address = Address([0xab; 20]);
    let wallet_address = address_to_string(&address);

    assert_eq!(wallet_address.len(), 41);
    assert!(wallet_address.starts_with("PX1"));
    assert_eq!(wallet_address, wallet_address.to_ascii_uppercase());
    assert_eq!(wallet_address, "PX14W46H2AT4W46H2AT4W46H2AT4W46H2ATALMC6F");
}

#[test]
fn parses_wallet_address_string() {
    let address = Address([0xab; 20]);
    let wallet_address = address_to_string(&address);

    assert_eq!(address_from_string(&wallet_address), Ok(address));
}

#[test]
fn rejects_invalid_wallet_address_string() {
    assert_eq!(
        address_from_string("abc"),
        Err(CryptoError::InvalidAddressEncoding)
    );
    assert_eq!(
        address_from_string("px14w46h2at4w46h2at4w46h2at4w46h2atalmc6f"),
        Err(CryptoError::InvalidAddressEncoding)
    );
    assert_eq!(
        address_from_string("PQ14W46H2AT4W46H2AT4W46H2AT4W46H2ATALMC6F"),
        Err(CryptoError::InvalidAddressEncoding)
    );
    assert_eq!(
        address_from_string("PX14W46H2AT4W46H2AT4W46H2AT4W46H2ATALMC6Q"),
        Err(CryptoError::InvalidAddressEncoding)
    );
}

#[test]
fn generated_keypair_can_produce_wallet_address_string() {
    let keypair = generate_keypair();
    let wallet_address = wallet_address_from_public_key(&keypair.public_key);
    let parsed = address_from_string(&wallet_address).expect("wallet address should parse");

    assert_eq!(wallet_address.len(), 41);
    assert!(wallet_address.starts_with("PX1"));
    assert_eq!(parsed, address_from_public_key(&keypair.public_key));
}
