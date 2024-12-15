mod error;
mod wallet;
pub use error::*;
pub use wallet::*;
pub type Result<T> = std::result::Result<T, Error>;
