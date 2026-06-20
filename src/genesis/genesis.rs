use crate::block::{Block, GenesisAllocation};
use crate::genesis::error::GenesisError;
use crate::ledger::Ledger;
use crate::params::GENESIS_PREMINE;
use crate::types::{Address, Amount};

pub const GENESIS_PREMINE_ADDRESS: Address = Address::ZERO;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisConfig {
    pub premine_address: Address,
    pub miner_address: Address,
    pub timestamp: u64,
}

pub fn genesis_premine_amount() -> Result<Amount, GenesisError> {
    Ok(Amount(GENESIS_PREMINE))
}

pub fn create_genesis_block(config: GenesisConfig) -> Block {
    Block::genesis(
        config.miner_address,
        config.timestamp,
        vec![GenesisAllocation::new(
            config.premine_address,
            genesis_premine_amount().expect("genesis premine amount should be valid"),
        )],
    )
}

pub fn create_genesis_ledger(config: GenesisConfig) -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(create_genesis_block(config))?;

    Ok(ledger)
}

pub fn create_default_genesis_ledger(
    miner_address: Address,
    timestamp: u64,
) -> Result<Ledger, GenesisError> {
    create_genesis_ledger(GenesisConfig {
        premine_address: GENESIS_PREMINE_ADDRESS,
        miner_address,
        timestamp,
    })
}
