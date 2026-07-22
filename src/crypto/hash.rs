use borsh::{BorshDeserialize, BorshSerialize};
use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha3::{Digest, Sha3_256, Sha3_512};
use static_assertions::const_assert_eq;
use std::fmt;

pub const HASH_SIZE: usize = 32;
pub const PROOF_OF_WORK_HASH_SIZE: usize = 64;
const_assert_eq!(HASH_SIZE, 32);
const_assert_eq!(PROOF_OF_WORK_HASH_SIZE, 64);

pub type HashBytes = [u8; HASH_SIZE];
pub type ProofOfWorkHashBytes = [u8; PROOF_OF_WORK_HASH_SIZE];

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize,
)]
pub struct Hash(pub HashBytes);

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HashVisitor;

        impl<'de> Visitor<'de> for HashVisitor {
            type Value = Hash;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{HASH_SIZE} hash bytes")
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                let bytes: HashBytes = value
                    .try_into()
                    .map_err(|_| E::invalid_length(value.len(), &self))?;
                Ok(Hash(bytes))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bytes = [0_u8; HASH_SIZE];
                for (index, byte) in bytes.iter_mut().enumerate() {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| DeError::invalid_length(index, &self))?;
                }
                Ok(Hash(bytes))
            }
        }

        deserializer.deserialize_bytes(HashVisitor)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize,
)]
pub struct ProofOfWorkHash(pub ProofOfWorkHashBytes);

impl Serialize for ProofOfWorkHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProofOfWorkHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ProofOfWorkHashVisitor;

        impl<'de> Visitor<'de> for ProofOfWorkHashVisitor {
            type Value = ProofOfWorkHash;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "{PROOF_OF_WORK_HASH_SIZE} proof-of-work hash bytes"
                )
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                let bytes: ProofOfWorkHashBytes = value
                    .try_into()
                    .map_err(|_| E::invalid_length(value.len(), &self))?;
                Ok(ProofOfWorkHash(bytes))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bytes = [0_u8; PROOF_OF_WORK_HASH_SIZE];
                for (index, byte) in bytes.iter_mut().enumerate() {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| DeError::invalid_length(index, &self))?;
                }
                Ok(ProofOfWorkHash(bytes))
            }
        }

        deserializer.deserialize_bytes(ProofOfWorkHashVisitor)
    }
}

macro_rules! hash_newtype {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
            BorshSerialize,
            BorshDeserialize,
        )]
        pub struct $name(pub HashBytes);

        impl $name {
            pub const ZERO: Self = Self([0; HASH_SIZE]);

            pub fn as_hash(self) -> Hash {
                Hash(self.0)
            }
        }

        impl From<Hash> for $name {
            fn from(hash: Hash) -> Self {
                Self(hash.0)
            }
        }

        impl From<$name> for Hash {
            fn from(hash: $name) -> Self {
                Hash(hash.0)
            }
        }

        impl PartialEq<Hash> for $name {
            fn eq(&self, other: &Hash) -> bool {
                self.0 == other.0
            }
        }

        impl PartialEq<$name> for Hash {
            fn eq(&self, other: &$name) -> bool {
                self.0 == other.0
            }
        }
    };
}

hash_newtype!(BlockHash);
hash_newtype!(TransactionHash);
hash_newtype!(WitnessTransactionHash);
hash_newtype!(MerkleHash);
hash_newtype!(WitnessMerkleHash);
hash_newtype!(StateRoot);
hash_newtype!(PreviousHash);

impl From<BlockHash> for PreviousHash {
    fn from(hash: BlockHash) -> Self {
        Self(hash.0)
    }
}

impl PartialEq<BlockHash> for PreviousHash {
    fn eq(&self, other: &BlockHash) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<PreviousHash> for BlockHash {
    fn eq(&self, other: &PreviousHash) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashDomain {
    Transaction,
    WitnessTransaction,
    BlockHeader,
    ChainParams,
    GenesisAllocation,
    Coinbase,
    MerkleNode,
    WitnessMerkleNode,
    AccountState,
    StateNode,
    QCashCoin,
    QCashCommitment,
    QCashDepositAuthorization,
    QCashDepositTransaction,
    QCashFile,
    QCashState,
    ProtocolEvent,
    ProtocolState,
    Raw,
}

impl HashDomain {
    fn tag(self) -> &'static [u8] {
        match self {
            HashDomain::Transaction => b"PAQUS_HASH_TX",
            HashDomain::WitnessTransaction => b"PAQUS_HASH_WITNESS_TX_V1",
            HashDomain::BlockHeader => b"PAQUS_HASH_BLOCK_HEADER",
            HashDomain::ChainParams => b"PAQUS_HASH_CHAIN_PARAMS_V1",
            HashDomain::GenesisAllocation => b"PAQUS_HASH_GENESIS_ALLOCATION",
            HashDomain::Coinbase => b"PAQUS_HASH_COINBASE",
            HashDomain::MerkleNode => b"PAQUS_HASH_MERKLE_NODE",
            HashDomain::WitnessMerkleNode => b"PAQUS_HASH_WITNESS_MERKLE_NODE_V1",
            HashDomain::AccountState => b"PAQUS_HASH_ACCOUNT_STATE",
            HashDomain::StateNode => b"PAQUS_HASH_STATE_NODE",
            // These legacy version-1 tags are consensus bytes. Their spelling
            // remains stable even though the public feature name is QCash.
            HashDomain::QCashCoin => b"PAQUS_HASH_ECASH_COIN_V1",
            HashDomain::QCashCommitment => b"PAQUS_HASH_ECASH_COMMITMENT_V1",
            HashDomain::QCashDepositAuthorization => b"PAQUS_HASH_QCASH_DEPOSIT_AUTH_V2",
            HashDomain::QCashDepositTransaction => b"PAQUS_HASH_QCASH_DEPOSIT_TX_V1",
            HashDomain::QCashFile => b"PAQUS_HASH_ECASH_FILE_V1",
            HashDomain::QCashState => b"PAQUS_HASH_ECASH_STATE_V1",
            HashDomain::ProtocolEvent => b"PAQUS_HASH_PROTOCOL_EVENT_V1",
            HashDomain::ProtocolState => b"PAQUS_HASH_PROTOCOL_STATE_V1",
            HashDomain::Raw => b"PAQUS_HASH_RAW",
        }
    }
}

pub fn hash_bytes(bytes: &[u8]) -> Hash {
    domain_hash(HashDomain::Raw, bytes)
}

pub fn domain_hash(domain: HashDomain, bytes: &[u8]) -> Hash {
    let mut hasher = Sha3_256::new();
    hasher.update(domain.tag());
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hash = [0_u8; HASH_SIZE];
    hash.copy_from_slice(&digest);
    Hash(hash)
}

pub fn sha3_512_proof_of_work_hash(header_bytes: &[u8]) -> ProofOfWorkHash {
    let digest = Sha3_512::digest(header_bytes);
    let mut output = [0_u8; PROOF_OF_WORK_HASH_SIZE];
    output.copy_from_slice(&digest);
    ProofOfWorkHash(output)
}

pub fn hash_meets_difficulty(hash: &ProofOfWorkHash, difficulty: u32) -> bool {
    let full_zero_bytes = (difficulty / 8) as usize;
    let remaining_zero_bits = (difficulty % 8) as u8;

    if full_zero_bytes > hash.0.len() {
        return false;
    }

    if !hash.0.iter().take(full_zero_bytes).all(|byte| *byte == 0) {
        return false;
    }

    if remaining_zero_bits == 0 {
        return true;
    }

    let Some(next_byte) = hash.0.get(full_zero_bytes) else {
        return false;
    };
    let mask = 0xff << (8 - remaining_zero_bits);
    next_byte & mask == 0
}
