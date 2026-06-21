use crate::block::{Block, GenesisAllocation};
use crate::genesis::error::GenesisError;
use crate::ledger::{Ledger, calculate_state_root};
use crate::params::{GENESIS_PREMINE, HASH_SIZE};
use crate::state::Account;
use crate::types::{Address, Amount, Hash};
use std::collections::BTreeMap;

pub const GENESIS_PREMINE_ADDRESS: Address = Address::ZERO;
pub const GENESIS_MINER_ADDRESS: Address = Address::ZERO;
pub const GENESIS_TIMESTAMP: u64 = 1_700_000_000;
pub const GENESIS_HASH: [u8; HASH_SIZE] = [
    245, 9, 234, 13, 174, 102, 145, 162, 254, 58, 251, 194, 6, 185, 240, 11, 186, 168, 135, 84, 55,
    122, 9, 205, 81, 137, 170, 222, 73, 108, 89, 217, 18, 186, 42, 33, 121, 59, 106, 210, 29, 202,
    128, 19, 29, 135, 193, 237, 181, 66, 52, 67, 210, 92, 136, 97, 126, 48, 138, 238, 171, 142,
    240, 248,
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
