use crate::block::Block;
use crate::crypto::Address;
use crate::crypto::{HASH_SIZE, Hash};
use crate::error::GenesisError;
use crate::ledger::Ledger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainParams {
    pub chain_name: &'static str,
    pub chain_id: u16,
    pub coin_name: &'static str,
    pub unit_name: &'static str,
    pub protocol_stage: &'static str,
    pub protocol_version: u8,
    pub network_magic: [u8; 4],
    pub genesis: GenesisParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisParams {
    pub miner_address: [u8; crate::crypto::ADDRESS_SIZE],
    pub timestamp: u64,
    pub hash: [u8; HASH_SIZE],
}

pub const PAQUS_CHAIN: ChainParams = ChainParams {
    chain_name: "Paqus",
    chain_id: 747,
    coin_name: "XPQ",
    unit_name: "paqus",
    protocol_stage: "Mainnet",
    protocol_version: 6,
    network_magic: [0x58, 0x50, 0x51, 0x01],
    genesis: GenesisParams {
        miner_address: [0; crate::crypto::ADDRESS_SIZE],
        // Fixed timestamp of the first canonical genesis build. This must stay static so all nodes
        // derive the same genesis hash.
        timestamp: 1_700_000_000,
        hash: [
            136, 139, 17, 129, 8, 26, 171, 19, 145, 131, 139, 214, 47, 242, 153, 105, 162, 128,
            109, 202, 96, 14, 6, 104, 207, 21, 185, 219, 175, 179, 173, 32, 31, 209, 21, 117, 106,
            108, 126, 166, 58, 22, 88, 97, 142, 47, 113, 38, 219, 14, 147, 99, 71, 173, 56, 223,
            169, 87, 107, 203, 76, 195, 6, 178,
        ],
    },
};

pub const CURRENT_CHAIN_PARAMS: ChainParams = PAQUS_CHAIN;

pub const GENESIS_MINER_ADDRESS: Address = Address(CURRENT_CHAIN_PARAMS.genesis.miner_address);
pub const GENESIS_TIMESTAMP: u64 = CURRENT_CHAIN_PARAMS.genesis.timestamp;
pub const GENESIS_HASH: [u8; HASH_SIZE] = CURRENT_CHAIN_PARAMS.genesis.hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisConfig {
    pub miner_address: Address,
    pub timestamp: u64,
}

pub fn create_genesis_block(config: GenesisConfig) -> Block {
    create_genesis_block_for_chain(CURRENT_CHAIN_PARAMS, config)
}

pub fn create_genesis_block_for_chain(_params: ChainParams, config: GenesisConfig) -> Block {
    Block::genesis(config.miner_address, config.timestamp, vec![])
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
