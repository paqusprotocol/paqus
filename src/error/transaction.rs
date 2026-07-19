use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionError {
    UnsupportedVersion,
    ZeroAmount,
    Expired,
    FromFuture,
    SameSenderAndRecipient,
    TooManyOutputs,
    DuplicateRecipient,
    AmountOverflow,
    EmptyPublicKey,
    EmptySignature,
    TransactionTooLarge,
    InvalidSignature,
    SenderAddressMismatch,
    InvalidQCashMetadata,
    QCashFeeExceedsAmount,
    InvalidQCashRecipient,
    InvalidValidityWindow,
    NotYetValid,
    ValidityExpired,
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
            TransactionError::Expired => f.write_str("transaction timestamp is too old"),
            TransactionError::FromFuture => {
                f.write_str("transaction timestamp is too far in the future")
            }
            TransactionError::SameSenderAndRecipient => {
                f.write_str("sender and recipient address must be different")
            }
            TransactionError::TooManyOutputs => f.write_str("transaction has too many outputs"),
            TransactionError::DuplicateRecipient => {
                f.write_str("transaction contains a duplicate recipient")
            }
            TransactionError::AmountOverflow => f.write_str("transaction output total overflow"),
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
            TransactionError::InvalidQCashMetadata => {
                f.write_str("transaction contains invalid QCash metadata")
            }
            TransactionError::QCashFeeExceedsAmount => {
                f.write_str("QCash deposit fee must be less than deposited amount")
            }
            TransactionError::InvalidQCashRecipient => {
                f.write_str("QCash deposit recipient is invalid")
            }
            TransactionError::InvalidValidityWindow => {
                f.write_str("transaction validity window is invalid")
            }
            TransactionError::NotYetValid => {
                f.write_str("transaction is not valid at this block height yet")
            }
            TransactionError::ValidityExpired => {
                f.write_str("transaction validity window has expired")
            }
        }
    }
}

impl Error for TransactionError {}
