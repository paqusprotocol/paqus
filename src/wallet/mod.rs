pub mod error;
#[cfg(test)]
mod test;
pub mod wallet;

pub use error::WalletError;
pub use wallet::Wallet;
