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

pub const CONFIRMATION_DEPTH: u32 = 10;
pub const BLOCK_REWARD_MATURITY: u32 = 120;
pub const FINALITY_DEPTH: u32 = 100;
pub const ECASH_WITHDRAW_MATURITY: u32 = FINALITY_DEPTH;
pub const ECASH_DEPOSIT_MATURITY: u32 = FINALITY_DEPTH;

pub use chain::Chain;
pub use invariants::validate_ledger_invariants;
pub use ledger::{EcashAccountJournal, Ledger};
pub use reorg::{ReorgPlan, common_ancestor, plan_reorg};
pub use state_proof::{
    AccountStateProof, ProofSide, SparseStateTree, StateProofNode, calculate_state_root,
    create_account_state_proof, verify_account_state_proof,
};
pub use transition::{
    BlockExecution, TransactionExecution, validate_signed_transaction_against_state,
    validate_transaction_against_state,
};
