use crate::codec::{HashDomain, block_bytes, block_header_hash, canonical_bytes, domain_hash};
use crate::consensus::block_reward;
use crate::error::BlockError;
use crate::params::{DIFFICULTY_START, HASH_SIZE, MAX_BLOCK_SIZE, MAX_BLOCK_TXS, MAX_FUTURE_TIME};
use crate::transaction::SignedTransaction;
use crate::types::{
    Address, Amount, BlockHash, BlockHeight, BlockNonce, Hash, Height, MerkleHash, Nonce,
    PreviousHash, StateRoot,
};
use crate::version::{active_versions, supported_block_version};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockHeader {
    pub version: u8,
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
            version: active_versions(height).block,
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
        block_header_hash(self)
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
        domain_hash(HashDomain::GenesisAllocation, &canonical_bytes(self))
    }
}

impl CoinbaseTransaction {
    pub fn new(to: Address, subsidy: Amount, fees: Amount) -> Self {
        Self { to, subsidy, fees }
    }

    pub fn total(&self) -> Amount {
        Amount(self.subsidy.0.saturating_add(self.fees.0))
    }

    pub fn checked_total(&self) -> Result<Amount, BlockError> {
        Ok(Amount(
            self.subsidy
                .0
                .checked_add(self.fees.0)
                .ok_or(BlockError::CoinbaseOverflow)?,
        ))
    }

    pub fn hash(&self) -> Hash {
        domain_hash(HashDomain::Coinbase, &canonical_bytes(self))
    }
}

impl Block {
    pub fn new(
        height: BlockHeight,
        previous_hash: impl Into<PreviousHash>,
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
        previous_hash: impl Into<PreviousHash>,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
        transactions: Vec<SignedTransaction>,
    ) -> Self {
        let previous_hash = previous_hash.into();
        let coinbase = if height.0 == 0 && previous_hash == Hash([0; HASH_SIZE]) {
            None
        } else {
            let fees = Amount(
                transactions
                    .iter()
                    .try_fold(0_u32, |total, transaction| {
                        total.checked_add(transaction.payload.fee.0)
                    })
                    .unwrap_or(u32::MAX),
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
            PreviousHash::ZERO,
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
        previous_hash: impl Into<PreviousHash>,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
        coinbase: Option<CoinbaseTransaction>,
        transactions: Vec<SignedTransaction>,
    ) -> Self {
        Self::with_parts(
            height,
            previous_hash.into(),
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
        previous_hash: impl Into<PreviousHash>,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
        genesis_allocations: Vec<GenesisAllocation>,
        coinbase: Option<CoinbaseTransaction>,
        transactions: Vec<SignedTransaction>,
    ) -> Self {
        let previous_hash = previous_hash.into();
        let merkle_root =
            calculate_merkle_root(&genesis_allocations, coinbase.as_ref(), &transactions);
        let state_root = StateRoot::ZERO;
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
        if !supported_block_version(self.height(), self.header.version) {
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

        self.checked_total_fees()?;
        if let Some(coinbase) = &self.coinbase {
            coinbase.checked_total()?;
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

    pub fn set_state_root(&mut self, state_root: impl Into<StateRoot>) {
        self.header.state_root = state_root.into();
    }

    pub fn difficulty(&self) -> u32 {
        self.header.difficulty
    }

    pub fn timestamp(&self) -> u64 {
        self.header.timestamp
    }

    pub fn total_fees(&self) -> Amount {
        self.checked_total_fees().unwrap_or(Amount(u32::MAX))
    }

    pub fn checked_total_fees(&self) -> Result<Amount, BlockError> {
        let fees = self
            .transactions
            .iter()
            .try_fold(0_u32, |total, transaction| {
                total.checked_add(transaction.payload.fee.0)
            })
            .ok_or(BlockError::FeeOverflow)?;
        Ok(Amount(fees))
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
        self.to_bytes().len()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        block_bytes(self)
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
        return MerkleHash::ZERO;
    }

    let mut hashes: Vec<Hash> = genesis_allocations
        .iter()
        .map(GenesisAllocation::hash)
        .chain(coinbase.into_iter().map(CoinbaseTransaction::hash))
        .chain(
            transactions
                .iter()
                .map(|transaction| transaction.hash().as_hash()),
        )
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
                domain_hash(HashDomain::MerkleNode, &bytes)
            })
            .collect();
    }

    MerkleHash(hashes[0].0)
}
