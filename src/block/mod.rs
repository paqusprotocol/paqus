use crate::codec::{HashDomain, block_bytes, block_header_hash, canonical_bytes, domain_hash};
use crate::consensus::supply::Amount;
use crate::consensus::{DIFFICULTY_START, MAX_FUTURE_TIME, block_reward};
use crate::crypto::{Address, PublicKey, Signature};
use crate::crypto::{
    BlockHash, HASH_SIZE, Hash, MerkleHash, PreviousHash, StateRoot, WitnessMerkleHash,
};
pub use crate::error::BlockError;
use crate::transaction::{
    QCashTransaction, SignedQCashTransaction, SignedTransaction, Transaction, Witness,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::io::{Error as IoError, ErrorKind, Read, Write};
use std::thread;

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct Height(pub u64);

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct Nonce(pub u64);

pub type BlockHeight = Height;
pub type BlockNonce = Nonce;

pub const MAX_BLOCK_SIZE: usize = 5 * 1024 * 1024;
pub const MAX_BLOCK_TXS: usize = 500;
pub const BLOCK_VERSION: u8 = 1;
pub const WITNESS_SCALE_FACTOR: usize = 4;
pub const MAX_BLOCK_WEIGHT: usize = MAX_BLOCK_SIZE * WITNESS_SCALE_FACTOR;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockHeader {
    pub version: u8,
    pub height: BlockHeight,
    pub previous_hash: PreviousHash,
    pub merkle_root: MerkleHash,
    pub witness_root: WitnessMerkleHash,
    pub state_root: StateRoot,
    pub miner_address: Address,
    pub difficulty: u32,
    pub timestamp: u64,
    pub nonce: BlockNonce,
}

impl BlockHeader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        height: BlockHeight,
        previous_hash: PreviousHash,
        merkle_root: MerkleHash,
        witness_root: WitnessMerkleHash,
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
            witness_root,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub header: BlockHeader,
    pub genesis_allocations: Vec<GenesisAllocation>,
    pub coinbase: Option<CoinbaseTransaction>,
    pub transactions: Vec<SignedTransaction>,
    pub qcash_transactions: Vec<SignedQCashTransaction>,
}

/// Consensus encoding keeps every payload section before every witness section.
impl BorshSerialize for Block {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        serialize_stripped_block(self, writer)?;
        let keys = witness_dictionary(self);
        keys.serialize(writer)?;
        serialize_indexed_witnesses(&self.transactions, &keys, writer, |tx| &tx.witness)?;
        serialize_indexed_witnesses(&self.qcash_transactions, &keys, writer, |tx| &tx.witness)
    }
}

pub(crate) fn serialize_stripped_block<W: Write>(
    block: &Block,
    writer: &mut W,
) -> std::io::Result<()> {
    block.header.serialize(writer)?;
    block.genesis_allocations.serialize(writer)?;
    block.coinbase.serialize(writer)?;
    serialize_projection(&block.transactions, writer, |tx| &tx.transaction)?;
    serialize_projection(&block.qcash_transactions, writer, |tx| &tx.transaction)
}

impl BorshDeserialize for Block {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let header = BlockHeader::deserialize_reader(reader)?;
        let genesis_allocations = Vec::<GenesisAllocation>::deserialize_reader(reader)?;
        let coinbase = Option::<CoinbaseTransaction>::deserialize_reader(reader)?;

        let transactions = Vec::<Transaction>::deserialize_reader(reader)?;
        let qcash_transactions = Vec::<QCashTransaction>::deserialize_reader(reader)?;

        let keys = Vec::<PublicKey>::deserialize_reader(reader)?;
        if keys
            .iter()
            .enumerate()
            .any(|(index, key)| keys[..index].contains(key))
        {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "duplicate witness dictionary key",
            ));
        }
        let transaction_witnesses = decode_indexed_witnesses(reader, &keys)?;
        let qcash_witnesses = decode_indexed_witnesses(reader, &keys)?;

        let block = Self {
            header,
            genesis_allocations,
            coinbase,
            transactions: zip_witnesses(
                transactions,
                transaction_witnesses,
                |transaction, witness| SignedTransaction {
                    transaction,
                    witness,
                },
            )?,
            qcash_transactions: zip_witnesses(
                qcash_transactions,
                qcash_witnesses,
                |transaction, witness| SignedQCashTransaction {
                    transaction,
                    witness,
                },
            )?,
        };
        if witness_dictionary(&block) != keys {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "non-canonical witness dictionary",
            ));
        }
        Ok(block)
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
struct IndexedWitness {
    key_index: u32,
    signature: Signature,
}

fn witness_dictionary(block: &Block) -> Vec<PublicKey> {
    let mut keys = Vec::new();
    for key in block
        .transactions
        .iter()
        .map(|tx| tx.witness.public_key)
        .chain(
            block
                .qcash_transactions
                .iter()
                .map(|tx| tx.witness.public_key),
        )
    {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

fn serialize_indexed_witnesses<T, W, F>(
    values: &[T],
    keys: &[PublicKey],
    writer: &mut W,
    project: F,
) -> std::io::Result<()>
where
    W: Write,
    F: Fn(&T) -> &Witness,
{
    let indexed = values
        .iter()
        .map(|value| {
            let witness = project(value);
            let key_index = keys
                .iter()
                .position(|key| key == &witness.public_key)
                .ok_or_else(|| IoError::new(ErrorKind::InvalidData, "missing witness key"))?;
            Ok(IndexedWitness {
                key_index: u32::try_from(key_index)
                    .map_err(|_| IoError::new(ErrorKind::InvalidData, "too many witness keys"))?,
                signature: witness.signature,
            })
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    indexed.serialize(writer)
}

fn decode_indexed_witnesses<R: Read>(
    reader: &mut R,
    keys: &[PublicKey],
) -> std::io::Result<Vec<Witness>> {
    Vec::<IndexedWitness>::deserialize_reader(reader)?
        .into_iter()
        .map(|indexed| {
            let public_key = keys
                .get(indexed.key_index as usize)
                .copied()
                .ok_or_else(|| {
                    IoError::new(ErrorKind::InvalidData, "witness key index out of range")
                })?;
            Ok(Witness::new(public_key, indexed.signature))
        })
        .collect()
}

fn serialize_projection<T, U, W, F>(values: &[T], writer: &mut W, project: F) -> std::io::Result<()>
where
    U: BorshSerialize,
    W: Write,
    F: Fn(&T) -> &U,
{
    let length = u32::try_from(values.len())
        .map_err(|_| IoError::new(ErrorKind::InvalidInput, "too many block section items"))?;
    BorshSerialize::serialize(&length, writer)?;
    for value in values {
        project(value).serialize(writer)?;
    }
    Ok(())
}

fn zip_witnesses<T, W, S, F>(
    transactions: Vec<T>,
    witnesses: Vec<W>,
    combine: F,
) -> std::io::Result<Vec<S>>
where
    F: Fn(T, W) -> S,
{
    if transactions.len() != witnesses.len() {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "transaction and witness section lengths differ",
        ));
    }
    Ok(transactions
        .into_iter()
        .zip(witnesses)
        .map(|(transaction, witness)| combine(transaction, witness))
        .collect())
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
                    .try_fold(0_u64, |total, transaction| {
                        total.checked_add(transaction.transaction.fee.0)
                    })
                    .unwrap_or(u64::MAX),
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
            vec![],
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
            vec![],
        )
    }

    #[allow(clippy::too_many_arguments)]
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
            vec![],
        )
    }

    #[allow(clippy::too_many_arguments)]
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
        qcash_transactions: Vec<SignedQCashTransaction>,
    ) -> Self {
        let previous_hash = previous_hash.into();
        let merkle_root = calculate_merkle_root(
            &genesis_allocations,
            coinbase.as_ref(),
            &transactions,
            &qcash_transactions,
        );
        let witness_root = calculate_witness_merkle_root(&transactions, &qcash_transactions);
        let state_root = StateRoot::ZERO;
        Self {
            header: BlockHeader::new(
                height,
                previous_hash,
                merkle_root,
                witness_root,
                state_root,
                miner_address,
                difficulty,
                timestamp,
                nonce,
            ),
            genesis_allocations,
            coinbase,
            transactions,
            qcash_transactions,
        }
    }

    /// Constructs a non-genesis SegWit block containing every transaction family.
    #[allow(clippy::too_many_arguments)]
    pub fn with_all_transactions(
        height: BlockHeight,
        previous_hash: impl Into<PreviousHash>,
        miner_address: Address,
        difficulty: u32,
        timestamp: u64,
        nonce: BlockNonce,
        transactions: Vec<SignedTransaction>,
        qcash_transactions: Vec<SignedQCashTransaction>,
    ) -> Result<Self, BlockError> {
        if height.0 == 0 {
            return Err(BlockError::InvalidTransaction);
        }
        let fees = checked_fees(&transactions, &qcash_transactions)?;
        Ok(Self::with_parts(
            height,
            previous_hash,
            miner_address,
            difficulty,
            timestamp,
            nonce,
            vec![],
            Some(CoinbaseTransaction::new(
                miner_address,
                block_reward(height),
                fees,
            )),
            transactions,
            qcash_transactions,
        ))
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
            if self.transaction_count() != 0 {
                return Err(BlockError::InvalidTransaction);
            }
        } else if self.coinbase.is_none() {
            return Err(BlockError::MissingCoinbase);
        } else if !self.genesis_allocations.is_empty() {
            return Err(BlockError::UnexpectedGenesisAllocation);
        }

        if self.transaction_count() > MAX_BLOCK_TXS {
            return Err(BlockError::TooManyTransactions);
        }

        self.checked_total_fees()?;
        if let Some(coinbase) = &self.coinbase {
            coinbase.checked_total()?;
        }

        if self.serialized_size() > MAX_BLOCK_SIZE {
            return Err(BlockError::BlockTooLarge);
        }
        if self.weight() > MAX_BLOCK_WEIGHT {
            return Err(BlockError::BlockTooHeavy);
        }

        if self.header.timestamp > now.saturating_add(MAX_FUTURE_TIME as u64) {
            return Err(BlockError::FutureTimestamp);
        }

        if !signed_transactions_are_valid_for_height(&self.transactions, self.height()) {
            return Err(BlockError::InvalidTransaction);
        }
        if self
            .qcash_transactions
            .iter()
            .any(|tx| tx.validate_signed_for_height(self.height()).is_err())
        {
            return Err(BlockError::InvalidTransaction);
        }

        if let Some(coinbase) = &self.coinbase
            && (coinbase.to != self.header.miner_address
                || coinbase.fees != self.checked_total_fees()?)
        {
            return Err(BlockError::InvalidCoinbase);
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
                &self.qcash_transactions,
            )
        {
            return Err(BlockError::InvalidMerkleRoot);
        }

        if self.header.witness_root != self.calculate_witness_merkle_root() {
            return Err(BlockError::InvalidWitnessRoot);
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
        self.checked_total_fees().unwrap_or(Amount(u64::MAX))
    }

    pub fn checked_total_fees(&self) -> Result<Amount, BlockError> {
        checked_fees(&self.transactions, &self.qcash_transactions)
    }

    pub fn miner_revenue(&self, subsidy: Amount) -> MinerRevenue {
        MinerRevenue {
            subsidy,
            fees: self.total_fees(),
        }
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len() + self.qcash_transactions.len()
    }

    pub fn is_genesis(&self) -> bool {
        self.header.height.0 == 0 && self.header.previous_hash == Hash([0; HASH_SIZE])
    }

    pub fn serialized_size(&self) -> usize {
        self.to_bytes().len()
    }

    /// Size of the header and transaction payload sections, excluding witness sections.
    pub fn stripped_size(&self) -> usize {
        crate::codec::stripped_block_bytes(self).len()
    }

    /// Size of the six witness sections, including their canonical length prefixes.
    pub fn witness_size(&self) -> usize {
        self.serialized_size().saturating_sub(self.stripped_size())
    }

    pub fn weight(&self) -> usize {
        self.stripped_size()
            .saturating_mul(WITNESS_SCALE_FACTOR)
            .saturating_add(self.witness_size())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        block_bytes(self)
    }

    pub fn calculate_merkle_root(&self) -> MerkleHash {
        calculate_merkle_root(
            &self.genesis_allocations,
            self.coinbase.as_ref(),
            &self.transactions,
            &self.qcash_transactions,
        )
    }

    pub fn calculate_witness_merkle_root(&self) -> WitnessMerkleHash {
        calculate_witness_merkle_root(&self.transactions, &self.qcash_transactions)
    }

    pub fn refresh_merkle_root(&mut self) {
        self.refresh_commitments();
    }

    pub fn refresh_commitments(&mut self) {
        self.header.merkle_root = self.calculate_merkle_root();
        self.header.witness_root = self.calculate_witness_merkle_root();
    }

    pub fn push_transaction(&mut self, transaction: SignedTransaction) {
        self.transactions.push(transaction);
        if let Ok(fees) = self.checked_total_fees()
            && let Some(coinbase) = &mut self.coinbase
        {
            coinbase.fees = fees;
        }
        self.refresh_merkle_root();
    }
}

fn calculate_merkle_root(
    genesis_allocations: &[GenesisAllocation],
    coinbase: Option<&CoinbaseTransaction>,
    transactions: &[SignedTransaction],
    qcash_transactions: &[SignedQCashTransaction],
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
        .chain(qcash_transactions.iter().map(|tx| tx.hash().as_hash()))
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

fn calculate_witness_merkle_root(
    transactions: &[SignedTransaction],
    qcash_transactions: &[SignedQCashTransaction],
) -> WitnessMerkleHash {
    let mut hashes: Vec<Hash> = transactions
        .iter()
        .map(|tx| tx.wtxid().as_hash())
        .chain(qcash_transactions.iter().map(|tx| tx.wtxid().as_hash()))
        .collect();

    if hashes.is_empty() {
        return WitnessMerkleHash::ZERO;
    }

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
                domain_hash(HashDomain::WitnessMerkleNode, &bytes)
            })
            .collect();
    }

    WitnessMerkleHash(hashes[0].0)
}

fn checked_fees(
    transactions: &[SignedTransaction],
    qcash_transactions: &[SignedQCashTransaction],
) -> Result<Amount, BlockError> {
    transactions
        .iter()
        .map(|tx| tx.transaction.fee.0)
        .chain(qcash_transactions.iter().map(|tx| tx.transaction.fee.0))
        .try_fold(0u64, |total, fee| total.checked_add(fee))
        .map(Amount)
        .ok_or(BlockError::FeeOverflow)
}

fn signed_transactions_are_valid_for_height(
    transactions: &[SignedTransaction],
    height: BlockHeight,
) -> bool {
    if transactions.len() < 2 {
        return transactions
            .iter()
            .all(|tx| tx.validate_signed_for_height(height).is_ok());
    }

    let workers = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1)
        .min(transactions.len());
    if workers <= 1 {
        return transactions
            .iter()
            .all(|tx| tx.validate_signed_for_height(height).is_ok());
    }

    let chunk_size = transactions.len().div_ceil(workers);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in transactions.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                chunk
                    .iter()
                    .all(|tx| tx.validate_signed_for_height(height).is_ok())
            }));
        }

        handles
            .into_iter()
            .all(|handle| handle.join().unwrap_or(false))
    })
}
