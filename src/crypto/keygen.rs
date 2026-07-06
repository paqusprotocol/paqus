use crate::error::CryptoError;
use borsh::{BorshDeserialize, BorshSerialize};
use ml_dsa::{
    ExpandedSigningKey, Generate, Keypair, MlDsa87, Signature as MlDsaSignature, SignatureEncoding,
    Signer, SigningKey, Verifier, VerifyingKey,
};
use static_assertions::const_assert_eq;
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

type PaqusSigningKey = SigningKey<MlDsa87>;
type PaqusExpandedSigningKey = ExpandedSigningKey<MlDsa87>;
type PaqusVerifyingKey = VerifyingKey<MlDsa87>;
type PaqusSignature = MlDsaSignature<MlDsa87>;

pub const PUBLIC_KEY_SIZE: usize = 2_592;
pub const SECRET_KEY_SIZE: usize = 4_896;
pub const SIGNATURE_SIZE: usize = 4_627;
const_assert_eq!(PUBLIC_KEY_SIZE, 2_592);
const_assert_eq!(SECRET_KEY_SIZE, 4_896);
const_assert_eq!(SIGNATURE_SIZE, 4_627);

pub type PublicKeyBytes = [u8; PUBLIC_KEY_SIZE];
pub type SecretKeyBytes = [u8; SECRET_KEY_SIZE];
pub type SignatureBytes = [u8; SIGNATURE_SIZE];

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize,
)]
pub struct PublicKey(pub PublicKeyBytes);

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Zeroize,
    ZeroizeOnDrop,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct SecretKey(pub SecretKeyBytes);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize,
)]
pub struct Signature(pub SignatureBytes);

#[derive(Clone)]
pub struct CachedVerifyingKey {
    inner: PaqusVerifyingKey,
}

impl fmt::Debug for CachedVerifyingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CachedVerifyingKey(..)")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPair {
    pub public_key: PublicKey,
    pub secret_key: SecretKey,
}

pub fn generate_keypair() -> KeyPair {
    let signing_key = PaqusSigningKey::generate();
    let public_key = PublicKey(signing_key.verifying_key().encode().into());

    #[allow(deprecated)]
    let secret_key = SecretKey(signing_key.expanded_key().to_expanded().into());

    KeyPair {
        public_key,
        secret_key,
    }
}

pub fn derive_public_key(secret_key: &SecretKey) -> PublicKey {
    let expanded_key = expanded_signing_key(secret_key);
    PublicKey(expanded_key.verifying_key().encode().into())
}

pub fn sign(secret_key: &SecretKey, message: &[u8]) -> Signature {
    let expanded_key = expanded_signing_key(secret_key);
    let signature: PaqusSignature = expanded_key.sign(message);
    Signature(signature.to_bytes().into())
}

pub fn verify(public_key: &PublicKey, message: &[u8], signature: &Signature) -> bool {
    verify_result(public_key, message, signature).is_ok()
}

pub fn cached_verifying_key(public_key: &PublicKey) -> CachedVerifyingKey {
    CachedVerifyingKey {
        inner: verifying_key(public_key),
    }
}

pub fn verify_result(
    public_key: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> Result<(), CryptoError> {
    cached_verifying_key(public_key).verify(message, signature)
}

impl CachedVerifyingKey {
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        let Some(signature) = PaqusSignature::decode(&signature.0.into()) else {
            return Err(CryptoError::InvalidSignatureEncoding);
        };

        self.inner
            .verify(message, &signature)
            .map_err(|_| CryptoError::VerificationFailed)
    }
}

fn expanded_signing_key(secret_key: &SecretKey) -> PaqusExpandedSigningKey {
    #[allow(deprecated)]
    PaqusExpandedSigningKey::from_expanded(&secret_key.0.into())
}

fn verifying_key(public_key: &PublicKey) -> PaqusVerifyingKey {
    PaqusVerifyingKey::decode(&public_key.0.into())
}
