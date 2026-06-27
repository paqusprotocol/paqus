use crate::block::{Block, GenesisAllocation};
use crate::error::GenesisError;
use crate::ledger::{Ledger, calculate_state_root};
use crate::params::{GENESIS_PREMINE, HASH_SIZE};
use crate::state::Account;
use crate::types::{Address, Amount, Hash};
use std::collections::BTreeMap;

pub const GENESIS_PREMINE_ADDRESS: Address = Address::ZERO;
pub const GENESIS_MINER_ADDRESS: Address = Address::ZERO;
pub const GENESIS_TIMESTAMP: u64 = 1_700_000_000;
pub const GENESIS_HASH: [u8; HASH_SIZE] = [
    71, 253, 122, 185, 102, 114, 162, 51, 213, 234, 18, 182, 210, 115, 174, 117, 124, 37, 39, 21,
    251, 188, 223, 112, 163, 237, 128, 206, 159, 168, 147, 171, 175, 22, 173, 53, 201, 145, 24, 37,
    126, 71, 8, 227, 103, 55, 17, 50, 150, 254, 1, 204, 96, 60, 148, 110, 14, 152, 34, 239, 22,
    224, 128, 63,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisConfig {
    pub miner_address: Address,
    pub timestamp: u64,
}

pub fn genesis_premine_amount() -> Result<Amount, GenesisError> {
    Ok(Amount(GENESIS_PREMINE))
}

pub fn create_genesis_block(config: GenesisConfig) -> Block {
    let premine = genesis_premine_amount().expect("genesis premine amount should be valid");
    let mut block = Block::genesis(
        config.miner_address,
        config.timestamp,
        vec![GenesisAllocation::new(GENESIS_PREMINE_ADDRESS, premine)],
    );
    let mut accounts = BTreeMap::new();
    accounts.insert(
        GENESIS_PREMINE_ADDRESS,
        Account::new(GENESIS_PREMINE_ADDRESS, premine),
    );
    block.set_state_root(calculate_state_root(&accounts));
    block
}

pub fn create_genesis_ledger(config: GenesisConfig) -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(create_genesis_block(config))?;

    Ok(ledger)
}

pub fn genesis_block() -> Block {
    create_genesis_block(GenesisConfig {
        miner_address: GENESIS_MINER_ADDRESS,
        timestamp: GENESIS_TIMESTAMP,
    })
}

pub fn genesis_hash() -> Hash {
    Hash(GENESIS_HASH)
}

pub fn genesis_ledger() -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(genesis_block())?;

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
