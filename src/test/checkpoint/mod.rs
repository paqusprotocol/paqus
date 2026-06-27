use crate::checkpoint::{checkpoint_for_height, validate_checkpoint};
use crate::types::{BlockHash, Height};

#[test]
fn accepts_blocks_when_no_checkpoint_exists_for_height() {
    assert_eq!(checkpoint_for_height(Height(123)), None);
    assert!(validate_checkpoint(Height(123), BlockHash([7; 64])));
}
