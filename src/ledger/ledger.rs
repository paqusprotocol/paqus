use crate::block::Block;
use crate::consensus::block_reward;
use crate::ledger::chain::Chain;
use crate::ledger::error::LedgerError;
use crate::ledger::{AccountStateProof, calculate_state_root, create_account_state_proof};
use crate::params::{BLOCK_REWARD_MATURITY, FINALITY_DEPTH, HASH_SIZE, MAX_UNIT_SUPPLY};
use crate::state::{Account, CreditSource};
use crate::transaction::{SignedTransaction, Transaction};
use crate::types::{Address, Amount, Balance, BlockHash, BlockHeight};
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
        let mut total = 0_u32;
        for account in self.accounts.values() {
            total = total
                .checked_add(account.balance.0)
                .ok_or(LedgerError::SupplyOverflow)?;
        }
        Ok(Amount(total))
    }

    pub fn validate_supply(&self) -> Result<(), LedgerError> {
        self.total_supply().map(|_| ())
    }

    pub fn apply_signed_transaction(
        &mut self,
        signed_transaction: &SignedTransaction,
    ) -> Result<(), LedgerError> {
        signed_transaction
            .validate_signed()
            .map_err(|_| LedgerError::InvalidBlock(crate::block::BlockError::InvalidTransaction))?;
        self.apply_transaction_at(&signed_transaction.payload, crate::types::Height(u64::MAX))
    }

    pub fn apply_transaction(&mut self, transaction: &Transaction) -> Result<(), LedgerError> {
        self.apply_transaction_at(transaction, crate::types::Height(u64::MAX))
    }

    fn apply_transaction_at(
        &mut self,
        transaction: &Transaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        if !self.accounts.contains_key(&transaction.from)
            || !self.accounts.contains_key(&transaction.to)
        {
            return Err(LedgerError::AccountNotFound);
        }

        {
            let sender = self
                .accounts
                .get_mut(&transaction.from)
                .ok_or(LedgerError::AccountNotFound)?;
            sender.apply_outgoing_transaction(transaction, height)?;
        }

        let spendable_height = crate::types::Height(height.0.saturating_add(FINALITY_DEPTH as u64));
        let receiver = self
            .accounts
            .get_mut(&transaction.to)
            .ok_or(LedgerError::AccountNotFound)?;
        receiver.apply_incoming_transaction(transaction, spendable_height)?;

        Ok(())
    }

    pub fn apply_block(&mut self, mut block: Block) -> Result<(), LedgerError> {
        block.validate()?;
        self.chain.validate_next_block(&block)?;

        let mut staged = self.staged_after_block(&block)?;
        let expected_state_root = staged.state_root();
        if block.state_root() == crate::types::Hash([0; HASH_SIZE]) {
            block.set_state_root(expected_state_root);
        } else if block.state_root() != expected_state_root {
            return Err(LedgerError::InvalidBlock(
                crate::block::BlockError::InvalidStateRoot,
            ));
        }
        staged.validate_supply()?;
        staged.chain.insert_block(block)?;
        *self = staged;

        Ok(())
    }

    pub fn state_root_after_block(
        &self,
        block: &Block,
    ) -> Result<crate::types::StateRoot, LedgerError> {
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

    pub fn state_root(&self) -> crate::types::StateRoot {
        calculate_state_root(&self.accounts)
    }

    pub fn create_account_state_proof(&self, address: &Address) -> Option<AccountStateProof> {
        create_account_state_proof(&self.accounts, address)
    }

    fn staged_after_block(&self, block: &Block) -> Result<Self, LedgerError> {
        let mut staged = self.clone();
        for transaction in &block.transactions {
            transaction.validate_signed().map_err(|_| {
                LedgerError::InvalidBlock(crate::block::BlockError::InvalidTransaction)
            })?;
            staged.apply_transaction_at(&transaction.payload, block.height())?;
        }

        if block.is_genesis() {
            for allocation in &block.genesis_allocations {
                staged.create_account(allocation.to, allocation.amount)?;
            }
            return Ok(staged);
        }

        let coinbase = block.coinbase.as_ref().ok_or(LedgerError::InvalidBlock(
            crate::block::BlockError::MissingCoinbase,
        ))?;
        let expected_fees = block.total_fees();
        let max_subsidy = block_reward(block.height());
        if coinbase.to != block.miner_address()
            || coinbase.fees != expected_fees
            || coinbase.subsidy.0 > max_subsidy.0
        {
            return Err(LedgerError::InvalidBlock(
                crate::block::BlockError::InvalidCoinbase,
            ));
        }

        staged.credit_miner_fees(coinbase.to, coinbase.fees, block.height())?;
        let subsidy = staged.mintable_subsidy(coinbase.subsidy)?;
        staged.mint_miner_subsidy(coinbase.to, subsidy, block.height())?;
        Ok(staged)
    }

    fn credit_miner_fees(
        &mut self,
        miner_address: Address,
        fees: Amount,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        self.credit_miner(miner_address, fees, height, CreditSource::Fee)
    }

    fn mint_miner_subsidy(
        &mut self,
        miner_address: Address,
        subsidy: Amount,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        let spendable_height =
            crate::types::Height(height.0.saturating_add(BLOCK_REWARD_MATURITY as u64));
        self.credit_miner(
            miner_address,
            subsidy,
            spendable_height,
            CreditSource::MiningReward,
        )
    }

    fn mintable_subsidy(&self, scheduled_subsidy: Amount) -> Result<Amount, LedgerError> {
        let total = self.total_supply()?.0;
        let remaining = MAX_UNIT_SUPPLY.saturating_sub(total);
        Ok(Amount(scheduled_subsidy.0.min(remaining)))
    }

    fn credit_miner(
        &mut self,
        miner_address: Address,
        amount: Amount,
        spendable_height: BlockHeight,
        source: CreditSource,
    ) -> Result<(), LedgerError> {
        if let Some(miner) = self.accounts.get_mut(&miner_address) {
            miner.credit_locked(amount, spendable_height, source)?;
        } else {
            let mut account = Account::new(miner_address, Amount(0));
            account.credit_locked(amount, spendable_height, source)?;
            self.accounts.insert(miner_address, account);
        }

        Ok(())
    }
}
