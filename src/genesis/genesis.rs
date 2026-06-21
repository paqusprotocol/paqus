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
    136, 97, 94, 85, 16, 248, 208, 180, 32, 217, 104, 221, 183, 10, 170, 28, 106, 70, 50, 148, 177,
    16, 11, 44, 94, 11, 243, 228, 136, 210, 83, 46, 9, 3, 37, 197, 169, 209, 148, 10, 54, 185, 223,
    193, 252, 240, 243, 194, 111, 97, 0, 18, 58, 160, 34, 84, 62, 72, 206, 103, 153, 24, 252, 253,
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
