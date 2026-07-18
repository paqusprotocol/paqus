use super::{AccountNonce, ValidityWindow, Witness};
use crate::block::BlockHeight;
use crate::codec::canonical_bytes;
use crate::consensus::supply::Amount;
use crate::crypto::{
    Address, HashDomain, PublicKey, Signature, TransactionHash, address_from_public_key,
    domain_hash, verify,
};
use crate::ecash::{DepositCashMetadata, WithdrawCashMetadata};
use crate::error::TransactionError;
use borsh::{BorshDeserialize, BorshSerialize};

pub const ECASH_TRANSACTION_VERSION: u8 = 1;
/// eCash carries one or more post-quantum coin authorizations in addition to
/// the transaction witness, so it needs a dedicated bounded envelope.
pub const MAX_ECASH_TX_SIZE: usize = 64 * 1024;
const ECASH_SIGNATURE_DOMAIN: &[u8] = b"PAQUSCORE_ECASH_TX_V1";

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EcashTransactionKind {
    WithdrawCash {
        amount: Amount,
        metadata: WithdrawCashMetadata,
    },
    DepositCash {
        recipient: Address,
        metadata: DepositCashMetadata,
    },
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EcashTransaction {
    pub version: u8,
    pub signer: Address,
    pub fee: Amount,
    pub nonce: AccountNonce,
    pub timestamp: u64,
    pub kind: EcashTransactionKind,
    pub validity: ValidityWindow,
}

impl EcashTransaction {
    pub fn withdraw(
        signer: Address,
        amount: Amount,
        fee: Amount,
        nonce: AccountNonce,
        metadata: WithdrawCashMetadata,
    ) -> Self {
        Self {
            version: ECASH_TRANSACTION_VERSION,
            signer,
            fee,
            nonce,
            timestamp: 0,
            kind: EcashTransactionKind::WithdrawCash { amount, metadata },
            validity: ValidityWindow::UNBOUNDED,
        }
    }

    pub fn deposit(
        signer: Address,
        recipient: Address,
        fee: Amount,
        nonce: AccountNonce,
        metadata: DepositCashMetadata,
    ) -> Self {
        Self {
            version: ECASH_TRANSACTION_VERSION,
            signer,
            fee,
            nonce,
            timestamp: 0,
            kind: EcashTransactionKind::DepositCash {
                recipient,
                metadata,
            },
            validity: ValidityWindow::UNBOUNDED,
        }
    }

    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }
    pub fn with_validity_window(mut self, validity: ValidityWindow) -> Self {
        self.validity = validity;
        self
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        if self.version != ECASH_TRANSACTION_VERSION {
            return Err(TransactionError::UnsupportedVersion);
        }
        match &self.kind {
            EcashTransactionKind::WithdrawCash { amount, metadata } => {
                if amount.0 == 0 {
                    return Err(TransactionError::ZeroAmount);
                }
                metadata
                    .validate_amount(*amount)
                    .map_err(|_| TransactionError::InvalidEcashMetadata)?;
            }
            EcashTransactionKind::DepositCash {
                recipient,
                metadata,
            } => {
                metadata
                    .validate_authorizations(*recipient)
                    .map_err(|_| TransactionError::InvalidEcashMetadata)?;
                let amount = metadata
                    .amount()
                    .map_err(|_| TransactionError::InvalidEcashMetadata)?;
                if self.fee.0 >= amount.0 {
                    return Err(TransactionError::EcashFeeExceedsAmount);
                }
                if *recipient == Address([0; 20]) {
                    return Err(TransactionError::InvalidEcashRecipient);
                }
            }
        }
        self.validity.validate()
    }

    pub fn validate_for_height(&self, height: BlockHeight) -> Result<(), TransactionError> {
        self.validate()?;
        self.validity.validate_at(height)
    }

    pub fn amount(&self) -> Result<Amount, TransactionError> {
        match &self.kind {
            EcashTransactionKind::WithdrawCash { amount, .. } => Ok(*amount),
            EcashTransactionKind::DepositCash { metadata, .. } => metadata
                .amount()
                .map_err(|_| TransactionError::InvalidEcashMetadata),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        canonical_bytes(self)
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = self.to_bytes();
        let mut bytes = Vec::with_capacity(ECASH_SIGNATURE_DOMAIN.len() + payload.len());
        bytes.extend_from_slice(ECASH_SIGNATURE_DOMAIN);
        bytes.extend_from_slice(&payload);
        bytes
    }

    pub fn hash(&self) -> TransactionHash {
        TransactionHash(domain_hash(HashDomain::Transaction, &self.to_bytes()).0)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignedEcashTransaction {
    pub transaction: EcashTransaction,
    pub witness: Witness,
}

impl SignedEcashTransaction {
    pub fn new(transaction: EcashTransaction, public_key: PublicKey, signature: Signature) -> Self {
        Self {
            transaction,
            witness: Witness::new(public_key, signature),
        }
    }

    pub fn validate_signed(&self) -> Result<(), TransactionError> {
        self.transaction.validate()?;
        if self.to_bytes().len() > MAX_ECASH_TX_SIZE {
            return Err(TransactionError::TransactionTooLarge);
        }
        if self.witness.public_key.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptyPublicKey);
        }
        if self.witness.signature.0.iter().all(|byte| *byte == 0) {
            return Err(TransactionError::EmptySignature);
        }
        if address_from_public_key(&self.witness.public_key) != self.transaction.signer {
            return Err(TransactionError::SenderAddressMismatch);
        }
        if !verify(
            &self.witness.public_key,
            &self.transaction.signing_bytes(),
            &self.witness.signature,
        ) {
            return Err(TransactionError::InvalidSignature);
        }
        Ok(())
    }

    pub fn validate_signed_for_height(&self, height: BlockHeight) -> Result<(), TransactionError> {
        self.validate_signed()?;
        self.transaction.validity.validate_at(height)
    }

    pub fn hash(&self) -> TransactionHash {
        self.transaction.hash()
    }

    pub fn wtxid(&self) -> crate::crypto::WitnessTransactionHash {
        crate::codec::family_witness_transaction_hash(
            super::TransactionFamily::Ecash,
            &self.to_bytes(),
        )
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        canonical_bytes(self)
    }
}
