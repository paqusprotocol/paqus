use crate::block::{Block, GenesisAllocation};
use crate::error::GenesisError;
use crate::ledger::{Ledger, calculate_state_root};
use crate::params::{CURRENT_CHAIN_PARAMS, ChainParams, GENESIS_PREMINE, HASH_SIZE};
use crate::state::Account;
use crate::types::{Address, Amount, Hash};
use std::collections::BTreeMap;

pub const GENESIS_PREMINE_ADDRESS: Address = Address(CURRENT_CHAIN_PARAMS.genesis.premine_address);
pub const GENESIS_MINER_ADDRESS: Address = Address(CURRENT_CHAIN_PARAMS.genesis.miner_address);
pub const GENESIS_TIMESTAMP: u64 = CURRENT_CHAIN_PARAMS.genesis.timestamp;
pub const GENESIS_HASH: [u8; HASH_SIZE] = CURRENT_CHAIN_PARAMS.genesis.hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisConfig {
    pub miner_address: Address,
    pub timestamp: u64,
}

pub fn genesis_premine_amount() -> Result<Amount, GenesisError> {
    Ok(Amount(GENESIS_PREMINE))
}

pub fn create_genesis_block(config: GenesisConfig) -> Block {
    create_genesis_block_for_chain(CURRENT_CHAIN_PARAMS, config)
}

pub fn create_genesis_block_for_chain(params: ChainParams, config: GenesisConfig) -> Block {
    let premine = genesis_premine_amount().expect("genesis premine amount should be valid");
    let mut block = Block::genesis(
        config.miner_address,
        config.timestamp,
        vec![GenesisAllocation::new(
            Address(params.genesis.premine_address),
            premine,
        )],
    );
    let mut accounts = BTreeMap::new();
    accounts.insert(
        Address(params.genesis.premine_address),
        Account::new(Address(params.genesis.premine_address), premine),
    );
    block.set_state_root(calculate_state_root(&accounts));
    block
}

pub fn create_genesis_ledger(config: GenesisConfig) -> Result<Ledger, GenesisError> {
    create_genesis_ledger_for_chain(CURRENT_CHAIN_PARAMS, config)
}

pub fn create_genesis_ledger_for_chain(
    params: ChainParams,
    config: GenesisConfig,
) -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(create_genesis_block_for_chain(params, config))?;

    Ok(ledger)
}

pub fn genesis_block() -> Block {
    genesis_block_for_chain(CURRENT_CHAIN_PARAMS)
}

pub fn genesis_block_for_chain(params: ChainParams) -> Block {
    create_genesis_block_for_chain(
        params,
        GenesisConfig {
            miner_address: Address(params.genesis.miner_address),
            timestamp: params.genesis.timestamp,
        },
    )
}

pub fn genesis_hash() -> Hash {
    Hash(GENESIS_HASH)
}

pub fn genesis_ledger() -> Result<Ledger, GenesisError> {
    genesis_ledger_for_chain(CURRENT_CHAIN_PARAMS)
}

pub fn genesis_ledger_for_chain(params: ChainParams) -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(genesis_block_for_chain(params))?;

    Ok(ledger)
}

pub fn create_default_genesis_ledger(
    miner_address: Address,
    timestamp: u64,
) -> Result<Ledger, GenesisError> {
    create_genesis_ledger(GenesisConfig {
        miner_address,
        timestamp,
    })
}
