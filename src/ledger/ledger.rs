use crate::block::Block;
use crate::block::BlockHeight;
use crate::consensus::supply::{Amount, Balance};
use crate::crypto::Address;
use crate::crypto::{BlockHash, HASH_SIZE, Hash, HashDomain, StateRoot, domain_hash};
use crate::error::LedgerError;
use crate::event::ProtocolEvent;
use crate::ledger::chain::Chain;
use crate::ledger::{AccountStateProof, SparseStateTree};
use crate::state::{Account, CreditSource, QCashUtxoSet};
use crate::transaction::{
    QCashTransaction, QCashTransactionKind, SignedQCashTransaction, SignedTransaction,
};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    pub(crate) accounts: BTreeMap<Address, Account>,
    account_state_tree: Arc<SparseStateTree>,
    pub chain: Chain,
    pub qcash_utxos: QCashUtxoSet,
    pub qcash_account_journals: BTreeMap<BlockHash, QCashAccountJournal>,
    rollback_states: BTreeMap<BlockHash, AccountRollbackState>,
    /// Derived receipts keyed by their canonical block. Not part of the protocol state root.
    pub events_by_block: BTreeMap<BlockHash, Vec<ProtocolEvent>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QCashAccountJournal {
    pub block_hash: BlockHash,
    pub block_height: BlockHeight,
    /// `None` means the account did not exist before this block.
    pub previous_accounts: BTreeMap<Address, Option<Account>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AccountRollbackState {
    accounts: BTreeMap<Address, Account>,
    account_state_tree: Arc<SparseStateTree>,
}

pub struct AccountMut<'a> {
    account: &'a mut Account,
    state_tree: &'a mut Arc<SparseStateTree>,
}

impl Deref for AccountMut<'_> {
    type Target = Account;

    fn deref(&self) -> &Self::Target {
        self.account
    }
}

impl DerefMut for AccountMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.account
    }
}

impl Drop for AccountMut<'_> {
    fn drop(&mut self) {
        Arc::make_mut(self.state_tree).update_account(self.account);
    }
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_accounts_and_chain(accounts: BTreeMap<Address, Account>, chain: Chain) -> Self {
        Self {
            account_state_tree: Arc::new(SparseStateTree::from_accounts(&accounts)),
            accounts,
            chain,
            ..Self::default()
        }
    }

    pub fn accounts(&self) -> &BTreeMap<Address, Account> {
        &self.accounts
    }

    pub fn replace_accounts(&mut self, accounts: BTreeMap<Address, Account>) {
        self.account_state_tree = Arc::new(SparseStateTree::from_accounts(&accounts));
        self.accounts = accounts;
    }

    pub(crate) fn refresh_account_state(&mut self, address: &Address) {
        if let Some(account) = self.accounts.get(address) {
            Arc::make_mut(&mut self.account_state_tree).update_account(account);
        } else {
            Arc::make_mut(&mut self.account_state_tree).remove_account(address);
        }
    }

    fn refresh_qcash_accounts(&mut self, transaction: &QCashTransaction) {
        self.refresh_account_state(&transaction.signer);
        if let QCashTransactionKind::DepositCash { recipient, .. } = &transaction.kind {
            self.refresh_account_state(recipient);
        }
    }

    pub fn create_account(
        &mut self,
        address: Address,
        balance: Balance,
    ) -> Result<(), LedgerError> {
        if self.accounts.contains_key(&address) {
            return Err(LedgerError::AccountAlreadyExists);
        }

        let mut staged = self.clone();
        staged
            .accounts
            .insert(address, Account::new(address, balance));
        staged.refresh_account_state(&address);
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    pub fn insert_account(&mut self, account: Account) -> Result<(), LedgerError> {
        if self.accounts.contains_key(&account.address) {
            return Err(LedgerError::AccountAlreadyExists);
        }

        let mut staged = self.clone();
        let address = account.address;
        staged.accounts.insert(address, account);
        staged.refresh_account_state(&address);
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    pub fn account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }

    pub fn account_mut(&mut self, address: &Address) -> Option<AccountMut<'_>> {
        let account = self.accounts.get_mut(address)?;
        Some(AccountMut {
            account,
            state_tree: &mut self.account_state_tree,
        })
    }

    pub fn balance(&self, address: &Address) -> Option<Balance> {
        self.account(address).map(|account| account.balance)
    }

    pub fn confirmed_balance(&self, address: &Address) -> Option<Balance> {
        self.balance(address)
    }

    pub fn total_supply(&self) -> Result<Amount, LedgerError> {
        let mut total = 0_u64;
        for account in self.accounts.values() {
            total = total
                .checked_add(account.balance.0)
                .ok_or(LedgerError::SupplyOverflow)?;
        }
        Ok(Amount(total))
    }

    /// Account balances plus issued, unredeemed bearer cash.
    pub fn economic_supply(&self) -> Result<Amount, LedgerError> {
        let accounts = self.total_supply()?;
        let cash = self.qcash_utxos.total_value()?;
        accounts
            .0
            .checked_add(cash.0)
            .map(Amount)
            .ok_or(LedgerError::SupplyOverflow)
    }

    pub fn validate_supply(&self) -> Result<(), LedgerError> {
        self.economic_supply()?;
        Ok(())
    }

    pub fn apply_signed_qcash_transaction(
        &mut self,
        signed: &SignedQCashTransaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        signed
            .validate_signed_for_height(height)
            .map_err(LedgerError::from)?;
        let mut staged = self.clone();
        staged.apply_qcash_transaction(&signed.transaction, height, None)?;
        staged.refresh_qcash_accounts(&signed.transaction);
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    pub fn apply_signed_qcash_transaction_in_block(
        &mut self,
        signed: &SignedQCashTransaction,
        height: BlockHeight,
        block_hash: BlockHash,
    ) -> Result<(), LedgerError> {
        signed
            .validate_signed_for_height(height)
            .map_err(LedgerError::from)?;
        let mut staged = self.clone();
        staged.capture_qcash_accounts(block_hash, height, &signed.transaction)?;
        staged.apply_qcash_transaction(&signed.transaction, height, Some(block_hash))?;
        staged.refresh_qcash_accounts(&signed.transaction);
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    fn capture_qcash_accounts(
        &mut self,
        block_hash: BlockHash,
        height: BlockHeight,
        transaction: &QCashTransaction,
    ) -> Result<(), LedgerError> {
        let mut addresses = vec![transaction.signer];
        if let QCashTransactionKind::DepositCash { recipient, .. } = &transaction.kind {
            addresses.push(*recipient);
        }
        let journal = self
            .qcash_account_journals
            .entry(block_hash)
            .or_insert_with(|| QCashAccountJournal {
                block_hash,
                block_height: height,
                previous_accounts: BTreeMap::new(),
            });
        if journal.block_height != height {
            return Err(LedgerError::MissingQCashAccountJournal);
        }
        for address in addresses {
            journal
                .previous_accounts
                .entry(address)
                .or_insert_with(|| self.accounts.get(&address).cloned());
        }
        Ok(())
    }

    /// Atomically restores account and coin state for a disconnected QCash block.
    pub fn rollback_qcash_block(&mut self, block_hash: BlockHash) -> Result<(), LedgerError> {
        let mut staged = self.clone();
        let journal = staged
            .qcash_account_journals
            .remove(&block_hash)
            .ok_or(LedgerError::MissingQCashAccountJournal)?;
        staged.qcash_utxos.rollback_block(block_hash)?;
        for (address, previous) in journal.previous_accounts {
            match previous {
                Some(account) => {
                    staged.accounts.insert(address, account);
                }
                None => {
                    staged.accounts.remove(&address);
                }
            }
            staged.refresh_account_state(&address);
        }
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    pub fn finalize_qcash_at(&mut self, tip_height: BlockHeight) {
        self.qcash_utxos.finalize_at(tip_height);
    }

    /// Disconnects the active tip and restores its complete rollback state.
    pub fn rollback_block(&mut self, block_hash: BlockHash) -> Result<(), LedgerError> {
        let mut staged = self.clone();
        staged.chain.remove_tip(block_hash)?;
        let rollback_state = staged
            .rollback_states
            .remove(&block_hash)
            .ok_or(LedgerError::MissingQCashAccountJournal)?;
        if staged.qcash_utxos.journal(block_hash).is_some() {
            staged.qcash_utxos.rollback_block(block_hash)?;
        }
        staged.qcash_account_journals.remove(&block_hash);
        staged.accounts = rollback_state.accounts;
        staged.account_state_tree = rollback_state.account_state_tree;
        staged.events_by_block.remove(&block_hash);
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    fn apply_qcash_transaction(
        &mut self,
        transaction: &QCashTransaction,
        height: BlockHeight,
        block_hash: Option<BlockHash>,
    ) -> Result<(), LedgerError> {
        let signer = self
            .accounts
            .get(&transaction.signer)
            .ok_or(LedgerError::AccountNotFound)?;
        if signer.nonce != transaction.nonce {
            return Err(LedgerError::NonceMismatch);
        }

        match &transaction.kind {
            QCashTransactionKind::WithdrawCash { amount, metadata } => {
                let debit = amount
                    .0
                    .checked_add(transaction.fee.0)
                    .map(Amount)
                    .ok_or(LedgerError::SupplyOverflow)?;
                let account = self
                    .accounts
                    .get_mut(&transaction.signer)
                    .ok_or(LedgerError::AccountNotFound)?;
                account.debit_at(debit, height)?;
                account.increment_nonce();
                if let Some(block_hash) = block_hash {
                    self.qcash_utxos.apply_withdraw_in_block(
                        block_hash,
                        height,
                        transaction.signer,
                        transaction.hash(),
                        metadata,
                    )?;
                } else {
                    self.qcash_utxos.apply_withdraw(
                        transaction.signer,
                        transaction.hash(),
                        metadata,
                        height,
                    )?;
                }
            }
            QCashTransactionKind::DepositCash {
                recipient,
                metadata,
            } => {
                let amount = if let Some(block_hash) = block_hash {
                    self.qcash_utxos
                        .apply_deposit_in_block(block_hash, height, metadata, *recipient)?
                } else {
                    self.qcash_utxos
                        .apply_deposit_proof(metadata, *recipient, height)?
                };
                let credited = Amount(amount.0 - transaction.fee.0);
                self.accounts
                    .get_mut(&transaction.signer)
                    .ok_or(LedgerError::AccountNotFound)?
                    .increment_nonce();
                let spendable_height = crate::block::Height(
                    height
                        .0
                        .saturating_add(crate::ledger::QCASH_DEPOSIT_MATURITY as u64),
                );
                self.accounts
                    .entry(*recipient)
                    .or_insert_with(|| Account::new(*recipient, Amount(0)))
                    .credit_locked(credited, spendable_height, CreditSource::QCashDeposit)?;
            }
        }
        Ok(())
    }

    pub fn apply_signed_transaction(
        &mut self,
        signed_transaction: &SignedTransaction,
    ) -> Result<(), LedgerError> {
        signed_transaction
            .validate_signed_for_height(crate::block::Height(0))
            .map_err(LedgerError::from)?;
        self.apply_transaction_at(&signed_transaction.transaction, crate::block::Height(0))
    }

    #[cfg(test)]
    pub(crate) fn apply_transaction(
        &mut self,
        transaction: &crate::transaction::Transaction,
    ) -> Result<(), LedgerError> {
        self.apply_transaction_at(transaction, crate::block::Height(0))
    }

    pub fn apply_block(&mut self, block: Block) -> Result<(), LedgerError> {
        let (mut staged, _) = self.staged_after_validated_block(&block)?;
        if !block.is_genesis() && block.state_root() == Hash([0; HASH_SIZE]) {
            return Err(LedgerError::InvalidStateRoot);
        }

        let block_hash = block.hash();
        staged.rollback_states.insert(
            block_hash,
            AccountRollbackState {
                accounts: self.accounts.clone(),
                account_state_tree: self.account_state_tree.clone(),
            },
        );
        staged.record_protocol_events(&block);
        staged.chain.insert_block(block)?;
        *self = staged;

        Ok(())
    }

    pub fn state_root_after_block(&self, block: &Block) -> Result<StateRoot, LedgerError> {
        self.staged_after_validated_block(block)
            .map(|(_, state_root)| state_root)
    }

    pub fn block(&self, height: &BlockHeight) -> Option<&Block> {
        self.chain.block(height)
    }

    pub fn has_blocks(&self) -> bool {
        self.chain.has_blocks()
    }

    pub fn tip_height(&self) -> Option<BlockHeight> {
        self.chain.tip_height()
    }

    pub fn tip_hash(&self) -> Option<BlockHash> {
        self.chain.tip_hash()
    }

    pub fn state_root(&self) -> StateRoot {
        self.account_state_tree.root()
    }

    /// Root committing accounts and all protocol extension state.
    pub fn protocol_state_root(&self) -> StateRoot {
        calculate_protocol_state_root(self.state_root(), &self.qcash_utxos)
    }

    pub fn create_account_state_proof(&self, address: &Address) -> Option<AccountStateProof> {
        self.accounts
            .get(address)
            .map(|account| self.account_state_tree.create_account_proof(account))
    }
}

/// Commits the account tree and QCash UTXO set into one protocol state root.
pub fn calculate_protocol_state_root(
    account_state_root: StateRoot,
    qcash_utxos: &QCashUtxoSet,
) -> StateRoot {
    let qcash_root = qcash_utxos.consensus_root();
    StateRoot(
        domain_hash(
            HashDomain::ProtocolState,
            &crate::codec::canonical_bytes(&(account_state_root, qcash_root)),
        )
        .0,
    )
}
