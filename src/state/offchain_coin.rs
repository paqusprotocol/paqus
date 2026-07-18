//! Local state for tracking eCash coins outside account consensus state.

use crate::block::BlockHeight;
use crate::consensus::supply::Amount;
use crate::crypto::{
    Address, BlockHash, HASH_SIZE, Hash, HashDomain, TransactionHash, domain_hash,
};
use crate::ecash::{
    CashDenomination, DepositCashMetadata, EcashMetadata, EcashOperation, EcashOutput,
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

impl CashCoinId {
    pub fn derive(withdraw_tx_hash: TransactionHash, output: &EcashOutput) -> Self {
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
pub enum OffchainCoinStatus {
    PendingIssue,
    Issued,
    PendingRedeem,
    Redeemed,
    Orphaned,
}

/// One individually tracked cash coin.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct OffchainCashCoin {
    pub id: CashCoinId,
    pub withdrawer: Address,
    pub withdraw_tx_hash: TransactionHash,
    pub coin_index: u32,
    pub denomination: CashDenomination,
    pub commitment: [u8; 32],
    pub status: OffchainCoinStatus,
    pub issued_height: BlockHeight,
    pub redeem_requested_height: Option<BlockHeight>,
    pub issued_event: u64,
    pub redeemed_event: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct EcashBlockJournal {
    pub block_hash: BlockHash,
    pub block_height: BlockHeight,
    /// Event sequence before the first eCash transition in this block.
    pub previous_next_event: u64,
    pub issued_coin_ids: Vec<CashCoinId>,
    pub previous_coins: Vec<OffchainCashCoin>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default,
)]
pub struct OffchainCoinState {
    next_event: u64,
    coins: BTreeMap<CashCoinId, OffchainCashCoin>,
    journals: BTreeMap<BlockHash, EcashBlockJournal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffchainCoinError {
    InvalidMetadata,
    WrongOperation,
    StateOverflow,
    UnknownCoin,
    DuplicateCoin,
    CoinAlreadyRedeemed,
    DenominationMismatch,
    CoinIdCollision,
    InvalidCoinProof,
    CoinNotMature,
    MissingBlockJournal,
}

impl fmt::Display for OffchainCoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata => f.write_str("invalid eCash metadata"),
            Self::WrongOperation => {
                f.write_str("eCash metadata operation does not match state operation")
            }
            Self::StateOverflow => f.write_str("offchain coin state sequence overflow"),
            Self::UnknownCoin => f.write_str("cash coin is not present in offchain state"),
            Self::DuplicateCoin => f.write_str("cash coin is repeated in the operation"),
            Self::CoinAlreadyRedeemed => f.write_str("cash coin has already been redeemed"),
            Self::DenominationMismatch => {
                f.write_str("cash coin denominations do not match metadata")
            }
            Self::CoinIdCollision => f.write_str("derived cash coin ID already exists"),
            Self::InvalidCoinProof => f.write_str("cash coin proof does not match issued output"),
            Self::CoinNotMature => f.write_str("cash coin has not reached eCash finality maturity"),
            Self::MissingBlockJournal => f.write_str("eCash block journal was not found"),
        }
    }
}

impl Error for OffchainCoinError {}

impl OffchainCoinState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn coin(&self, id: CashCoinId) -> Option<&OffchainCashCoin> {
        self.coins.get(&id)
    }

    pub fn coins(&self) -> impl Iterator<Item = &OffchainCashCoin> {
        self.coins.values()
    }

    pub fn journal(&self, block_hash: BlockHash) -> Option<&EcashBlockJournal> {
        self.journals.get(&block_hash)
    }

    /// Consensus commitment excluding local rollback journals and event counters.
    pub fn consensus_root(&self) -> Hash {
        let records: Vec<_> = self.coins.values().collect();
        domain_hash(
            HashDomain::EcashState,
            &crate::codec::canonical_bytes(&records),
        )
    }

    pub fn issued_coins(&self) -> impl Iterator<Item = &OffchainCashCoin> {
        self.coins
            .values()
            .filter(|coin| coin.status == OffchainCoinStatus::Issued)
    }

    pub fn outstanding_coins(&self) -> impl Iterator<Item = &OffchainCashCoin> {
        self.coins.values().filter(|coin| {
            matches!(
                coin.status,
                OffchainCoinStatus::PendingIssue
                    | OffchainCoinStatus::Issued
                    | OffchainCoinStatus::PendingRedeem
            )
        })
    }

    pub fn issued_balance(&self) -> Result<Amount, OffchainCoinError> {
        self.issued_coins().try_fold(Amount(0), |total, coin| {
            total
                .0
                .checked_add(coin.denomination.amount().0)
                .map(Amount)
                .ok_or(OffchainCoinError::StateOverflow)
        })
    }

    pub fn outstanding_balance(&self) -> Result<Amount, OffchainCoinError> {
        self.outstanding_coins().try_fold(Amount(0), |total, coin| {
            total
                .0
                .checked_add(coin.denomination.amount().0)
                .map(Amount)
                .ok_or(OffchainCoinError::StateOverflow)
        })
    }

    /// Finalizes pending issue and redemption states at the active chain tip.
    pub fn finalize_at(&mut self, tip_height: BlockHeight) {
        for coin in self.coins.values_mut() {
            match coin.status {
                OffchainCoinStatus::PendingIssue
                    if tip_height.0
                        >= coin
                            .issued_height
                            .0
                            .saturating_add(crate::ledger::ECASH_WITHDRAW_MATURITY as u64) =>
                {
                    coin.status = OffchainCoinStatus::Issued;
                }
                OffchainCoinStatus::PendingRedeem
                    if coin.redeem_requested_height.is_some_and(|height| {
                        tip_height.0
                            >= height
                                .0
                                .saturating_add(crate::ledger::ECASH_DEPOSIT_MATURITY as u64)
                    }) =>
                {
                    coin.status = OffchainCoinStatus::Redeemed;
                }
                _ => {}
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
    ) -> Result<Vec<CashCoinId>, OffchainCoinError> {
        metadata
            .validate()
            .map_err(|_| OffchainCoinError::InvalidMetadata)?;
        let event = self.next_event;
        let next_event = event
            .checked_add(1)
            .ok_or(OffchainCoinError::StateOverflow)?;

        let mut pending: Vec<(CashCoinId, &EcashOutput)> =
            Vec::with_capacity(metadata.outputs.len());
        for output in &metadata.outputs {
            let id = CashCoinId::derive(withdraw_tx_hash, output);
            if self.coins.contains_key(&id)
                || pending.iter().any(|(pending_id, _)| *pending_id == id)
            {
                return Err(OffchainCoinError::CoinIdCollision);
            }
            pending.push((id, output));
        }

        let mut ids = Vec::with_capacity(pending.len());
        for (id, output) in pending {
            self.coins.insert(
                id,
                OffchainCashCoin {
                    id,
                    withdrawer,
                    withdraw_tx_hash,
                    coin_index: output.coin_index,
                    denomination: output.denomination,
                    commitment: output.commitment,
                    status: OffchainCoinStatus::PendingIssue,
                    issued_height: height,
                    redeem_requested_height: None,
                    issued_event: event,
                    redeemed_event: None,
                },
            );
            ids.push(id);
        }
        self.next_event = next_event;
        Ok(ids)
    }

    /// Redeems issued coin IDs after matching them against deposit metadata.
    pub fn apply_deposit(
        &mut self,
        metadata: &EcashMetadata,
        coin_ids: &[CashCoinId],
    ) -> Result<(), OffchainCoinError> {
        metadata
            .validate()
            .map_err(|_| OffchainCoinError::InvalidMetadata)?;
        if metadata.operation != EcashOperation::Deposit {
            return Err(OffchainCoinError::WrongOperation);
        }
        let unique: BTreeSet<_> = coin_ids.iter().copied().collect();
        if unique.len() != coin_ids.len() {
            return Err(OffchainCoinError::DuplicateCoin);
        }

        let mut actual = BTreeMap::<CashDenomination, u64>::new();
        for id in coin_ids {
            let coin = self.coins.get(id).ok_or(OffchainCoinError::UnknownCoin)?;
            if coin.status != OffchainCoinStatus::Issued {
                return Err(
                    if matches!(
                        coin.status,
                        OffchainCoinStatus::PendingRedeem | OffchainCoinStatus::Redeemed
                    ) {
                        OffchainCoinError::CoinAlreadyRedeemed
                    } else {
                        OffchainCoinError::CoinNotMature
                    },
                );
            }
            *actual.entry(coin.denomination).or_default() += 1;
        }
        let expected: BTreeMap<_, _> = metadata
            .coins
            .iter()
            .map(|run| (run.denomination, run.count))
            .collect();
        if actual != expected {
            return Err(OffchainCoinError::DenominationMismatch);
        }

        let event = self.next_event;
        self.next_event = event
            .checked_add(1)
            .ok_or(OffchainCoinError::StateOverflow)?;
        for id in coin_ids {
            let coin = self.coins.get_mut(id).expect("coins were validated above");
            coin.status = OffchainCoinStatus::Redeemed;
            coin.redeemed_event = Some(event);
        }
        Ok(())
    }

    /// Verifies bearer secrets and atomically redeems explicit deposit inputs.
    pub fn apply_deposit_proof(
        &mut self,
        metadata: &DepositCashMetadata,
        recipient: Address,
        height: BlockHeight,
    ) -> Result<Amount, OffchainCoinError> {
        metadata
            .validate_authorizations(recipient)
            .map_err(|_| OffchainCoinError::InvalidMetadata)?;
        let mut ids = Vec::with_capacity(metadata.inputs.len());
        for input in &metadata.inputs {
            let id = CashCoinId(input.coin_id);
            let coin = self.coins.get(&id).ok_or(OffchainCoinError::UnknownCoin)?;
            if coin.status != OffchainCoinStatus::Issued {
                return Err(
                    if matches!(
                        coin.status,
                        OffchainCoinStatus::PendingRedeem | OffchainCoinStatus::Redeemed
                    ) {
                        OffchainCoinError::CoinAlreadyRedeemed
                    } else {
                        OffchainCoinError::CoinNotMature
                    },
                );
            }
            if coin.denomination != input.denomination
                || coin.commitment != cash_spend_public_key_commitment(&input.spend_public_key)
            {
                return Err(OffchainCoinError::InvalidCoinProof);
            }
            ids.push(id);
        }

        let amount = metadata
            .amount()
            .map_err(|_| OffchainCoinError::InvalidMetadata)?;
        let event = self.next_event;
        self.next_event = event
            .checked_add(1)
            .ok_or(OffchainCoinError::StateOverflow)?;
        for id in ids {
            let coin = self.coins.get_mut(&id).expect("coins were validated above");
            coin.status = OffchainCoinStatus::PendingRedeem;
            coin.redeem_requested_height = Some(height);
            coin.redeemed_event = Some(event);
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
    ) -> Result<Vec<CashCoinId>, OffchainCoinError> {
        let mut staged = self.clone();
        let previous_next_event = staged.next_event;
        let ids = staged.apply_withdraw(withdrawer, withdraw_tx_hash, metadata, height)?;
        let journal = staged
            .journals
            .entry(block_hash)
            .or_insert_with(|| EcashBlockJournal {
                block_hash,
                block_height: height,
                previous_next_event,
                issued_coin_ids: Vec::new(),
                previous_coins: Vec::new(),
            });
        if journal.block_height != height {
            return Err(OffchainCoinError::InvalidMetadata);
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
    ) -> Result<Amount, OffchainCoinError> {
        let mut staged = self.clone();
        let previous_next_event = staged.next_event;
        let mut previous = Vec::with_capacity(metadata.inputs.len());
        for input in &metadata.inputs {
            let id = CashCoinId(input.coin_id);
            previous.push(
                staged
                    .coins
                    .get(&id)
                    .cloned()
                    .ok_or(OffchainCoinError::UnknownCoin)?,
            );
        }
        let amount = staged.apply_deposit_proof(metadata, recipient, height)?;
        let journal = staged
            .journals
            .entry(block_hash)
            .or_insert_with(|| EcashBlockJournal {
                block_hash,
                block_height: height,
                previous_next_event,
                issued_coin_ids: Vec::new(),
                previous_coins: Vec::new(),
            });
        if journal.block_height != height {
            return Err(OffchainCoinError::InvalidMetadata);
        }
        journal.previous_coins.extend(previous);
        *self = staged;
        Ok(amount)
    }

    /// Reverses all eCash changes made by a disconnected block.
    pub fn rollback_block(&mut self, block_hash: BlockHash) -> Result<(), OffchainCoinError> {
        let journal = self
            .journals
            .remove(&block_hash)
            .ok_or(OffchainCoinError::MissingBlockJournal)?;
        for previous in journal.previous_coins.into_iter().rev() {
            self.coins.insert(previous.id, previous);
        }
        for id in journal.issued_coin_ids {
            self.coins.remove(&id);
        }
        self.next_event = journal.previous_next_event;
        Ok(())
    }
}
