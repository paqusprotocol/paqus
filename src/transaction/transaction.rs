use crate::codec::{
    signed_transaction_bytes, signed_transaction_hash, transaction_bytes, transaction_hash,
};
use crate::crypto::{address_from_public_key, verify};
use crate::error::TransactionError;
use crate::params::{AGGRESSIVE_FEE, BASE_FEE, FAST_FEE, MAX_TX_SIZE, MIN_FEE, SLOW_FEE};
use crate::types::{AccountNonce, Address, Amount, PublicKey, Signature, TransactionHash};
use crate::version::active_versions;
use borsh::{BorshDeserialize, BorshSerialize};

pub type TransactionPayload = Transaction;
const TRANSACTION_SIGNATURE_DOMAIN: &[u8] = b"PAQUSCORE_TX_V1";

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeeRate {
    Slow,
    Normal,
    Fast,
    Aggressive,
}

impl FeeRate {
    pub fn amount(self) -> Amount {
        Amount(match self {
            FeeRate::Slow => SLOW_FEE,
            FeeRate::Normal => BASE_FEE,
            FeeRate::Fast => FAST_FEE,
            FeeRate::Aggressive => AGGRESSIVE_FEE,
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub version: u8,
    pub from: Address,
    pub to: Address,
    pub amount: Amount,
    pub fee: Amount,
    pub nonce: AccountNonce,
}

impl Transaction {
    pub fn new(
        from: Address,
        to: Address,
        amount: Amount,
        fee: Amount,
        nonce: AccountNonce,
    ) -> Self {
        Self {
            version: active_versions(crate::types::Height(0)).transaction,
            from,
            to,
            amount,
            fee,
            nonce,
        }
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        if self.version != active_versions(crate::types::Height(0)).transaction {
            return Err(TransactionError::UnsupportedVersion);
        }

        if self.amount.0 == 0 {
            return Err(TransactionError::ZeroAmount);
        }

        if self.fee.0 < MIN_FEE {
            return Err(TransactionError::InvalidFee);
        }

        if self.from == self.to {
            return Err(TransactionError::SameSenderAndRecipient);
        }

        Ok(())
    }

    pub fn hash(&self) -> TransactionHash {
        transaction_hash(self)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        transaction_bytes(self)
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload_bytes = self.to_bytes();
        let mut bytes =
            Vec::with_capacity(TRANSACTION_SIGNATURE_DOMAIN.len() + payload_bytes.len());
        bytes.extend_from_slice(TRANSACTION_SIGNATURE_DOMAIN);
        bytes.extend_from_slice(&payload_bytes);
        bytes
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignedTransaction {
    pub payload: TransactionPayload,
    pub public_key: PublicKey,
    pub signature: Signature,
}

impl SignedTransaction {
    pub fn new(payload: TransactionPayload, public_key: PublicKey, signature: Signature) -> Self {
        Self {
            payload,
            public_key,
            signature,
        }
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        self.payload.validate()?;

        if self.serialized_size() > MAX_TX_SIZE {
            return Err(TransactionError::TransactionTooLarge);
        }

        if self.public_key.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptyPublicKey);
        }

        if self.signature.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptySignature);
        }

        Ok(())
    }

    pub fn verify_signature(&self) -> Result<(), TransactionError> {
        let payload_bytes = self.payload.signing_bytes();

        if verify(&self.public_key, &payload_bytes, &self.signature) {
            Ok(())
        } else {
            Err(TransactionError::InvalidSignature)
        }
    }

    pub fn sender_address(&self) -> Address {
        address_from_public_key(&self.public_key)
    }

    pub fn validate_signed(&self) -> Result<(), TransactionError> {
        self.validate()?;

        if self.sender_address() != self.payload.from {
            return Err(TransactionError::SenderAddressMismatch);
        }

        self.verify_signature()
    }

    pub fn hash(&self) -> TransactionHash {
        signed_transaction_hash(self)
    }

    pub fn payload_hash(&self) -> TransactionHash {
        self.payload.hash()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        signed_transaction_bytes(self)
    }

    pub fn serialized_size(&self) -> usize {
        self.to_bytes().len()
    }
}
