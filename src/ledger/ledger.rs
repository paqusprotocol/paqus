use crate::block::Block;
use crate::block::BlockHeight;
use crate::consensus::supply::{Amount, Balance};
use crate::crypto::Address;
use crate::crypto::{BlockHash, HASH_SIZE, Hash, StateRoot};
use crate::error::LedgerError;
use crate::ledger::chain::Chain;
use crate::ledger::{AccountStateProof, calculate_state_root, create_account_state_proof};
use crate::state::Account;
use crate::transaction::SignedTransaction;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    pub accounts: BTreeMap<Address, Account>,
    pub chain: Chain,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
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
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    pub fn insert_account(&mut self, account: Account) -> Result<(), LedgerError> {
        if self.accounts.contains_key(&account.address) {
            return Err(LedgerError::AccountAlreadyExists);
        }

        let mut staged = self.clone();
        staged.accounts.insert(account.address, account);
        staged.validate_supply()?;
        *self = staged;
        Ok(())
    }

    pub fn account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }

    pub fn account_mut(&mut self, address: &Address) -> Option<&mut Account> {
        self.accounts.get_mut(address)
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

    pub fn validate_supply(&self) -> Result<(), LedgerError> {
        self.total_supply()?;
        Ok(())
    }

    pub fn apply_signed_transaction(
        &mut self,
        signed_transaction: &SignedTransaction,
    ) -> Result<(), LedgerError> {
        signed_transaction
            .validate_signed()
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

    pub fn apply_block(&mut self, mut block: Block) -> Result<(), LedgerError> {
        let (mut staged, expected_state_root) = self.staged_after_validated_block(&block)?;
        if block.state_root() == Hash([0; HASH_SIZE]) {
            block.set_state_root(expected_state_root);
        }

        staged.chain.insert_block(block)?;
        *self = staged;

        Ok(())
    }

    pub fn state_root_after_block(&self, block: &Block) -> Result<StateRoot, LedgerError> {
        Ok(self.staged_after_block(block)?.state_root())
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
        calculate_state_root(&self.accounts)
    }

    pub fn create_account_state_proof(&self, address: &Address) -> Option<AccountStateProof> {
        create_account_state_proof(&self.accounts, address)
    }
}
