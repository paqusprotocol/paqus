use crate::block::{BlockHeight, Height, Nonce};
use crate::codec::{signed_transaction_bytes, transaction_bytes, transaction_hash};
use crate::consensus::supply::Amount;
use crate::crypto::{Address, PublicKey, Signature};
use crate::crypto::{TransactionHash, WitnessTransactionHash};
use crate::crypto::{address_from_public_key, verify};
pub use crate::error::TransactionError;
use crate::genesis::CURRENT_CHAIN_PARAMS;
use borsh::{BorshDeserialize, BorshSerialize};
use static_assertions::const_assert;

pub mod qcash;
pub use qcash::{QCashTransaction, QCashTransactionKind, SignedQCashTransaction};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SignedProtocolTransaction {
    Transfer(SignedTransaction),
    QCash(SignedQCashTransaction),
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransactionFamily {
    Transfer,
    QCash,
}

/// Maximum canonical unified envelope size.
pub const MAX_PROTOCOL_TRANSACTION_SIZE: usize = qcash::MAX_QCASH_TX_SIZE + 1;
const_assert!(MAX_TX_SIZE <= qcash::MAX_QCASH_TX_SIZE);

impl SignedProtocolTransaction {
    pub fn family(&self) -> TransactionFamily {
        match self {
            Self::Transfer(_) => TransactionFamily::Transfer,
            Self::QCash(_) => TransactionFamily::QCash,
        }
    }

    pub fn hash(&self) -> TransactionHash {
        match self {
            Self::Transfer(tx) => tx.hash(),
            Self::QCash(tx) => tx.hash(),
        }
    }

    /// Commits to the family, payload, public keys, signatures, and approvals.
    pub fn wtxid(&self) -> WitnessTransactionHash {
        crate::codec::signed_protocol_transaction_hash(self)
    }

    /// Unified envelope size without public keys, signatures, or approvals.
    pub fn stripped_size(&self) -> usize {
        1 + match self {
            Self::Transfer(tx) => tx.transaction.to_bytes().len(),
            Self::QCash(tx) => tx.transaction.to_bytes().len(),
        }
    }

    pub fn witness_size(&self) -> usize {
        self.to_bytes().len().saturating_sub(self.stripped_size())
    }

    pub fn weight(&self) -> usize {
        self.stripped_size()
            .saturating_mul(crate::block::WITNESS_SCALE_FACTOR)
            .saturating_add(self.witness_size())
    }

    pub fn virtual_size(&self) -> usize {
        self.weight()
            .saturating_add(crate::block::WITNESS_SCALE_FACTOR - 1)
            / crate::block::WITNESS_SCALE_FACTOR
    }

    pub fn signer(&self) -> Address {
        match self {
            Self::Transfer(tx) => tx.transaction.from,
            Self::QCash(tx) => tx.transaction.signer,
        }
    }

    pub fn nonce(&self) -> AccountNonce {
        match self {
            Self::Transfer(tx) => tx.transaction.nonce,
            Self::QCash(tx) => tx.transaction.nonce,
        }
    }

    pub fn fee(&self) -> Amount {
        match self {
            Self::Transfer(tx) => tx.transaction.fee,
            Self::QCash(tx) => tx.transaction.fee,
        }
    }

    pub fn validity(&self) -> ValidityWindow {
        match self {
            Self::Transfer(tx) => tx.transaction.validity,
            Self::QCash(tx) => tx.transaction.validity,
        }
    }

    /// Returns every public key carried by the transaction witness.
    ///
    /// This is an inspection API; callers must still run normal transaction
    /// validation before trusting the key or its derived address.
    pub fn witness_public_keys(&self) -> Vec<&PublicKey> {
        match self {
            Self::Transfer(tx) => vec![&tx.witness.public_key],
            Self::QCash(tx) => vec![&tx.witness.public_key],
        }
    }

    /// Returns the envelope's single witness public key.
    pub fn single_witness_public_key(&self) -> Option<&PublicKey> {
        match self {
            Self::Transfer(tx) => Some(&tx.witness.public_key),
            Self::QCash(tx) => Some(&tx.witness.public_key),
        }
    }

    /// Derives signer addresses from all public keys carried by the witness.
    pub fn witness_addresses(&self) -> Vec<Address> {
        self.witness_public_keys()
            .into_iter()
            .map(address_from_public_key)
            .collect()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        crate::codec::signed_protocol_transaction_bytes(self)
    }
}

impl From<SignedTransaction> for SignedProtocolTransaction {
    fn from(transaction: SignedTransaction) -> Self {
        Self::Transfer(transaction)
    }
}
impl From<SignedQCashTransaction> for SignedProtocolTransaction {
    fn from(transaction: SignedQCashTransaction) -> Self {
        Self::QCash(transaction)
    }
}

pub const MAX_TX_SIZE: usize = 10 * 1024;

pub type AccountNonce = Nonce;
pub type TransactionHeight = Height;

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValidityWindow {
    pub valid_from: BlockHeight,
    pub valid_until: BlockHeight,
}

impl Default for ValidityWindow {
    fn default() -> Self {
        Self::UNBOUNDED
    }
}

impl ValidityWindow {
    pub const UNBOUNDED: Self = Self {
        valid_from: Height(0),
        valid_until: Height(u64::MAX),
    };

    pub fn new(
        valid_from: BlockHeight,
        valid_until: BlockHeight,
    ) -> Result<Self, TransactionError> {
        let window = Self {
            valid_from,
            valid_until,
        };
        window.validate()?;
        Ok(window)
    }

    pub fn validate(self) -> Result<(), TransactionError> {
        if self.valid_from.0 > self.valid_until.0 {
            return Err(TransactionError::InvalidValidityWindow);
        }
        Ok(())
    }

    pub fn validate_at(self, height: BlockHeight) -> Result<(), TransactionError> {
        self.validate()?;
        if height.0 < self.valid_from.0 {
            return Err(TransactionError::NotYetValid);
        }
        if height.0 > self.valid_until.0 {
            return Err(TransactionError::ValidityExpired);
        }
        Ok(())
    }
}

const TRANSACTION_SIGNATURE_DOMAIN: &[u8] = b"PAQUSCORE_TX_V1";

#[derive(BorshSerialize)]
struct TransactionSigningContext {
    chain_id: u16,
    protocol_version: u8,
    genesis_hash: [u8; crate::crypto::HASH_SIZE],
    payload: Vec<u8>,
}

pub(crate) fn chain_bound_signing_bytes(domain: &[u8], payload: Vec<u8>) -> Vec<u8> {
    let context = TransactionSigningContext {
        chain_id: CURRENT_CHAIN_PARAMS.chain_id,
        protocol_version: CURRENT_CHAIN_PARAMS.protocol_version,
        genesis_hash: CURRENT_CHAIN_PARAMS.genesis.hash,
        payload,
    };
    let context_bytes = crate::codec::canonical_bytes(&context);
    let mut bytes = Vec::with_capacity(domain.len() + context_bytes.len());
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&context_bytes);
    bytes
}
pub const TRANSACTION_VERSION: u8 = 1;
pub const MAX_BATCH_OUTPUTS: usize = 64;

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TransferOutput {
    pub to: Address,
    pub amount: Amount,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub version: u8,
    pub from: Address,
    pub to: Address,
    pub amount: Amount,
    pub additional_outputs: Vec<TransferOutput>,
    pub fee: Amount,
    pub nonce: AccountNonce,
    pub timestamp: u64,
    pub validity: ValidityWindow,
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
            version: TRANSACTION_VERSION,
            from,
            to,
            amount,
            additional_outputs: Vec::new(),
            fee,
            nonce,
            timestamp,
            validity: ValidityWindow::UNBOUNDED,
        }
    }

    pub fn with_validity_window(mut self, validity: ValidityWindow) -> Self {
        self.validity = validity;
        self
    }

    pub fn with_additional_outputs(mut self, outputs: Vec<TransferOutput>) -> Self {
        self.additional_outputs = outputs;
        self
    }

    pub fn outputs(&self) -> impl Iterator<Item = TransferOutput> + '_ {
        std::iter::once(TransferOutput {
            to: self.to,
            amount: self.amount,
        })
        .chain(self.additional_outputs.iter().copied())
    }

    pub fn total_amount(&self) -> Result<Amount, TransactionError> {
        self.outputs()
            .try_fold(0_u64, |total, output| total.checked_add(output.amount.0))
            .map(Amount)
            .ok_or(TransactionError::AmountOverflow)
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        if self.version != TRANSACTION_VERSION {
            return Err(TransactionError::UnsupportedVersion);
        }
        if self.amount.0 == 0 {
            return Err(TransactionError::ZeroAmount);
        }
        if self.from == self.to {
            return Err(TransactionError::SameSenderAndRecipient);
        }
        if self.additional_outputs.len() + 1 > MAX_BATCH_OUTPUTS {
            return Err(TransactionError::TooManyOutputs);
        }
        let mut recipients = std::collections::BTreeSet::new();
        for output in self.outputs() {
            if output.amount.0 == 0 {
                return Err(TransactionError::ZeroAmount);
            }
            if output.to == self.from {
                return Err(TransactionError::SameSenderAndRecipient);
            }
            if !recipients.insert(output.to) {
                return Err(TransactionError::DuplicateRecipient);
            }
        }
        self.total_amount()?;
        self.validity.validate()
    }

    pub fn validate_for_height(
        &self,
        height: crate::block::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.validate()?;
        self.validity.validate_at(height)
    }

    /// Validates structure while accepting a caller timestamp for API symmetry.
    ///
    /// Transfer transaction time validity is height-based through
    /// `ValidityWindow`; `timestamp` is signed metadata and is not a mempool
    /// or consensus clock bound here.
    pub fn validate_at(&self, _now: u64) -> Result<(), TransactionError> {
        self.validate()
    }

    /// Validates structure at a block height. The timestamp parameter is kept
    /// for call-site symmetry with block-level validation; transfer validity is
    /// height-based.
    pub fn validate_at_height(
        &self,
        _now: u64,
        height: crate::block::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.validate_for_height(height)
    }

    pub fn hash(&self) -> TransactionHash {
        transaction_hash(self)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        transaction_bytes(self)
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        chain_bound_signing_bytes(TRANSACTION_SIGNATURE_DOMAIN, self.to_bytes())
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
        self.transaction.validate()?;
        self.validate_witness_and_size()
    }

    pub fn validate_for_height(
        &self,
        height: crate::block::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.transaction.validate_for_height(height)?;
        self.validate_witness_and_size()
    }

    pub fn validate_at(&self, now: u64) -> Result<(), TransactionError> {
        self.validate_at_height(now, crate::block::Height(0))
    }

    pub fn validate_at_height(
        &self,
        now: u64,
        height: crate::block::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.transaction.validate_at_height(now, height)?;
        self.validate_witness_and_size()
    }

    fn validate_witness_and_size(&self) -> Result<(), TransactionError> {
        if self.serialized_size() > MAX_TX_SIZE {
            return Err(TransactionError::TransactionTooLarge);
        }
        // Cheap sentinel checks only; full key/signature validity is enforced
        // by `verify_signature`.
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
        self.validate()?;
        if self.sender_address() != self.transaction.from {
            return Err(TransactionError::SenderAddressMismatch);
        }
        self.verify_signature()
    }

    pub fn validate_signed_for_height(
        &self,
        height: crate::block::BlockHeight,
    ) -> Result<(), TransactionError> {
        self.validate_for_height(height)?;

        if self.sender_address() != self.transaction.from {
            return Err(TransactionError::SenderAddressMismatch);
        }

        self.verify_signature()
    }

    pub fn validate_signed_at(&self, now: u64) -> Result<(), TransactionError> {
        self.validate_signed_at_height(now, crate::block::Height(0))
    }

    pub fn validate_signed_at_height(
        &self,
        now: u64,
        height: crate::block::BlockHeight,
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

    pub fn wtxid(&self) -> WitnessTransactionHash {
        SignedProtocolTransaction::Transfer(self.clone()).wtxid()
    }

    pub fn stripped_size(&self) -> usize {
        self.transaction.to_bytes().len()
    }

    pub fn witness_size(&self) -> usize {
        self.serialized_size().saturating_sub(self.stripped_size())
    }

    pub fn weight(&self) -> usize {
        self.stripped_size()
            .saturating_mul(crate::block::WITNESS_SCALE_FACTOR)
            .saturating_add(self.witness_size())
    }

    pub fn virtual_size(&self) -> usize {
        self.weight()
            .saturating_add(crate::block::WITNESS_SCALE_FACTOR - 1)
            / crate::block::WITNESS_SCALE_FACTOR
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
