use crate::block::error::BlockError;
use crate::consensus::block_reward;
use crate::params::{
    BLOCK_VERSION, DIFFICULTY_START, HASH_SIZE, MAX_BLOCK_SIZE, MAX_BLOCK_TXS, MAX_FUTURE_TIME,
};
use crate::transaction::SignedTransaction;
use crate::types::{
    Address, Amount, BlockHash, BlockHeight, BlockNonce, Hash, Height, MerkleHash, Nonce,
    PreviousHash, StateRoot,
};
use borsh::{BorshDeserialize, BorshSerialize};
use sha3::{Digest, Sha3_512};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockHeader {
    pub version: u16,
    pub height: BlockHeight,
    pub previous_hash: PreviousHash,
    pub merkle_root: MerkleHash,
    pub state_root: StateRoot,
    pub miner_address: Address,
    pub difficulty: u32,
    pub timestamp: u64,
    pub nonce: BlockNonce,
}

impl BlockHeader {
    pub fn new(
        height: BlockHeight,
        previous_hash: PreviousHash,
        merkle_root: MerkleHash,
        state_root: StateRoot,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
    ) -> Self {
        Self {
            version: BLOCK_VERSION,
            height,
            previous_hash,
            merkle_root,
            state_root,
            miner_address,
            difficulty,
            timestamp,
            nonce,
        }
    }

    pub fn hash(&self) -> BlockHash {
        let bytes = borsh::to_vec(self).expect("block header serialization should not fail");
        hash_bytes(&bytes)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub header: BlockHeader,
    pub genesis_allocations: Vec<GenesisAllocation>,
    pub coinbase: Option<CoinbaseTransaction>,
    pub transactions: Vec<SignedTransaction>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenesisAllocation {
    pub to: Address,
    pub amount: Amount,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CoinbaseTransaction {
    pub to: Address,
    pub subsidy: Amount,
    pub fees: Amount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinerRevenue {
    pub subsidy: Amount,
    pub fees: Amount,
}

impl GenesisAllocation {
    pub fn new(to: Address, amount: Amount) -> Self {
        Self { to, amount }
    }

    pub fn hash(&self) -> Hash {
        hash_bytes(&borsh::to_vec(self).expect("genesis allocation serialization should not fail"))
    }
}

impl CoinbaseTransaction {
    pub fn new(to: Address, subsidy: Amount, fees: Amount) -> Self {
        Self { to, subsidy, fees }
    }

    pub fn total(&self) -> Amount {
        Amount(self.subsidy.0.saturating_add(self.fees.0))
    }

    pub fn hash(&self) -> Hash {
        hash_bytes(&borsh::to_vec(self).expect("coinbase serialization should not fail"))
    }
}

impl Block {
    pub fn new(
        height: BlockHeight,
        previous_hash: PreviousHash,
        miner_address: Address,
        timestamp: u64,
        nonce: BlockNonce,
        transactions: Vec<SignedTransaction>,
    ) -> Self {
        Self::with_difficulty(
            height,
            previous_hash,
            miner_address,
            DIFFICULTY_START,
            timestamp,
            nonce,
            transactions,
        )
    }

    pub fn with_difficulty(
        height: BlockHeight,
        previous_hash: PreviousHash,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
        transactions: Vec<SignedTransaction>,
    ) -> Self {
        let coinbase = if height.0 == 0 && previous_hash == Hash([0; HASH_SIZE]) {
            None
        } else {
            let fees = Amount(
                transactions
                    .iter()
                    .map(|transaction| transaction.payload.fee.0)
                    .sum(),
            );
            Some(CoinbaseTransaction::new(
                miner_address,
                block_reward(height),
                fees,
            ))
        };
        Self::with_parts(
            height,
            previous_hash,
            miner_address,
            difficulty,
            timestamp,
            nonce,
            vec![],
            coinbase,
            transactions,
        )
    }

    pub fn genesis(
        miner_address: Address,
        timestamp: u64,
        allocations: Vec<GenesisAllocation>,
    ) -> Self {
        Self::with_parts(
            Height(0),
            Hash([0; HASH_SIZE]),
            miner_address,
            DIFFICULTY_START,
            timestamp,
            Nonce(0),
            allocations,
            None,
            vec![],
        )
    }

    pub fn with_coinbase(
        height: BlockHeight,
        previous_hash: PreviousHash,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
        coinbase: Option<CoinbaseTransaction>,
        transactions: Vec<SignedTransaction>,
    ) -> Self {
        Self::with_parts(
            height,
            previous_hash,
            miner_address,
            difficulty,
            timestamp,
            nonce,
            vec![],
            coinbase,
            transactions,
        )
    }

    pub fn with_parts(
        height: BlockHeight,
        previous_hash: PreviousHash,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
        genesis_allocations: Vec<GenesisAllocation>,
        coinbase: Option<CoinbaseTransaction>,
        transactions: Vec<SignedTransaction>,
    ) -> Self {
        let merkle_root =
            calculate_merkle_root(&genesis_allocations, coinbase.as_ref(), &transactions);
        let state_root = Hash([0; HASH_SIZE]);
        Self {
            header: BlockHeader::new(
                height,
                previous_hash,
                merkle_root,
                state_root,
                miner_address,
                difficulty,
                timestamp,
                nonce,
            ),
            genesis_allocations,
            coinbase,
            transactions,
        }
    }

    pub fn validate(&self) -> Result<(), BlockError> {
        self.validate_at(self.header.timestamp)
    }

    pub fn validate_at(&self, now: u64) -> Result<(), BlockError> {
        if self.header.version != BLOCK_VERSION {
            return Err(BlockError::UnsupportedVersion);
        }

        if self.is_genesis() {
            if self.coinbase.is_some() {
                return Err(BlockError::UnexpectedCoinbase);
            }
            if !self.transactions.is_empty() {
                return Err(BlockError::InvalidTransaction);
            }
        } else if self.coinbase.is_none() {
            return Err(BlockError::MissingCoinbase);
        } else if !self.genesis_allocations.is_empty() {
            return Err(BlockError::UnexpectedGenesisAllocation);
        }

        if self.transactions.len() > MAX_BLOCK_TXS {
            return Err(BlockError::TooManyTransactions);
        }

        if self.serialized_size() > MAX_BLOCK_SIZE {
            return Err(BlockError::BlockTooLarge);
        }

        if self.header.timestamp > now.saturating_add(MAX_FUTURE_TIME as u64) {
            return Err(BlockError::FutureTimestamp);
        }

        if self.transactions.iter().any(|tx| tx.validate().is_err()) {
            return Err(BlockError::InvalidTransaction);
        }

        if let Some(coinbase) = &self.coinbase {
            if coinbase.to != self.header.miner_address {
                return Err(BlockError::InvalidCoinbase);
            }
        }

        if self
            .genesis_allocations
            .iter()
            .any(|allocation| allocation.amount.0 == 0)
        {
            return Err(BlockError::InvalidGenesisAllocation);
        }

        if self.header.merkle_root
            != calculate_merkle_root(
                &self.genesis_allocations,
                self.coinbase.as_ref(),
                &self.transactions,
            )
        {
            return Err(BlockError::InvalidMerkleRoot);
        }

        Ok(())
    }

    pub fn hash(&self) -> BlockHash {
        self.header.hash()
    }

    pub fn height(&self) -> BlockHeight {
        self.header.height
    }

    pub fn previous_hash(&self) -> PreviousHash {
        self.header.previous_hash
    }

    pub fn miner_address(&self) -> Address {
        self.header.miner_address
    }

    pub fn state_root(&self) -> StateRoot {
        self.header.state_root
    }

    pub fn set_state_root(&mut self, state_root: StateRoot) {
        self.header.state_root = state_root;
    }

    pub fn difficulty(&self) -> u32 {
        self.header.difficulty
    }

    pub fn timestamp(&self) -> u64 {
        self.header.timestamp
    }

    pub fn total_fees(&self) -> Amount {
        Amount(
            self.transactions
                .iter()
                .map(|transaction| transaction.payload.fee.0)
                .sum(),
        )
    }

    pub fn miner_revenue(&self, subsidy: Amount) -> MinerRevenue {
        MinerRevenue {
            subsidy,
            fees: self.total_fees(),
        }
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_genesis(&self) -> bool {
        self.header.height.0 == 0 && self.header.previous_hash == Hash([0; HASH_SIZE])
    }

    pub fn serialized_size(&self) -> usize {
        borsh::to_vec(self)
            .expect("block serialization should not fail")
            .len()
    }

    pub fn calculate_merkle_root(&self) -> MerkleHash {
        calculate_merkle_root(
            &self.genesis_allocations,
            self.coinbase.as_ref(),
            &self.transactions,
        )
    }

    pub fn refresh_merkle_root(&mut self) {
        self.header.merkle_root = self.calculate_merkle_root();
    }

    pub fn push_transaction(&mut self, transaction: SignedTransaction) {
        self.transactions.push(transaction);
        self.refresh_merkle_root();
    }
}

fn calculate_merkle_root(
    genesis_allocations: &[GenesisAllocation],
    coinbase: Option<&CoinbaseTransaction>,
    transactions: &[SignedTransaction],
) -> MerkleHash {
    if genesis_allocations.is_empty() && coinbase.is_none() && transactions.is_empty() {
        return Hash([0; HASH_SIZE]);
    }

    let mut hashes: Vec<Hash> = genesis_allocations
        .iter()
        .map(GenesisAllocation::hash)
        .chain(coinbase.into_iter().map(CoinbaseTransaction::hash))
        .chain(transactions.iter().map(SignedTransaction::hash))
        .collect();

    while hashes.len() > 1 {
        if hashes.len() % 2 == 1 {
            let last = *hashes.last().expect("hashes is not empty");
            hashes.push(last);
        }

        hashes = hashes
            .chunks(2)
            .map(|pair| {
                let mut bytes = Vec::with_capacity(HASH_SIZE * 2);
                bytes.extend_from_slice(&pair[0].0);
                bytes.extend_from_slice(&pair[1].0);
                hash_bytes(&bytes)
            })
            .collect();
    }

    hashes[0]
}

fn hash_bytes(bytes: &[u8]) -> Hash {
    let digest = Sha3_512::digest(bytes);
    let mut hash = [0_u8; HASH_SIZE];
    hash.copy_from_slice(&digest);
    Hash(hash)
}
