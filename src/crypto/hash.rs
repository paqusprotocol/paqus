use argon2::{Algorithm, Argon2, Params, Version};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha3::{Digest, Sha3_256};
use static_assertions::const_assert_eq;
use std::fmt;

use crate::error::CryptoError;

pub const HASH_SIZE: usize = 32;
pub const PROOF_OF_WORK_HASH_SIZE: usize = 32;
const_assert_eq!(HASH_SIZE, 32);
const_assert_eq!(PROOF_OF_WORK_HASH_SIZE, 32);

pub const SNAPSHOT_ROOT_DOMAIN: &[u8] = b"PAQUS_SNAPSHOT_ROOT_V1";

const ARGON2_POW_SALT: &[u8] = b"paquscore-proof-of-work";
const ARGON2_POW_MEMORY_KIB: u32 = 512 * 1024; // 512MiB
const ARGON2_POW_TIME_COST: u32 = 2;
const ARGON2_POW_PARALLELISM: u32 = 2;
const ARGON2_POW_OUTPUT_LEN: usize = 32;

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
pub struct ProofOfWorkHash(pub ProofOfWorkHashBytes);

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
hash_newtype!(MerkleHash);
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
    SignedTransaction,
    BlockHeader,
    GenesisAllocation,
    Coinbase,
    MerkleNode,
    AccountState,
    StateNode,
    SnapshotRoot,
    Raw,
}

impl HashDomain {
    fn tag(self) -> &'static [u8] {
        match self {
            HashDomain::Transaction => b"PAQUS_HASH_TX",
            HashDomain::SignedTransaction => b"PAQUS_HASH_SIGNED_TX",
            HashDomain::BlockHeader => b"PAQUS_HASH_BLOCK_HEADER",
            HashDomain::GenesisAllocation => b"PAQUS_HASH_GENESIS_ALLOCATION",
            HashDomain::Coinbase => b"PAQUS_HASH_COINBASE",
            HashDomain::MerkleNode => b"PAQUS_HASH_MERKLE_NODE",
            HashDomain::AccountState => b"PAQUS_HASH_ACCOUNT_STATE",
            HashDomain::StateNode => b"PAQUS_HASH_STATE_NODE",
            HashDomain::SnapshotRoot => SNAPSHOT_ROOT_DOMAIN,
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

pub fn argon2_proof_of_work_hash(header_bytes: &[u8]) -> Result<ProofOfWorkHash, CryptoError> {
    let params = Params::new(
        ARGON2_POW_MEMORY_KIB,
        ARGON2_POW_TIME_COST,
        ARGON2_POW_PARALLELISM,
        Some(ARGON2_POW_OUTPUT_LEN),
    )
    .map_err(|_| CryptoError::InvalidProofOfWorkParameters)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output = [0_u8; PROOF_OF_WORK_HASH_SIZE];

    argon2
        .hash_password_into(header_bytes, ARGON2_POW_SALT, &mut output)
        .map_err(|_| CryptoError::ProofOfWorkHashFailed)?;

    Ok(ProofOfWorkHash(output))
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
