use crate::block::Block;
use crate::block::BlockHeight;
use crate::consensus::supply::Amount;
use crate::crypto::Address;
use crate::crypto::{BlockHash, StateRoot, TransactionHash};
use crate::ledger::{CONFIRMATION_DEPTH, Ledger, LedgerError, SparseStateTree};
use crate::state::Account;
use crate::transaction::{SignedTransaction, Transaction};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransactionExecution {
    pub transaction_hash: TransactionHash,
    pub from: crate::crypto::Address,
    pub to: crate::crypto::Address,
    pub amount: Amount,
    pub fee: Amount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockExecution {
    pub block_hash: BlockHash,
    pub height: BlockHeight,
    pub state_root_before: StateRoot,
    pub state_root_after: StateRoot,
    pub transactions: Vec<TransactionExecution>,
}

impl TransactionExecution {
    pub fn from_signed(transaction: &SignedTransaction) -> Self {
        Self::from_payload(transaction.hash(), &transaction.transaction)
    }

    pub fn from_payload(transaction_hash: TransactionHash, transaction: &Transaction) -> Self {
        Self {
            transaction_hash,
            from: transaction.from,
            to: transaction.to,
            amount: transaction.amount,
            fee: transaction.fee,
        }
    }
}

pub(crate) fn apply_transaction_to_state(
    accounts: &mut BTreeMap<Address, Account>,
    transaction: &Transaction,
    height: BlockHeight,
) -> Result<(), LedgerError> {
    if !accounts.contains_key(&transaction.from) {
        return Err(LedgerError::AccountNotFound);
    }

    {
        let sender = accounts
            .get_mut(&transaction.from)
            .ok_or(LedgerError::AccountNotFound)?;
        sender.apply_outgoing_transaction(transaction, height)?;
    }

    let spendable_height = crate::block::Height(height.0.saturating_add(CONFIRMATION_DEPTH as u64));
    let receiver = accounts
        .entry(transaction.to)
        .or_insert_with(|| Account::new(transaction.to, Amount(0)));
    receiver.apply_incoming_transaction(transaction, spendable_height)?;

    Ok(())
}

pub fn validate_transaction_against_state(
    accounts: &BTreeMap<Address, Account>,
    transaction: &Transaction,
    height: BlockHeight,
) -> Result<(), LedgerError> {
    let mut staged = accounts.clone();
    apply_transaction_to_state(&mut staged, transaction, height)
}

pub fn validate_signed_transaction_against_state(
    accounts: &BTreeMap<Address, Account>,
    transaction: &SignedTransaction,
    height: BlockHeight,
) -> Result<(), LedgerError> {
    transaction
        .validate_signed_for_height(height)
        .map_err(LedgerError::from)?;
    validate_transaction_against_state(accounts, &transaction.transaction, height)
}

impl Ledger {
    pub(crate) fn apply_transaction_at(
        &mut self,
        transaction: &Transaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        apply_transaction_to_state(&mut self.accounts, transaction, height)
    }

    pub(crate) fn staged_after_block(&self, block: &Block) -> Result<Self, LedgerError> {
        block.validate()?;
        self.staged_after_valid_signed_block(block)
    }

    pub(crate) fn staged_after_valid_signed_block(
        &self,
        block: &Block,
    ) -> Result<Self, LedgerError> {
        let mut staged = self.clone();
        for transaction in &block.transactions {
            staged.apply_transaction_at(&transaction.transaction, block.height())?;
        }

        if block.is_genesis() {
            for allocation in &block.genesis_allocations {
                staged.create_account(allocation.to, allocation.amount)?;
            }
            return Ok(staged);
        }

        staged.apply_coinbase(block)?;
        Ok(staged)
    }

    pub fn validate_transaction_against_state(
        &self,
        transaction: &Transaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        validate_transaction_against_state(&self.accounts, transaction, height)
    }

    pub fn validate_signed_transaction_against_state(
        &self,
        transaction: &SignedTransaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        validate_signed_transaction_against_state(&self.accounts, transaction, height)
    }

    pub fn validate_block(&self, block: &Block) -> Result<StateRoot, LedgerError> {
        self.staged_after_validated_block(block)
            .map(|(_, expected_state_root)| expected_state_root)
    }

    pub fn execute_block(&self, block: &Block) -> Result<(Ledger, BlockExecution), LedgerError> {
        let state_root_before = self.state_root();
        let (mut staged, expected_state_root) = self.staged_after_validated_block(block)?;
        let mut committed_block = block.clone();
        if committed_block.state_root() == crate::crypto::Hash([0; crate::crypto::HASH_SIZE]) {
            committed_block.set_state_root(expected_state_root);
        }
        let block_hash = committed_block.hash();
        staged.chain.insert_block(committed_block)?;

        let execution = BlockExecution {
            block_hash,
            height: block.height(),
            state_root_before,
            state_root_after: expected_state_root,
            transactions: block
                .transactions
                .iter()
                .map(TransactionExecution::from_signed)
                .collect(),
        };

        Ok((staged, execution))
    }

    pub(crate) fn staged_after_validated_block(
        &self,
        block: &Block,
    ) -> Result<(Self, StateRoot), LedgerError> {
        block.validate()?;
        self.chain.validate_next_block(block)?;

        let mut staged = self.clone();
        let mut state_tree = SparseStateTree::from_accounts(&staged.accounts);

        for transaction in &block.transactions {
            staged.apply_transaction_at(&transaction.transaction, block.height())?;
            if let Some(sender) = staged.accounts.get(&transaction.transaction.from) {
                state_tree.update_account(sender);
            }
            if let Some(receiver) = staged.accounts.get(&transaction.transaction.to) {
                state_tree.update_account(receiver);
            }
        }

        if block.is_genesis() {
            for allocation in &block.genesis_allocations {
                staged.create_account(allocation.to, allocation.amount)?;
                if let Some(account) = staged.accounts.get(&allocation.to) {
                    state_tree.update_account(account);
                }
            }
        } else {
            staged.apply_coinbase(block)?;
            if let Some(miner) = staged.accounts.get(&block.miner_address()) {
                state_tree.update_account(miner);
            }
        }

        let expected_state_root = state_tree.root();
        if !block.is_genesis() && block.state_root() != expected_state_root {
            return Err(LedgerError::InvalidStateRoot);
        }

        staged.validate_supply()?;
        Ok((staged, expected_state_root))
    }
}
