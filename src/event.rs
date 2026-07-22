//! Deterministic receipts emitted by successful protocol state transitions.

use crate::block::BlockHeight;
use crate::consensus::supply::Amount;
use crate::crypto::{Address, BlockHash, HashDomain, TransactionHash, domain_hash};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_EVENT_VERSION: u8 = 1;
pub const MAX_PROTOCOL_EVENT_SIZE: usize = 256 * 1024;

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
pub struct EventId(pub [u8; 32]);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub enum ProtocolEventKind {
    Transfer {
        from: Address,
        to: Address,
        amount: Amount,
        fee: Amount,
    },
    QCashWithdrawn {
        signer: Address,
        amount: Amount,
    },
    QCashDeposited {
        signer: Address,
        recipient: Address,
        amount: Amount,
    },
    GenesisAllocation {
        recipient: Address,
        amount: Amount,
    },
    CoinbasePaid {
        miner: Address,
        subsidy: Amount,
    },
    MinerFeeRevenue {
        miner: Address,
        fees: Amount,
    },
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct ProtocolEvent {
    pub version: u8,
    pub block_height: BlockHeight,
    pub block_hash: BlockHash,
    pub transaction_hash: Option<TransactionHash>,
    pub event_index: u32,
    pub kind: ProtocolEventKind,
}

impl ProtocolEvent {
    pub fn new(
        block_height: BlockHeight,
        block_hash: BlockHash,
        transaction_hash: Option<TransactionHash>,
        event_index: u32,
        kind: ProtocolEventKind,
    ) -> Self {
        Self {
            version: PROTOCOL_EVENT_VERSION,
            block_height,
            block_hash,
            transaction_hash,
            event_index,
            kind,
        }
    }

    pub fn id(&self) -> EventId {
        EventId(domain_hash(HashDomain::ProtocolEvent, &self.to_bytes()).0)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        crate::codec::protocol_event_bytes(self)
    }

    pub fn validate(&self) -> bool {
        self.version == PROTOCOL_EVENT_VERSION && self.block_hash != BlockHash::ZERO
    }
}
