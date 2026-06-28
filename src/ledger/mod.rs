#![allow(clippy::module_inception)]

pub mod chain;
pub mod coinbase;
pub mod fork_choice;
pub mod invariants;
pub mod ledger;
pub mod reorg;
pub mod state_proof;
pub mod transition;

pub use crate::error::LedgerError;
pub use chain::Chain;
pub use invariants::validate_ledger_invariants;
pub use ledger::Ledger;
pub use reorg::{ReorgPlan, common_ancestor, plan_reorg};
pub use state_proof::{
    AccountStateProof, ProofSide, StateProofNode, calculate_state_root, create_account_state_proof,
    verify_account_state_proof,
};
pub use transition::{
    BlockExecution, TransactionExecution, apply_transaction_to_state,
    validate_signed_transaction_against_state, validate_transaction_against_state,
};
