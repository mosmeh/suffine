#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

mod build;
mod index;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub use build::IndexBuilder;
pub use index::Index;

unsafe fn slice_from_bytes<T>(bytes: &[u8]) -> &[T] {
    assert_eq!(0, std::mem::size_of::<T>() % std::mem::size_of::<u8>());
    assert_eq!(0, bytes.len() % std::mem::size_of::<T>());

    let ratio = std::mem::size_of::<T>() / std::mem::size_of::<u8>();
    let ptr = bytes.as_ptr() as *const T;
    let length = bytes.len() / ratio;

    std::slice::from_raw_parts(ptr, length)
}
