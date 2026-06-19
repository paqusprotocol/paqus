use crate::crypto::{address_from_public_key, verify};
use crate::params::{
    AGGRESSIVE_FEE, BASE_FEE, FAST_FEE, HASH_SIZE, MAX_TX_SIZE, MIN_FEE, SLOW_FEE,
    TRANSACTION_VERSION,
};
use crate::transaction::error::TransactionError;
use crate::types::{AccountNonce, Address, Amount, Hash, PublicKey, Signature, TransactionHash};
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Sha3_512};

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
    pub version: u16,
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
            version: TRANSACTION_VERSION,
            from,
            to,
            amount,
            fee,
            nonce,
        }
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        if self.version != TRANSACTION_VERSION {
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
        hash_bytes(&self.to_bytes())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("transaction serialization should not fail")
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
        hash_bytes(&self.to_bytes())
    }

    pub fn payload_hash(&self) -> TransactionHash {
        self.payload.hash()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("signed transaction serialization should not fail")
    }

    pub fn serialized_size(&self) -> usize {
        self.to_bytes().len()
    }
}

fn hash_bytes(bytes: &[u8]) -> Hash {
    let digest = Sha3_512::digest(bytes);
    let mut hash = [0_u8; HASH_SIZE];
    hash.copy_from_slice(&digest);
    Hash(hash)
}
