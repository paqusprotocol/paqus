use crate::block::Block;
use crate::ledger::{Ledger, LedgerError};
use crate::mempool::error::MempoolError;
use crate::params::{HASH_SIZE, MAX_MEMPOOL_TXS};
use crate::state::StateError;
use crate::transaction::SignedTransaction;
use crate::types::{Address, BlockNonce, Hash, Height, TransactionHash};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mempool {
    transactions: BTreeMap<TransactionHash, SignedTransaction>,
    config: MempoolConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MempoolConfig {
    pub max_transactions: usize,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_transactions: MAX_MEMPOOL_TXS,
        }
    }
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: MempoolConfig) -> Self {
        Self {
            transactions: BTreeMap::new(),
            config,
        }
    }

    pub fn config(&self) -> MempoolConfig {
        self.config
    }

    pub fn insert(
        &mut self,
        transaction: SignedTransaction,
    ) -> Result<TransactionHash, MempoolError> {
        transaction.validate_signed()?;
        self.insert_unchecked(transaction)
    }

    pub fn insert_validated(
        &mut self,
        ledger: &Ledger,
        transaction: SignedTransaction,
    ) -> Result<TransactionHash, MempoolError> {
        transaction.validate_signed()?;
        self.validate_against_ledger(ledger, &transaction)?;
        self.insert_unchecked(transaction)
    }

    fn insert_unchecked(
        &mut self,
        transaction: SignedTransaction,
    ) -> Result<TransactionHash, MempoolError> {
        let hash = transaction.hash();
        if self.transactions.contains_key(&hash) {
            return Err(MempoolError::DuplicateTransaction);
        }

        if self.transactions.len() >= self.config.max_transactions {
            return Err(MempoolError::MempoolFull);
        }

        self.transactions.insert(hash, transaction);
        Ok(hash)
    }

    pub fn validate_against_ledger(
        &self,
        ledger: &Ledger,
        transaction: &SignedTransaction,
    ) -> Result<(), MempoolError> {
        transaction.validate_signed()?;

        let payload = &transaction.payload;
        let sender = ledger
            .account(&payload.from)
            .ok_or(LedgerError::AccountNotFound)?;
        ledger
            .account(&payload.to)
            .ok_or(LedgerError::AccountNotFound)?;

        let current_height = ledger.tip_height().unwrap_or(Height(0));
        let mut expected_nonce = sender.nonce;
        let mut spendable = sender.available_balance_at(current_height);
        let mut pending_from_sender: Vec<_> = self
            .transactions
            .values()
            .filter(|pending| pending.payload.from == payload.from)
            .collect();
        pending_from_sender.sort_by_key(|pending| pending.payload.nonce);

        for pending in pending_from_sender {
            if pending.payload.nonce != expected_nonce {
                return Err(LedgerError::InvalidState(StateError::InvalidNonce).into());
            }

            let total = pending
                .payload
                .amount
                .0
                .checked_add(pending.payload.fee.0)
                .ok_or(LedgerError::InvalidState(StateError::BalanceOverflow))?;
            if spendable.0 < total {
                return Err(LedgerError::InvalidState(StateError::InsufficientBalance).into());
            }

            spendable.0 -= total;
            expected_nonce.0 = expected_nonce.0.saturating_add(1);
        }

        if payload.nonce != expected_nonce {
            return Err(LedgerError::InvalidState(StateError::InvalidNonce).into());
        }

        let total = payload
            .amount
            .0
            .checked_add(payload.fee.0)
            .ok_or(LedgerError::InvalidState(StateError::BalanceOverflow))?;
        if spendable.0 < total {
            return Err(LedgerError::InvalidState(StateError::InsufficientBalance).into());
        }

        Ok(())
    }

    pub fn remove(&mut self, hash: &TransactionHash) -> Option<SignedTransaction> {
        self.transactions.remove(hash)
    }

    pub fn get(&self, hash: &TransactionHash) -> Option<&SignedTransaction> {
        self.transactions.get(hash)
    }

    pub fn transactions(&self) -> impl Iterator<Item = &SignedTransaction> {
        self.transactions.values()
    }

    pub fn contains(&self, hash: &TransactionHash) -> bool {
        self.transactions.contains_key(hash)
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn clear(&mut self) {
        self.transactions.clear();
    }

    pub fn select_for_block(&self, limit: usize) -> Vec<SignedTransaction> {
        let mut by_sender: BTreeMap<Address, Vec<SignedTransaction>> = BTreeMap::new();
        for transaction in self.transactions.values() {
            by_sender
                .entry(transaction.payload.from)
                .or_default()
                .push(transaction.clone());
        }
        for transactions in by_sender.values_mut() {
            transactions.sort_by_key(|transaction| (transaction.payload.nonce, transaction.hash()));
        }

        let mut selected = Vec::new();
        while selected.len() < limit {
            let Some(sender) = by_sender
                .iter()
                .filter_map(|(sender, transactions)| {
                    transactions.first().map(|transaction| {
                        (
                            *sender,
                            transaction.payload.fee.0,
                            transaction.payload.nonce,
                            transaction.hash(),
                        )
                    })
                })
                .max_by(|left, right| {
                    left.1
                        .cmp(&right.1)
                        .then_with(|| right.2.cmp(&left.2))
                        .then_with(|| right.3.cmp(&left.3))
                })
                .map(|candidate| candidate.0)
            else {
                break;
            };

            let transactions = by_sender
                .get_mut(&sender)
                .expect("selected sender should exist");
            selected.push(transactions.remove(0));
            if transactions.is_empty() {
                by_sender.remove(&sender);
            }
        }

        selected
    }

    pub fn create_candidate_block(
        &self,
        ledger: &Ledger,
        miner_address: Address,
        timestamp: u64,
        nonce: BlockNonce,
        transaction_limit: usize,
    ) -> Result<Block, LedgerError> {
        let height = ledger
            .tip_height()
            .map(|height| Height(height.0.saturating_add(1)))
            .unwrap_or(Height(0));
        let previous_hash = ledger.tip_hash().unwrap_or(Hash([0; HASH_SIZE]));

        let mut block = Block::new(
            height,
            previous_hash,
            miner_address,
            timestamp,
            nonce,
            self.select_for_block(transaction_limit),
        );
        let state_root = ledger.state_root_after_block(&block)?;
        block.set_state_root(state_root);
        Ok(block)
    }

    pub fn remove_confirmed(&mut self, block: &Block) {
        for transaction in &block.transactions {
            self.transactions.remove(&transaction.hash());
        }
    }
}
