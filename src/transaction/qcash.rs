use super::{AccountNonce, ValidityWindow, Witness, chain_bound_signing_bytes};
use crate::block::BlockHeight;
use crate::codec::canonical_bytes;
use crate::consensus::supply::Amount;
use crate::crypto::{
    Address, HashDomain, PublicKey, Signature, TransactionHash, address_from_public_key,
    domain_hash, verify,
};
use crate::error::TransactionError;
use crate::qcash::{CashCoinFile, DepositCashMetadata, QCashError, WithdrawCashMetadata};
use borsh::{BorshDeserialize, BorshSerialize};

pub const QCASH_TRANSACTION_VERSION: u8 = 1;
/// QCash carries one or more post-quantum coin authorizations in addition to
/// the transaction witness, so it needs a dedicated bounded envelope.
pub const MAX_QCASH_TX_SIZE: usize = 64 * 1024;
// Keep the version-1 signing domain stable across the public QCash rename.
const QCASH_SIGNATURE_DOMAIN: &[u8] = b"PAQUSCORE_ECASH_TX_V1";

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum QCashTransactionKind {
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
pub struct QCashTransaction {
    pub version: u8,
    pub signer: Address,
    pub fee: Amount,
    pub nonce: AccountNonce,
    pub timestamp: u64,
    pub kind: QCashTransactionKind,
    pub validity: ValidityWindow,
}

#[derive(BorshSerialize)]
struct DepositTransactionCommitmentInput {
    version: u8,
    coin_id: [u8; 32],
    denomination: crate::qcash::CashDenomination,
    spend_public_key: PublicKey,
}

#[derive(BorshSerialize)]
struct DepositTransactionCommitmentPayload {
    version: u8,
    signer: Address,
    recipient: Address,
    fee: Amount,
    nonce: AccountNonce,
    timestamp: u64,
    validity: ValidityWindow,
    inputs: Vec<DepositTransactionCommitmentInput>,
}

impl QCashTransaction {
    pub fn withdraw(
        signer: Address,
        amount: Amount,
        fee: Amount,
        nonce: AccountNonce,
        metadata: WithdrawCashMetadata,
    ) -> Self {
        Self {
            version: QCASH_TRANSACTION_VERSION,
            signer,
            fee,
            nonce,
            timestamp: 0,
            kind: QCashTransactionKind::WithdrawCash { amount, metadata },
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
            version: QCASH_TRANSACTION_VERSION,
            signer,
            fee,
            nonce,
            timestamp: 0,
            kind: QCashTransactionKind::DepositCash {
                recipient,
                metadata,
            },
            validity: ValidityWindow::UNBOUNDED,
        }
    }

    pub fn deposit_from_files(
        signer: Address,
        recipient: Address,
        fee: Amount,
        nonce: AccountNonce,
        files: &[CashCoinFile],
    ) -> Result<Self, QCashError> {
        Self::deposit_from_files_at(signer, recipient, fee, nonce, 0, files)
    }

    pub fn deposit_from_files_at(
        signer: Address,
        recipient: Address,
        fee: Amount,
        nonce: AccountNonce,
        timestamp: u64,
        files: &[CashCoinFile],
    ) -> Result<Self, QCashError> {
        let placeholder_inputs = files
            .iter()
            .map(|file| file.deposit_input_for_transaction(recipient, [0; 32]))
            .collect::<Result<Vec<_>, _>>()?;
        let mut transaction = Self::deposit(
            signer,
            recipient,
            fee,
            nonce,
            DepositCashMetadata::from_inputs(placeholder_inputs)?,
        )
        .with_timestamp(timestamp);
        let commitment = transaction
            .deposit_transaction_commitment()
            .expect("deposit transaction should have a deposit commitment");
        transaction.kind = QCashTransactionKind::DepositCash {
            recipient,
            metadata: DepositCashMetadata::new_for_transaction(files, recipient, commitment)?,
        };
        Ok(transaction)
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
        if self.version != QCASH_TRANSACTION_VERSION {
            return Err(TransactionError::UnsupportedVersion);
        }
        match &self.kind {
            QCashTransactionKind::WithdrawCash { amount, metadata } => {
                if amount.0 == 0 {
                    return Err(TransactionError::ZeroAmount);
                }
                metadata
                    .validate_amount(*amount)
                    .map_err(|_| TransactionError::InvalidQCashMetadata)?;
            }
            QCashTransactionKind::DepositCash {
                recipient,
                metadata,
            } => {
                metadata
                    .validate_authorizations_for_transaction(
                        *recipient,
                        self.deposit_transaction_commitment()
                            .ok_or(TransactionError::InvalidQCashMetadata)?,
                    )
                    .map_err(|_| TransactionError::InvalidQCashMetadata)?;
                if *recipient == Address([0; 20]) {
                    return Err(TransactionError::InvalidQCashRecipient);
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
            QCashTransactionKind::WithdrawCash { amount, .. } => Ok(*amount),
            QCashTransactionKind::DepositCash { metadata, .. } => metadata
                .amount()
                .map_err(|_| TransactionError::InvalidQCashMetadata),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        canonical_bytes(self)
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        chain_bound_signing_bytes(QCASH_SIGNATURE_DOMAIN, self.to_bytes())
    }

    pub fn hash(&self) -> TransactionHash {
        TransactionHash(domain_hash(HashDomain::Transaction, &self.to_bytes()).0)
    }

    pub fn deposit_transaction_commitment(&self) -> Option<[u8; 32]> {
        let QCashTransactionKind::DepositCash {
            recipient,
            metadata,
        } = &self.kind
        else {
            return None;
        };
        let payload = DepositTransactionCommitmentPayload {
            version: self.version,
            signer: self.signer,
            recipient: *recipient,
            fee: self.fee,
            nonce: self.nonce,
            timestamp: self.timestamp,
            validity: self.validity,
            inputs: metadata
                .inputs
                .iter()
                .map(|input| DepositTransactionCommitmentInput {
                    version: input.version,
                    coin_id: input.coin_id,
                    denomination: input.denomination,
                    spend_public_key: input.spend_public_key,
                })
                .collect(),
        };
        Some(
            domain_hash(
                HashDomain::QCashDepositTransaction,
                &canonical_bytes(&payload),
            )
            .0,
        )
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SignedQCashTransaction {
    pub transaction: QCashTransaction,
    pub witness: Witness,
}

impl SignedQCashTransaction {
    pub fn new(transaction: QCashTransaction, public_key: PublicKey, signature: Signature) -> Self {
        Self {
            transaction,
            witness: Witness::new(public_key, signature),
        }
    }

    pub fn validate_signed(&self) -> Result<(), TransactionError> {
        self.transaction.validate()?;
        if self.to_bytes().len() > MAX_QCASH_TX_SIZE {
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
        super::SignedProtocolTransaction::QCash(self.clone()).wtxid()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        canonical_bytes(self)
    }
}
