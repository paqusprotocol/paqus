use crate::params::{BLOCK_VERSION, TRANSACTION_VERSION};
use crate::types::BlockHeight;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolVersions {
    pub block: u8,
    pub transaction: u8,
}

pub fn active_versions(_height: BlockHeight) -> ProtocolVersions {
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
