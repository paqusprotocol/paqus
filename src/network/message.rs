use crate::block::Block;
use crate::network::error::NetworkError;
use crate::params::{MAX_NETWORK_MESSAGE_SIZE, NETWORK_MAGIC};
use crate::transaction::SignedTransaction;
use crate::types::{BlockHash, BlockHeight};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct PeerInfo {
    pub address: String,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TipInfo {
    pub height: BlockHeight,
    pub hash: BlockHash,
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum NetworkMessage {
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    GetTip,
    Tip(TipInfo),
    GetBlockByHeight { height: BlockHeight },
    GetBlockByHash { hash: BlockHash },
    Block(Block),
    Transaction(SignedTransaction),
    GetPeers,
    Peers(Vec<PeerInfo>),
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct NetworkEnvelope {
    pub magic: [u8; 4],
    pub message: NetworkMessage,
}

impl NetworkEnvelope {
    pub fn new(message: NetworkMessage) -> Self {
        Self {
            magic: NETWORK_MAGIC,
            message,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, NetworkError> {
        let bytes = borsh::to_vec(self)?;
        if bytes.len() > MAX_NETWORK_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooLarge);
        }
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NetworkError> {
        if bytes.len() > MAX_NETWORK_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooLarge);
        }

        let envelope = Self::try_from_slice(bytes)?;
        if envelope.magic != NETWORK_MAGIC {
            return Err(NetworkError::Serialization(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "network magic mismatch",
            )));
        }
        Ok(envelope)
    }
}

impl NetworkMessage {
    pub fn to_envelope(self) -> NetworkEnvelope {
        NetworkEnvelope::new(self)
    }
}
