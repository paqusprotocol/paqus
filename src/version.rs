use crate::params::{
    BLOCK_VERSION, BLOCK_VERSION_ACTIVATION_HEIGHT, TRANSACTION_VERSION,
    TRANSACTION_VERSION_ACTIVATION_HEIGHT,
};
use crate::types::{BlockHeight, Height};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolVersions {
    pub block: u8,
    pub transaction: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionActivation {
    pub height: BlockHeight,
    pub versions: ProtocolVersions,
}

pub const VERSION_ACTIVATIONS: &[VersionActivation] = &[VersionActivation {
    height: Height(activation_height(
        BLOCK_VERSION_ACTIVATION_HEIGHT,
        TRANSACTION_VERSION_ACTIVATION_HEIGHT,
    )),
    versions: ProtocolVersions {
        block: BLOCK_VERSION,
        transaction: TRANSACTION_VERSION,
    },
}];

const fn activation_height(block_height: u64, transaction_height: u64) -> u64 {
    if block_height > transaction_height {
        block_height
    } else {
        transaction_height
    }
}

pub fn active_versions(height: BlockHeight) -> ProtocolVersions {
    VERSION_ACTIVATIONS
        .iter()
        .rev()
        .find(|activation| activation.height.0 <= height.0)
        .map(|activation| activation.versions)
        .unwrap_or(ProtocolVersions {
            block: BLOCK_VERSION,
            transaction: TRANSACTION_VERSION,
        })
}

pub fn active_block_version(height: BlockHeight) -> u8 {
    active_versions(height).block
}

pub fn active_transaction_version(height: BlockHeight) -> u8 {
    active_versions(height).transaction
}

pub fn genesis_versions() -> ProtocolVersions {
    ProtocolVersions {
        block: BLOCK_VERSION,
        transaction: TRANSACTION_VERSION,
    }
}

pub fn supported_block_version(height: BlockHeight, version: u8) -> bool {
    version == active_versions(height).block
}

pub fn supported_transaction_version(height: BlockHeight, version: u8) -> bool {
    version == active_versions(height).transaction
}
