//! Consensus UTXO set for QCash bearer outputs.

use crate::block::{BlockHeight, Height};
use crate::consensus::supply::Amount;
use crate::crypto::{
    Address, BlockHash, HASH_SIZE, Hash, HashDomain, TransactionHash, domain_hash,
};
use crate::qcash::{
    CashDenomination, DepositCashMetadata, QCashOutput, WithdrawCashMetadata, cash_coin_id_bytes,
    cash_spend_public_key_commitment,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct CashCoinId(pub [u8; HASH_SIZE]);

/// Canonical origin of one QCash output.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct QCashOutPoint {
    pub transaction_hash: TransactionHash,
    pub output_index: u32,
}

impl CashCoinId {
    pub fn derive(withdraw_tx_hash: TransactionHash, output: &QCashOutput) -> Self {
        Self(cash_coin_id_bytes(withdraw_tx_hash, output))
    }

    /// Sixteen uppercase hexadecimal characters for a human-facing file name.
    pub fn short_id(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let mut value = String::with_capacity(16);
        for byte in self.0.iter().take(8) {
            value.push(HEX[(byte >> 4) as usize] as char);
            value.push(HEX[(byte & 0x0f) as usize] as char);
        }
        value
    }

    pub fn file_name(&self, denomination: CashDenomination) -> String {
        format!("{}_{}.XPQ", denomination.xpq(), self.short_id())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub enum QCashUtxoStatus {
    Pending,
    Spendable,
}

/// One individually tracked cash coin.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct QCashUtxo {
    pub id: CashCoinId,
    pub outpoint: QCashOutPoint,
    pub withdrawer: Address,
    pub denomination: CashDenomination,
    pub commitment: [u8; 32],
    pub issued_height: BlockHeight,
}

impl QCashUtxo {
    pub fn status_at(&self, height: BlockHeight) -> QCashUtxoStatus {
        if is_spendable_at(self, height) {
            QCashUtxoStatus::Spendable
        } else {
            QCashUtxoStatus::Pending
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct QCashBlockJournal {
    pub block_hash: BlockHash,
    pub block_height: BlockHeight,
    pub issued_coin_ids: Vec<CashCoinId>,
    pub spent_utxos: Vec<QCashUtxo>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default,
)]
pub struct QCashUtxoSet {
    coins: BTreeMap<CashCoinId, QCashUtxo>,
    journals: BTreeMap<BlockHash, QCashBlockJournal>,
    active_journal_tip: Option<BlockHash>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QCashUtxoError {
    InvalidMetadata,
    WrongOperation,
    StateOverflow,
    UnknownCoin,
    DuplicateCoin,
    DenominationMismatch,
    CoinIdCollision,
    InvalidCoinProof,
    CoinNotMature,
    MissingBlockJournal,
    NonTipRollback,
}

impl fmt::Display for QCashUtxoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata => f.write_str("invalid QCash metadata"),
            Self::WrongOperation => {
                f.write_str("QCash metadata operation does not match state operation")
            }
            Self::StateOverflow => f.write_str("QCash UTXO value overflow"),
            Self::UnknownCoin => f.write_str("QCash output is unknown or already spent"),
            Self::DuplicateCoin => f.write_str("cash coin is repeated in the operation"),
            Self::DenominationMismatch => {
                f.write_str("cash coin denominations do not match metadata")
            }
            Self::CoinIdCollision => f.write_str("derived cash coin ID already exists"),
            Self::InvalidCoinProof => f.write_str("cash coin proof does not match issued output"),
            Self::CoinNotMature => f.write_str("cash coin has not reached QCash finality maturity"),
            Self::MissingBlockJournal => f.write_str("QCash block journal was not found"),
            Self::NonTipRollback => f.write_str("QCash rollback must disconnect the journal tip"),
        }
    }
}

impl Error for QCashUtxoError {}

impl QCashUtxoSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn coin(&self, id: CashCoinId) -> Option<&QCashUtxo> {
        self.coins.get(&id)
    }

    pub fn coins(&self) -> impl Iterator<Item = &QCashUtxo> {
        self.coins.values()
    }

    pub fn journal(&self, block_hash: BlockHash) -> Option<&QCashBlockJournal> {
        self.journals.get(&block_hash)
    }

    /// Consensus commitment excluding local rollback journals and event counters.
    pub fn consensus_root(&self) -> Hash {
        domain_hash(
            HashDomain::QCashState,
            &crate::codec::canonical_bytes(&self.coins),
        )
    }

    pub fn spendable_utxos_at(&self, height: BlockHeight) -> impl Iterator<Item = &QCashUtxo> {
        self.coins
            .values()
            .filter(move |coin| is_spendable_at(coin, height))
    }

    pub fn spendable_utxos(&self) -> impl Iterator<Item = &QCashUtxo> {
        self.spendable_utxos_at(Height(u64::MAX))
    }

    pub fn utxos(&self) -> impl Iterator<Item = &QCashUtxo> {
        self.coins.values()
    }

    pub fn spendable_balance(&self) -> Result<Amount, QCashUtxoError> {
        self.spendable_utxos().try_fold(Amount(0), |total, coin| {
            total
                .0
                .checked_add(coin.denomination.amount().0)
                .map(Amount)
                .ok_or(QCashUtxoError::StateOverflow)
        })
    }

    pub fn spendable_balance_at(&self, height: BlockHeight) -> Result<Amount, QCashUtxoError> {
        self.spendable_utxos_at(height)
            .try_fold(Amount(0), |total, coin| {
                total
                    .0
                    .checked_add(coin.denomination.amount().0)
                    .map(Amount)
                    .ok_or(QCashUtxoError::StateOverflow)
            })
    }

    pub fn total_value(&self) -> Result<Amount, QCashUtxoError> {
        self.utxos().try_fold(Amount(0), |total, coin| {
            total
                .0
                .checked_add(coin.denomination.amount().0)
                .map(Amount)
                .ok_or(QCashUtxoError::StateOverflow)
        })
    }

    /// Maturity is derived from active height. Kept as a no-op for callers that
    /// previously advanced cached wallet/RPC status through this API.
    pub fn finalize_at(&mut self, tip_height: BlockHeight) {
        let _ = tip_height;
    }

    /// Issues and stores every coin represented by withdraw metadata.
    pub fn apply_withdraw(
        &mut self,
        withdrawer: Address,
        withdraw_tx_hash: TransactionHash,
        metadata: &WithdrawCashMetadata,
        height: BlockHeight,
    ) -> Result<Vec<CashCoinId>, QCashUtxoError> {
        metadata
            .validate()
            .map_err(|_| QCashUtxoError::InvalidMetadata)?;
        let mut pending: Vec<(CashCoinId, &QCashOutput)> =
            Vec::with_capacity(metadata.outputs.len());
        for output in &metadata.outputs {
            let id = CashCoinId::derive(withdraw_tx_hash, output);
            if self.coins.contains_key(&id)
                || pending.iter().any(|(pending_id, _)| *pending_id == id)
            {
                return Err(QCashUtxoError::CoinIdCollision);
            }
            pending.push((id, output));
        }

        let mut ids = Vec::with_capacity(pending.len());
        for (id, output) in pending {
            self.coins.insert(
                id,
                QCashUtxo {
                    id,
                    outpoint: QCashOutPoint {
                        transaction_hash: withdraw_tx_hash,
                        output_index: output.coin_index,
                    },
                    withdrawer,
                    denomination: output.denomination,
                    commitment: output.commitment,
                    issued_height: height,
                },
            );
            ids.push(id);
        }
        Ok(ids)
    }

    /// Verifies bearer secrets and atomically redeems explicit deposit inputs.
    pub fn apply_deposit_proof(
        &mut self,
        metadata: &DepositCashMetadata,
        recipient: Address,
        height: BlockHeight,
        transaction_commitment: [u8; 32],
    ) -> Result<Amount, QCashUtxoError> {
        metadata
            .validate_authorizations_for_transaction(recipient, transaction_commitment)
            .map_err(|_| QCashUtxoError::InvalidMetadata)?;
        let (ids, amount) = self.validate_deposit_proof(metadata, height)?;
        for id in ids {
            self.coins.remove(&id).expect("UTXOs were validated above");
        }
        Ok(amount)
    }

    pub fn apply_withdraw_in_block(
        &mut self,
        block_hash: BlockHash,
        height: BlockHeight,
        withdrawer: Address,
        withdraw_tx_hash: TransactionHash,
        metadata: &WithdrawCashMetadata,
    ) -> Result<Vec<CashCoinId>, QCashUtxoError> {
        metadata
            .validate()
            .map_err(|_| QCashUtxoError::InvalidMetadata)?;
        let journal = self
            .journals
            .entry(block_hash)
            .or_insert_with(|| QCashBlockJournal {
                block_hash,
                block_height: height,
                issued_coin_ids: Vec::new(),
                spent_utxos: Vec::new(),
            });
        if journal.block_height != height {
            return Err(QCashUtxoError::InvalidMetadata);
        }
        let ids = self.apply_withdraw(withdrawer, withdraw_tx_hash, metadata, height)?;
        let journal = self
            .journals
            .get_mut(&block_hash)
            .expect("journal was created above");
        journal.issued_coin_ids.extend(ids.iter().copied());
        self.active_journal_tip = Some(block_hash);
        Ok(ids)
    }

    pub fn apply_deposit_in_block(
        &mut self,
        block_hash: BlockHash,
        height: BlockHeight,
        metadata: &DepositCashMetadata,
        recipient: Address,
        transaction_commitment: [u8; 32],
    ) -> Result<Amount, QCashUtxoError> {
        metadata
            .validate_authorizations_for_transaction(recipient, transaction_commitment)
            .map_err(|_| QCashUtxoError::InvalidMetadata)?;
        let (ids, amount) = self.validate_deposit_proof(metadata, height)?;
        let previous = ids
            .iter()
            .map(|id| {
                self.coins
                    .get(id)
                    .cloned()
                    .ok_or(QCashUtxoError::UnknownCoin)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let journal = self
            .journals
            .entry(block_hash)
            .or_insert_with(|| QCashBlockJournal {
                block_hash,
                block_height: height,
                issued_coin_ids: Vec::new(),
                spent_utxos: Vec::new(),
            });
        if journal.block_height != height {
            return Err(QCashUtxoError::InvalidMetadata);
        }
        for id in &ids {
            self.coins.remove(id).expect("UTXOs were validated above");
        }
        let journal = self
            .journals
            .get_mut(&block_hash)
            .expect("journal was created above");
        journal.spent_utxos.extend(previous);
        self.active_journal_tip = Some(block_hash);
        Ok(amount)
    }

    /// Reverses all QCash changes made by a disconnected block.
    pub fn rollback_block(&mut self, block_hash: BlockHash) -> Result<(), QCashUtxoError> {
        if self.active_journal_tip != Some(block_hash) {
            return Err(QCashUtxoError::NonTipRollback);
        }
        let journal = self
            .journals
            .remove(&block_hash)
            .ok_or(QCashUtxoError::MissingBlockJournal)?;
        for previous in journal.spent_utxos.into_iter().rev() {
            self.coins.insert(previous.id, previous);
        }
        for id in journal.issued_coin_ids {
            self.coins.remove(&id);
        }
        self.active_journal_tip = None;
        Ok(())
    }

    pub fn set_active_journal_tip(
        &mut self,
        block_hash: Option<BlockHash>,
    ) -> Result<(), QCashUtxoError> {
        if let Some(hash) = block_hash
            && !self.journals.contains_key(&hash)
        {
            return Err(QCashUtxoError::MissingBlockJournal);
        }
        self.active_journal_tip = block_hash;
        Ok(())
    }

    pub fn prune_journals(&mut self, finalized_height: BlockHeight) {
        self.journals
            .retain(|_, journal| journal.block_height > finalized_height);
        if self
            .active_journal_tip
            .is_some_and(|tip| !self.journals.contains_key(&tip))
        {
            self.active_journal_tip = None;
        }
    }

    fn validate_deposit_proof(
        &self,
        metadata: &DepositCashMetadata,
        height: BlockHeight,
    ) -> Result<(Vec<CashCoinId>, Amount), QCashUtxoError> {
        let mut ids = Vec::with_capacity(metadata.inputs.len());
        for input in &metadata.inputs {
            let id = CashCoinId(input.coin_id);
            let coin = self.coins.get(&id).ok_or(QCashUtxoError::UnknownCoin)?;
            if !is_spendable_at(coin, height) {
                return Err(QCashUtxoError::CoinNotMature);
            }
            if coin.denomination != input.denomination
                || coin.commitment != cash_spend_public_key_commitment(&input.spend_public_key)
            {
                return Err(QCashUtxoError::InvalidCoinProof);
            }
            ids.push(id);
        }
        let amount = metadata
            .amount()
            .map_err(|_| QCashUtxoError::InvalidMetadata)?;
        Ok((ids, amount))
    }
}

fn is_spendable_at(coin: &QCashUtxo, height: BlockHeight) -> bool {
    coin.issued_height
        .0
        .checked_add(crate::ledger::QCASH_WITHDRAW_MATURITY as u64)
        .is_some_and(|maturity_height| height.0 >= maturity_height)
}
