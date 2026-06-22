use crate::error::{LedgerError, StateError};
use crate::ledger::Ledger;
use crate::params::MAX_UNIT_SUPPLY;

pub fn validate_ledger_invariants(ledger: &Ledger) -> Result<(), LedgerError> {
    if ledger.total_supply()?.0 > MAX_UNIT_SUPPLY {
        return Err(LedgerError::SupplyOverflow);
    }

    for (address, account) in &ledger.accounts {
        if account.address != *address {
            return Err(LedgerError::InvalidState(StateError::AddressMismatch));
        }

        let credit_total = account
            .credits
            .iter()
            .try_fold(0_u32, |total, credit| total.checked_add(credit.amount.0))
            .ok_or(LedgerError::SupplyOverflow)?;
        if credit_total != account.balance.0 {
            return Err(LedgerError::InvalidState(StateError::BalanceOverflow));
        }
    }

    Ok(())
}
