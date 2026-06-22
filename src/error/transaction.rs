use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionError {
    UnsupportedVersion,
    ZeroAmount,
    InvalidFee,
    SameSenderAndRecipient,
    EmptyPublicKey,
    EmptySignature,
    TransactionTooLarge,
    InvalidSignature,
    SenderAddressMismatch,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::UnsupportedVersion => {
                f.write_str("transaction version is unsupported")
            }
            TransactionError::ZeroAmount => {
                f.write_str("transaction amount must be greater than zero")
            }
            TransactionError::InvalidFee => f.write_str("transaction fee is below minimum fee"),
            TransactionError::SameSenderAndRecipient => {
                f.write_str("sender and recipient address must be different")
            }
            TransactionError::EmptyPublicKey => {
                f.write_str("signed transaction public key is empty")
            }
            TransactionError::EmptySignature => {
                f.write_str("signed transaction signature is empty")
            }
            TransactionError::TransactionTooLarge => {
                f.write_str("signed transaction exceeds maximum serialized size")
            }
            TransactionError::InvalidSignature => f.write_str("transaction signature is invalid"),
            TransactionError::SenderAddressMismatch => {
                f.write_str("transaction sender does not match public key address")
            }
        }
    }
}

impl Error for TransactionError {}
