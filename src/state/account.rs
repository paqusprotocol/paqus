use crate::state::error::StateError;
use crate::transaction::Transaction;
use crate::types::{AccountNonce, Address, Amount, Balance, BlockHeight, Nonce};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash,
)]
pub struct Account {
    pub address: Address,
    pub balance: Balance,
    pub nonce: AccountNonce,
    pub credits: Vec<Credit>,
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash,
)]
pub struct Credit {
    pub amount: Amount,
    pub spendable_height: BlockHeight,
    pub source: CreditSource,
}

#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub enum CreditSource {
    Genesis,
    Transaction,
    Fee,
    MiningReward,
}

impl Account {
    pub fn new(address: Address, balance: Balance) -> Self {
        Self {
            address,
            balance,
            nonce: Nonce(0),
            credits: vec![Credit {
                amount: balance,
                spendable_height: crate::types::Height(0),
                source: CreditSource::Genesis,
            }],
        }
    }

    pub fn with_nonce(address: Address, balance: Balance, nonce: AccountNonce) -> Self {
        Self {
            address,
            balance,
            nonce,
            credits: vec![Credit {
                amount: balance,
                spendable_height: crate::types::Height(0),
                source: CreditSource::Genesis,
            }],
        }
    }

    pub fn available_balance_at(&self, height: BlockHeight) -> Amount {
        Amount(
            self.credits
                .iter()
                .filter(|credit| credit.spendable_height.0 <= height.0)
                .map(|credit| credit.amount.0)
                .sum(),
        )
    }

    pub fn unspendable_balance_at(&self, height: BlockHeight) -> Amount {
        Amount(
            self.balance
                .0
                .saturating_sub(self.available_balance_at(height).0),
        )
    }

    pub fn can_spend_at(&self, amount: Balance, height: BlockHeight) -> bool {
        self.available_balance_at(height).0 >= amount.0
    }

    pub fn credit(&mut self, amount: Balance) -> Result<(), StateError> {
        self.credit_locked(amount, crate::types::Height(0), CreditSource::Genesis)
    }

    pub fn credit_locked(
        &mut self,
        amount: Balance,
        spendable_height: BlockHeight,
        source: CreditSource,
    ) -> Result<(), StateError> {
        self.balance.0 = self
            .balance
            .0
            .checked_add(amount.0)
            .ok_or(StateError::BalanceOverflow)?;
        if amount.0 > 0 {
            self.credits.push(Credit {
                amount,
                spendable_height,
                source,
            });
        }
        self.compact_credits();
        Ok(())
    }

    pub fn debit(&mut self, amount: Balance) -> Result<(), StateError> {
        self.debit_at(amount, crate::types::Height(u64::MAX))
    }

    pub fn debit_at(&mut self, amount: Balance, height: BlockHeight) -> Result<(), StateError> {
        if !self.can_spend_at(amount, height) {
            return Err(StateError::InsufficientBalance);
        }

        self.balance.0 -= amount.0;
        let mut remaining = amount.0;
        for credit in &mut self.credits {
            if remaining == 0 {
                break;
            }
            if credit.spendable_height.0 > height.0 || credit.amount.0 == 0 {
                continue;
            }

            let spent = credit.amount.0.min(remaining);
            credit.amount.0 -= spent;
            remaining -= spent;
        }
        self.credits.retain(|credit| credit.amount.0 > 0);
        self.compact_credits();
        Ok(())
    }

    pub fn compact_credits(&mut self) {
        let mut compacted: BTreeMap<(BlockHeight, CreditSource), u32> = BTreeMap::new();
        for credit in &self.credits {
            if credit.amount.0 == 0 {
                continue;
            }

            let entry = compacted
                .entry((credit.spendable_height, credit.source))
                .or_insert(0);
            *entry = entry.saturating_add(credit.amount.0);
        }

        self.credits = compacted
            .into_iter()
            .map(|((spendable_height, source), amount)| Credit {
                amount: Amount(amount),
                spendable_height,
                source,
            })
            .collect();
    }

    pub fn increment_nonce(&mut self) {
        self.nonce.0 = self.nonce.0.saturating_add(1);
    }

    pub fn apply_outgoing_transaction(
        &mut self,
        transaction: &Transaction,
        height: BlockHeight,
    ) -> Result<(), StateError> {
        if transaction.from != self.address {
            return Err(StateError::AddressMismatch);
        }

        if transaction.nonce != self.nonce {
            return Err(StateError::InvalidNonce);
        }

        let total = transaction
            .amount
            .0
            .checked_add(transaction.fee.0)
            .ok_or(StateError::BalanceOverflow)?;

        self.debit_at(Amount(total), height)?;
        self.increment_nonce();
        Ok(())
    }

    pub fn apply_incoming_transaction(
        &mut self,
        transaction: &Transaction,
        spendable_height: BlockHeight,
    ) -> Result<(), StateError> {
        if transaction.to != self.address {
            return Err(StateError::AddressMismatch);
        }

        self.credit_locked(
            transaction.amount,
            spendable_height,
            CreditSource::Transaction,
        )
    }
}
