pub mod error;
pub mod storage;
#[cfg(test)]
mod test;

pub use error::StorageError;
pub use storage::{StateSnapshot, Storage};
