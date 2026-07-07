use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    InvalidAddressEncoding,
    InvalidPublicKey,
    InvalidSignatureEncoding,
    InvalidProofOfWorkParameters,
    ProofOfWorkHashFailed,
    VerificationFailed,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::InvalidAddressEncoding => f.write_str("address string is invalid"),
            CryptoError::InvalidPublicKey => f.write_str("public key bytes are invalid"),
            CryptoError::InvalidSignatureEncoding => {
                f.write_str("signature bytes are not valid ML-DSA-87 encoding")
            }
            CryptoError::InvalidProofOfWorkParameters => {
                f.write_str("proof-of-work hash parameters are invalid")
            }
            CryptoError::ProofOfWorkHashFailed => f.write_str("proof-of-work hash failed"),
            CryptoError::VerificationFailed => f.write_str("signature verification failed"),
        }
    }
}

impl Error for CryptoError {}
