use crate::codec::{
    signed_transaction_bytes, signed_transaction_hash, transaction_bytes, transaction_hash,
};
use crate::crypto::{address_from_public_key, verify};
pub use crate::error::TransactionError;
use crate::params::{MAX_TRANSACTION_AGE, MAX_TRANSACTION_FUTURE_TIME, MAX_TX_SIZE};
use crate::types::{AccountNonce, Address, Amount, PublicKey, Signature, TransactionHash};
use crate::version::{active_versions, supported_transaction_version};
use borsh::{BorshDeserialize, BorshSerialize};

const TRANSACTION_SIGNATURE_DOMAIN: &[u8] = b"PAQUSCORE_TX_V1";

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub version: u8,
    pub from: Address,
    pub to: Address,
    pub amount: Amount,
    pub fee: Amount,
    pub nonce: AccountNonce,
    pub timestamp: u64,
}

impl Transaction {
    pub fn new(
        from: Address,
        to: Address,
        amount: Amount,
        fee: Amount,
        nonce: AccountNonce,
    ) -> Self {
        Self::new_at(from, to, amount, fee, nonce, 0)
    }

    pub fn new_at(
        from: Address,
        to: Address,
        amount: Amount,
        fee: Amount,
        nonce: AccountNonce,
        timestamp: u64,
    ) -> Self {
        Self {
            version: active_versions(crate::types::Height(0)).transaction,
            from,
            to,
            amount,
            fee,
            nonce,
            timestamp,
        }
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        self.validate_for_height(crate::types::Height(0))
    }

    pub fn validate_for_height(
        &self,
        height: crate::types::BlockHeight,
    ) -> Result<(), TransactionError> {
        if !supported_transaction_version(height, self.version) {
            return Err(TransactionError::UnsupportedVersion);
        }

        if self.amount.0 == 0 {
            return Err(TransactionError::ZeroAmount);
        }

        if self.from == self.to {
            return Err(TransactionError::SameSenderAndRecipient);
        }

        Ok(())
    }

    pub fn validate_at(&self, now: u64) -> Result<(), TransactionError> {
        self.validate_at_height(now, crate::types::Height(0))
    }

    pub fn validate_at_height(
        &self,
        now: u64,
        height: crate::types::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.validate_for_height(height)?;

        if self.timestamp > now.saturating_add(MAX_TRANSACTION_FUTURE_TIME as u64) {
            return Err(TransactionError::FromFuture);
        }

        if now.saturating_sub(self.timestamp) > MAX_TRANSACTION_AGE as u64 {
            return Err(TransactionError::Expired);
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
pub struct Witness {
    pub public_key: PublicKey,
    pub signature: Signature,
}

impl Witness {
    pub fn new(public_key: PublicKey, signature: Signature) -> Self {
        Self {
            public_key,
            signature,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub witness: Witness,
}

impl SignedTransaction {
    pub fn new(transaction: Transaction, public_key: PublicKey, signature: Signature) -> Self {
        Self {
            transaction,
            witness: Witness::new(public_key, signature),
        }
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        self.validate_for_height(crate::types::Height(0))
    }

    pub fn validate_for_height(
        &self,
        height: crate::types::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.transaction.validate_for_height(height)?;

        let serialized_size = self.serialized_size();
        if serialized_size > MAX_TX_SIZE {
            return Err(TransactionError::TransactionTooLarge);
        }

        if self.witness.public_key.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptyPublicKey);
        }

        if self.witness.signature.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptySignature);
        }

        Ok(())
    }

    pub fn validate_at(&self, now: u64) -> Result<(), TransactionError> {
        self.validate_at_height(now, crate::types::Height(0))
    }

    pub fn validate_at_height(
        &self,
        now: u64,
        height: crate::types::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.transaction.validate_at_height(now, height)?;

        let serialized_size = self.serialized_size();
        if serialized_size > MAX_TX_SIZE {
            return Err(TransactionError::TransactionTooLarge);
        }

        if self.witness.public_key.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptyPublicKey);
        }

        if self.witness.signature.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptySignature);
        }

        Ok(())
    }

    pub fn verify_signature(&self) -> Result<(), TransactionError> {
        let payload_bytes = self.transaction.signing_bytes();

        if verify(
            &self.witness.public_key,
            &payload_bytes,
            &self.witness.signature,
        ) {
            Ok(())
        } else {
            Err(TransactionError::InvalidSignature)
        }
    }

    pub fn sender_address(&self) -> Address {
        address_from_public_key(&self.witness.public_key)
    }

    pub fn validate_signed(&self) -> Result<(), TransactionError> {
        self.validate_signed_for_height(crate::types::Height(0))
    }

    pub fn validate_signed_for_height(
        &self,
        height: crate::types::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.validate_for_height(height)?;

        if self.sender_address() != self.transaction.from {
            return Err(TransactionError::SenderAddressMismatch);
        }

        self.verify_signature()
    }

    pub fn validate_signed_at(&self, now: u64) -> Result<(), TransactionError> {
        self.validate_signed_at_height(now, crate::types::Height(0))
    }

    pub fn validate_signed_at_height(
        &self,
        now: u64,
        height: crate::types::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.validate_at_height(now, height)?;

        if self.sender_address() != self.transaction.from {
            return Err(TransactionError::SenderAddressMismatch);
        }

        self.verify_signature()
    }

    pub fn hash(&self) -> TransactionHash {
        self.txid()
    }

    pub fn txid(&self) -> TransactionHash {
        self.transaction.hash()
    }

    pub fn wtxid(&self) -> TransactionHash {
        signed_transaction_hash(self)
    }

    pub fn transaction_hash(&self) -> TransactionHash {
        self.txid()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        signed_transaction_bytes(self)
    }

    pub fn serialized_size(&self) -> usize {
        self.to_bytes().len()
    }
}
