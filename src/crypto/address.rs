use crate::crypto::error::CryptoError;
use crate::types::{Address, PublicKey};
use sha3::{Digest, Sha3_512};

pub fn wallet_address_from_public_key(public_key: &PublicKey) -> String {
    address_to_string(&address_from_public_key(public_key))
}

pub fn address_from_public_key(public_key: &PublicKey) -> Address {
    try_address_from_public_key(public_key).expect("public key should be valid")
}

pub fn try_address_from_public_key(public_key: &PublicKey) -> Result<Address, CryptoError> {
    if public_key.0.iter().all(|byte| *byte == 0) {
        return Err(CryptoError::InvalidPublicKey);
    }

    let digest = Sha3_512::digest(public_key.0);
    let mut address = [0_u8; 20];
    address.copy_from_slice(&digest[44..64]);
    Ok(Address(address))
}

pub fn address_to_string(address: &Address) -> String {
    hex::encode(address.0)
}

pub fn address_from_string(address: &str) -> Result<Address, CryptoError> {
    if address.len() != 40 {
        return Err(CryptoError::InvalidAddressEncoding);
    }

    let bytes = hex::decode(address).map_err(|_| CryptoError::InvalidAddressEncoding)?;
    let bytes: [u8; 20] = bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidAddressEncoding)?;

    Ok(Address(bytes))
}
