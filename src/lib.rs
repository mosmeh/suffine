#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

mod build;
mod index;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub use index::{Index, IndexBuilder, MultiDocIndex, MultiDocIndexBuilder};
