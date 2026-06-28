#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainParams {
    pub chain_name: &'static str,
    pub chain_id: u16,
    pub coin_name: &'static str,
    pub unit_name: &'static str,
    pub protocol_stage: &'static str,
    pub protocol_version: u8,
    pub network_magic: [u8; 4],
    pub block_version: u8,
    pub transaction_version: u8,
    pub block_version_activation_height: u64,
    pub transaction_version_activation_height: u64,
    pub genesis: GenesisParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisParams {
    pub premine_address: [u8; ADDRESS_SIZE],
    pub miner_address: [u8; ADDRESS_SIZE],
    pub timestamp: u64,
    pub hash: [u8; HASH_SIZE],
}

pub const UNIT: u32 = 1;
pub const XPQ: u32 = 100;
pub const DECIMALS: u8 = 2;

pub const MAX_UNIT_SUPPLY: u32 = u32::MAX;
pub const GENESIS_PREMINE: u32 = 95; // 0.95 XPQ
pub const MAX_XPQ_SUPPLY: u32 = MAX_UNIT_SUPPLY / XPQ;
pub const MAX_MINED_SUPPLY: u32 = MAX_UNIT_SUPPLY - GENESIS_PREMINE;

pub const SECOND: u32 = 1;
pub const MINUTE: u32 = 60 * SECOND;
pub const HOUR: u32 = 60 * MINUTE;
pub const DAY: u32 = 24 * HOUR;
pub const BLOCK_TIME: u32 = 5 * MINUTE;
pub const BLOCKS_PER_DAY: u64 = DAY as u64 / BLOCK_TIME as u64;
pub const BLOCKS_PER_YEAR: u64 = 365 * BLOCKS_PER_DAY;
pub const TAIL_EMISSION_START_YEARS: u64 = 4;
pub const TAIL_EMISSION_START_HEIGHT: u64 = TAIL_EMISSION_START_YEARS * BLOCKS_PER_YEAR;

pub const BLOCK_REWARD: u32 = 5_000; // 50 XPQ
pub const TAIL_EMISSION: u32 = 100; // 1 XPQ
pub const CONFIRMATION_DEPTH: u32 = 10;
pub const BLOCK_REWARD_MATURITY: u32 = 120;
pub const FINALITY_DEPTH: u32 = 100;

pub const MIN_DIFFICULTY: u32 = 1;
pub const MAX_DIFFICULTY: u32 = (PROOF_OF_WORK_HASH_SIZE * 8) as u32;
pub const DIFFICULTY_START: u32 = 1;
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;
pub const DIFFICULTY_TIMESPAN_CLAMP_FACTOR: u64 = 16;
pub const MAX_DIFFICULTY_ADJUSTMENT_BITS: u32 = 4;
pub const MAX_FUTURE_TIME: u32 = 15 * SECOND;
pub const MAX_TRANSACTION_AGE: u32 = DAY;
pub const MAX_TRANSACTION_FUTURE_TIME: u32 = BLOCK_TIME;

pub const CHECKPOINT_INTERVAL: u64 = 10_000;
pub const CHECKPOINT_MIN_CONFIRMATIONS: u32 = FINALITY_DEPTH;
pub const SNAPSHOT_INTERVAL: u64 = 50_000;
pub const SNAPSHOT_MIN_CONFIRMATIONS: u32 = FINALITY_DEPTH;
pub const SNAPSHOT_ROOT_DOMAIN: &[u8] = b"PAQUS_SNAPSHOT_ROOT_V1";

pub const ADDRESS_SIZE: usize = 20;
pub const PUBLIC_KEY_SIZE: usize = 2_592;
pub const SECRET_KEY_SIZE: usize = 4_896;
pub const SIGNATURE_SIZE: usize = 4_627;
pub const HASH_SIZE: usize = 64;
pub const PROOF_OF_WORK_HASH_SIZE: usize = 32;

pub const MAX_TX_SIZE: usize = 10 * 1024;
pub const MAX_BLOCK_SIZE: usize = 3 * 1024 * 1024;
pub const MAX_BLOCK_TXS: usize = 350;

pub const MAINNET: ChainParams = ChainParams {
    chain_name: "Paqus",
    chain_id: 747,
    coin_name: "XPQ",
    unit_name: "paqus",
    protocol_stage: "Mainnet",
    protocol_version: 3,
    network_magic: [0x58, 0x50, 0x51, 0x01],
    block_version: 1,
    transaction_version: 2,
    block_version_activation_height: 0,
    transaction_version_activation_height: 0,
    genesis: GenesisParams {
        // The premine intentionally goes to the zero address. It is not an owner allocation; it
        // reserves the initial supply offset so mined supply accounting lands on the intended cap.
        premine_address: [0; ADDRESS_SIZE],
        miner_address: [0; ADDRESS_SIZE],
        // Fixed timestamp of the first canonical genesis build. This must stay static so all nodes
        // derive the same genesis hash.
        timestamp: 1_700_000_000,
        hash: [
            71, 253, 122, 185, 102, 114, 162, 51, 213, 234, 18, 182, 210, 115, 174, 117, 124, 37,
            39, 21, 251, 188, 223, 112, 163, 237, 128, 206, 159, 168, 147, 171, 175, 22, 173, 53,
            201, 145, 24, 37, 126, 71, 8, 227, 103, 55, 17, 50, 150, 254, 1, 204, 96, 60, 148, 110,
            14, 152, 34, 239, 22, 224, 128, 63,
        ],
    },
};

pub const TESTNET: ChainParams = ChainParams {
    chain_name: "Paqus",
    chain_id: 74,
    coin_name: "tXPQ",
    unit_name: "paqus",
    protocol_stage: "Testnet",
    protocol_version: 3,
    network_magic: [0x58, 0x50, 0x51, 0x03],
    block_version: 1,
    transaction_version: 2,
    block_version_activation_height: 0,
    transaction_version_activation_height: 0,
    genesis: MAINNET.genesis,
};

pub const DEVNET: ChainParams = ChainParams {
    chain_name: "Paqus",
    chain_id: 7,
    coin_name: "dXPQ",
    unit_name: "paqus",
    protocol_stage: "Devnet",
    protocol_version: 3,
    network_magic: [0x58, 0x50, 0x51, 0x07],
    block_version: 1,
    transaction_version: 2,
    block_version_activation_height: 0,
    transaction_version_activation_height: 0,
    genesis: MAINNET.genesis,
};

pub const CURRENT_CHAIN_PARAMS: ChainParams = MAINNET;
pub const CHAIN_NAME: &str = CURRENT_CHAIN_PARAMS.chain_name;
pub const CHAIN_ID: u16 = CURRENT_CHAIN_PARAMS.chain_id;
pub const COIN_NAME: &str = CURRENT_CHAIN_PARAMS.coin_name;
pub const UNIT_NAME: &str = CURRENT_CHAIN_PARAMS.unit_name;
pub const PROTOCOL_STAGE: &str = CURRENT_CHAIN_PARAMS.protocol_stage;
pub const PROTOCOL_VERSION: u8 = CURRENT_CHAIN_PARAMS.protocol_version;
pub const NETWORK_MAGIC: [u8; 4] = CURRENT_CHAIN_PARAMS.network_magic;
pub const BLOCK_VERSION: u8 = CURRENT_CHAIN_PARAMS.block_version;
pub const TRANSACTION_VERSION: u8 = CURRENT_CHAIN_PARAMS.transaction_version;
pub const BLOCK_VERSION_ACTIVATION_HEIGHT: u64 =
    CURRENT_CHAIN_PARAMS.block_version_activation_height;
pub const TRANSACTION_VERSION_ACTIVATION_HEIGHT: u64 =
    CURRENT_CHAIN_PARAMS.transaction_version_activation_height;
