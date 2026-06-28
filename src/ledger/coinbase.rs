use crate::block::Block;
use crate::consensus::block_reward;
use crate::ledger::{Ledger, LedgerError};
use crate::params::{BLOCK_REWARD_MATURITY, CONFIRMATION_DEPTH, GENESIS_PREMINE, MAX_MINED_SUPPLY};
use crate::state::{Account, CreditSource};
use crate::types::{Address, Amount, BlockHeight};

impl Ledger {
    pub(crate) fn apply_coinbase(&mut self, block: &Block) -> Result<(), LedgerError> {
        let coinbase = block
            .coinbase
            .as_ref()
            .ok_or(LedgerError::InvalidCoinbase)?;
        let expected_fees = block.checked_total_fees().map_err(LedgerError::from)?;
        if coinbase.to != block.miner_address() || coinbase.fees != expected_fees {
            return Err(LedgerError::InvalidCoinbase);
        }

        self.credit_miner_fees(coinbase.to, coinbase.fees, block.height())?;
        let expected_subsidy = self.expected_mintable_subsidy(block_reward(block.height()))?;
        if coinbase.subsidy != expected_subsidy {
            return Err(LedgerError::InvalidCoinbase);
        }
        self.mint_miner_subsidy(coinbase.to, coinbase.subsidy, block.height())
    }

    fn credit_miner_fees(
        &mut self,
        miner_address: Address,
        fees: Amount,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        let spendable_height =
            crate::types::Height(height.0.saturating_add(CONFIRMATION_DEPTH as u64));
        self.credit_miner(miner_address, fees, spendable_height, CreditSource::Fee)
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

    fn expected_mintable_subsidy(&self, scheduled_subsidy: Amount) -> Result<Amount, LedgerError> {
        Ok(Amount(
            scheduled_subsidy.0.min(self.remaining_mined_supply()?),
        ))
    }

    pub fn mintable_subsidy(&self, height: BlockHeight) -> Result<Amount, LedgerError> {
        self.expected_mintable_subsidy(block_reward(height))
    }

    pub fn remaining_mined_supply(&self) -> Result<u32, LedgerError> {
        let total = self.total_supply()?.0;
        let mined_supply = total.saturating_sub(GENESIS_PREMINE);
        Ok(MAX_MINED_SUPPLY.saturating_sub(mined_supply))
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
