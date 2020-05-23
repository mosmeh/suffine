#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

mod build;
mod error;
mod index;

pub use error::Error;
pub type Result<T> = std::result::Result<T, error::Error>;

pub use index::{Index, IndexBuilder, MultiDocIndex, MultiDocIndexBuilder};
