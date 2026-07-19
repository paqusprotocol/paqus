//! Consensus UTXO set for QCash bearer outputs.

use crate::block::BlockHeight;
use crate::consensus::supply::Amount;
use crate::crypto::{
    Address, BlockHash, HASH_SIZE, Hash, HashDomain, TransactionHash, domain_hash,
};
use crate::qcash::{
    CashDenomination, DepositCashMetadata, QCashMetadata, QCashOperation, QCashOutput,
    WithdrawCashMetadata, cash_coin_id_bytes, cash_spend_public_key_commitment,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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

    /// Nine uppercase hexadecimal characters for a human-facing file name.
    pub fn short_id(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let mut value = String::with_capacity(9);
        for byte in self.0.iter().take(5) {
            value.push(HEX[(byte >> 4) as usize] as char);
            if value.len() == 9 {
                break;
            }
            value.push(HEX[(byte & 0x0f) as usize] as char);
        }
        value
    }

    pub fn file_name(&self, denomination: CashDenomination) -> String {
        format!("{}+{}.XPQ", denomination.xpq(), self.short_id())
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
    pub status: QCashUtxoStatus,
    pub issued_height: BlockHeight,
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
        let records: Vec<_> = self.coins.values().collect();
        domain_hash(
            HashDomain::QCashState,
            &crate::codec::canonical_bytes(&records),
        )
    }

    pub fn spendable_utxos(&self) -> impl Iterator<Item = &QCashUtxo> {
        self.coins
            .values()
            .filter(|coin| coin.status == QCashUtxoStatus::Spendable)
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

    pub fn total_value(&self) -> Result<Amount, QCashUtxoError> {
        self.utxos().try_fold(Amount(0), |total, coin| {
            total
                .0
                .checked_add(coin.denomination.amount().0)
                .map(Amount)
                .ok_or(QCashUtxoError::StateOverflow)
        })
    }

    /// Makes finalized withdrawal outputs spendable at the active chain tip.
    pub fn finalize_at(&mut self, tip_height: BlockHeight) {
        for coin in self.coins.values_mut() {
            if coin.status == QCashUtxoStatus::Pending
                && tip_height.0
                    >= coin
                        .issued_height
                        .0
                        .saturating_add(crate::ledger::QCASH_WITHDRAW_MATURITY as u64)
            {
                coin.status = QCashUtxoStatus::Spendable;
            }
        }
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
                    status: QCashUtxoStatus::Pending,
                    issued_height: height,
                },
            );
            ids.push(id);
        }
        Ok(ids)
    }

    /// Redeems issued coin IDs after matching them against deposit metadata.
    pub fn apply_deposit(
        &mut self,
        metadata: &QCashMetadata,
        coin_ids: &[CashCoinId],
    ) -> Result<(), QCashUtxoError> {
        metadata
            .validate()
            .map_err(|_| QCashUtxoError::InvalidMetadata)?;
        if metadata.operation != QCashOperation::Deposit {
            return Err(QCashUtxoError::WrongOperation);
        }
        let unique: BTreeSet<_> = coin_ids.iter().copied().collect();
        if unique.len() != coin_ids.len() {
            return Err(QCashUtxoError::DuplicateCoin);
        }

        let mut actual = BTreeMap::<CashDenomination, u64>::new();
        for id in coin_ids {
            let coin = self.coins.get(id).ok_or(QCashUtxoError::UnknownCoin)?;
            if coin.status != QCashUtxoStatus::Spendable {
                return Err(QCashUtxoError::CoinNotMature);
            }
            *actual.entry(coin.denomination).or_default() += 1;
        }
        let expected: BTreeMap<_, _> = metadata
            .coins
            .iter()
            .map(|run| (run.denomination, run.count))
            .collect();
        if actual != expected {
            return Err(QCashUtxoError::DenominationMismatch);
        }

        for id in coin_ids {
            self.coins.remove(id).expect("UTXOs were validated above");
        }
        Ok(())
    }

    /// Verifies bearer secrets and atomically redeems explicit deposit inputs.
    pub fn apply_deposit_proof(
        &mut self,
        metadata: &DepositCashMetadata,
        recipient: Address,
        height: BlockHeight,
    ) -> Result<Amount, QCashUtxoError> {
        metadata
            .validate_authorizations(recipient)
            .map_err(|_| QCashUtxoError::InvalidMetadata)?;
        let mut ids = Vec::with_capacity(metadata.inputs.len());
        for input in &metadata.inputs {
            let id = CashCoinId(input.coin_id);
            let coin = self.coins.get(&id).ok_or(QCashUtxoError::UnknownCoin)?;
            if coin.status != QCashUtxoStatus::Spendable {
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
        for id in ids {
            self.coins.remove(&id).expect("UTXOs were validated above");
        }
        let _ = height;
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
        let mut staged = self.clone();
        let ids = staged.apply_withdraw(withdrawer, withdraw_tx_hash, metadata, height)?;
        let journal = staged
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
        journal.issued_coin_ids.extend(ids.iter().copied());
        *self = staged;
        Ok(ids)
    }

    pub fn apply_deposit_in_block(
        &mut self,
        block_hash: BlockHash,
        height: BlockHeight,
        metadata: &DepositCashMetadata,
        recipient: Address,
    ) -> Result<Amount, QCashUtxoError> {
        let mut staged = self.clone();
        let mut previous = Vec::with_capacity(metadata.inputs.len());
        for input in &metadata.inputs {
            let id = CashCoinId(input.coin_id);
            previous.push(
                staged
                    .coins
                    .get(&id)
                    .cloned()
                    .ok_or(QCashUtxoError::UnknownCoin)?,
            );
        }
        let amount = staged.apply_deposit_proof(metadata, recipient, height)?;
        let journal = staged
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
        journal.spent_utxos.extend(previous);
        *self = staged;
        Ok(amount)
    }

    /// Reverses all QCash changes made by a disconnected block.
    pub fn rollback_block(&mut self, block_hash: BlockHash) -> Result<(), QCashUtxoError> {
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
        Ok(())
    }
}
