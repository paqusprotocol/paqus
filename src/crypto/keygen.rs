use crate::crypto::error::CryptoError;
use crate::types::{PublicKey, SecretKey, Signature as PaqusSignatureBytes};
use ml_dsa::{
    ExpandedSigningKey, Generate, Keypair, MlDsa87, Signature as MlDsaSignature, SignatureEncoding,
    Signer, SigningKey, Verifier, VerifyingKey,
};

type PaqusSigningKey = SigningKey<MlDsa87>;
type PaqusExpandedSigningKey = ExpandedSigningKey<MlDsa87>;
type PaqusVerifyingKey = VerifyingKey<MlDsa87>;
type PaqusSignature = MlDsaSignature<MlDsa87>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

pub fn sign(secret_key: &SecretKey, message: &[u8]) -> PaqusSignatureBytes {
    let expanded_key = expanded_signing_key(secret_key);
    let signature: PaqusSignature = expanded_key.sign(message);
    PaqusSignatureBytes(signature.to_bytes().into())
}

pub fn verify(public_key: &PublicKey, message: &[u8], signature: &PaqusSignatureBytes) -> bool {
    verify_result(public_key, message, signature).is_ok()
}

pub fn verify_result(
    public_key: &PublicKey,
    message: &[u8],
    signature: &PaqusSignatureBytes,
) -> Result<(), CryptoError> {
    let verifying_key = verifying_key(public_key);
    let Some(signature) = PaqusSignature::decode(&signature.0.into()) else {
        return Err(CryptoError::InvalidSignatureEncoding);
    };

    verifying_key
        .verify(message, &signature)
        .map_err(|_| CryptoError::VerificationFailed)
}

fn expanded_signing_key(secret_key: &SecretKey) -> PaqusExpandedSigningKey {
    #[allow(deprecated)]
    PaqusExpandedSigningKey::from_expanded(&secret_key.0.into())
}

fn verifying_key(public_key: &PublicKey) -> PaqusVerifyingKey {
    PaqusVerifyingKey::decode(&public_key.0.into())
}
