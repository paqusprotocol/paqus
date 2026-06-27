use crate::codec::{HashDomain, canonical_bytes, domain_hash};
use crate::params::{SNAPSHOT_INTERVAL, SNAPSHOT_MIN_CONFIRMATIONS};
use crate::types::{BlockHash, Hash, Height, StateRoot};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotRootPayload {
    pub height: Height,
    pub block_hash: BlockHash,
    pub state_root: StateRoot,
    pub accounts_root: Hash,
}

pub fn snapshot_root(
    height: Height,
    block_hash: BlockHash,
    state_root: StateRoot,
    accounts_root: Hash,
) -> Hash {
    domain_hash(
        HashDomain::SnapshotRoot,
        &canonical_bytes(&SnapshotRootPayload {
            height,
            block_hash,
            state_root,
            accounts_root,
        }),
    )
}

pub fn is_snapshot_height(height: Height) -> bool {
    height.0 != 0 && height.0.is_multiple_of(SNAPSHOT_INTERVAL)
}

pub fn is_snapshot_finalized(snapshot_height: Height, tip_height: Height) -> bool {
    tip_height.0
        >= snapshot_height
            .0
            .saturating_add(SNAPSHOT_MIN_CONFIRMATIONS as u64)
}
