use crate::crypto::PublicKey;
use crate::error::CryptoError;
use bech32::primitives::decode::CheckedHrpstring;
use bech32::{Bech32, Hrp};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use static_assertions::const_assert_eq;

pub const ADDRESS_SIZE: usize = 20;
pub type AddressBytes = [u8; ADDRESS_SIZE];
const_assert_eq!(ADDRESS_SIZE, 20);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Address(pub AddressBytes);

impl Address {
    pub const ZERO: Self = Self([0; ADDRESS_SIZE]);
}

const ADDRESS_HRP: &str = "PX";
const BECH32_CHECKSUM_LEN: usize = 6;
const BECH32_ADDRESS_LEN: usize =
    ADDRESS_HRP.len() + 1 + (ADDRESS_SIZE * 8 / 5) + BECH32_CHECKSUM_LEN;
const_assert_eq!(BECH32_CHECKSUM_LEN, 6);
const_assert_eq!(BECH32_ADDRESS_LEN, 41);

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

    let digest = Sha3_256::digest(public_key.0);
    let mut address = [0_u8; 20];
    address.copy_from_slice(&digest[12..32]);
    Ok(Address(address))
}

pub fn address_to_string(address: &Address) -> String {
    bech32::encode_upper::<Bech32>(address_hrp(), &address.0)
        .expect("address bech32 encoding should not fail")
}

pub fn address_from_string(address: &str) -> Result<Address, CryptoError> {
    if address.len() != BECH32_ADDRESS_LEN || address != address.to_ascii_uppercase() {
        return Err(CryptoError::InvalidAddressEncoding);
    }

    let decoded = CheckedHrpstring::new::<Bech32>(address)
        .map_err(|_| CryptoError::InvalidAddressEncoding)?;
    if decoded.hrp() != address_hrp() {
        return Err(CryptoError::InvalidAddressEncoding);
    }

    let bytes: Vec<u8> = decoded.byte_iter().collect();
    let bytes: [u8; ADDRESS_SIZE] = bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidAddressEncoding)?;

    Ok(Address(bytes))
}

fn address_hrp() -> Hrp {
    Hrp::parse(ADDRESS_HRP).expect("address HRP should be valid")
}
