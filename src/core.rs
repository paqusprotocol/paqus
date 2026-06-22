//! Pure blockchain logic.
//!
//! This module is intentionally limited to deterministic ledger rules and
//! primitives. Node orchestration, networking, mempool handling, mining loops,
//! wallets, and disk storage live in `runtime`.

pub use crate::block;
pub use crate::consensus;
pub use crate::crypto;
pub use crate::genesis;
pub use crate::ledger;
pub use crate::params;
pub use crate::state;
pub use crate::transaction;
pub use crate::types;

pub use crate::block::{
    Block, BlockError, BlockHeader, CoinbaseTransaction, GenesisAllocation, MinerRevenue,
};
pub use crate::codec::{
    HashDomain, block_bytes, block_header_bytes, block_header_hash, canonical_bytes,
    canonical_decode, decode_block, decode_signed_transaction, decode_transaction, domain_hash,
    hash_bytes, signed_transaction_bytes, signed_transaction_hash, state_root_bytes,
    transaction_bytes, transaction_hash,
};
pub use crate::consensus::{Consensus, ConsensusError, block_reward};
pub use crate::genesis::{
    GENESIS_HASH, GENESIS_MINER_ADDRESS, GENESIS_PREMINE_ADDRESS, GENESIS_TIMESTAMP, GenesisConfig,
    GenesisError, create_default_genesis_ledger, create_genesis_block, create_genesis_ledger,
    genesis_block, genesis_hash, genesis_ledger, genesis_premine_amount,
};
pub use crate::invariants::validate_ledger_invariants;
pub use crate::ledger::fork_choice::{BlockNode, ForkChoice, ForkChoiceError, block_work};
pub use crate::ledger::{
    AccountStateProof, BlockExecution, Chain, Ledger, LedgerError, ProofSide, ReorgPlan,
    StateProofNode, TransactionExecution, apply_transaction_to_state, calculate_state_root,
    common_ancestor, create_account_state_proof, plan_reorg,
    validate_signed_transaction_against_state, validate_transaction_against_state,
    verify_account_state_proof,
};
pub use crate::state::{Account, Credit, CreditSource, StateError};
pub use crate::transaction::{FeeRate, SignedTransaction, Transaction, TransactionError};
pub use crate::types::*;
pub use crate::version::{
    ProtocolVersions, active_versions, supported_block_version, supported_transaction_version,
};
