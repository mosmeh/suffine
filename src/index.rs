use crate::{slice_from_bytes, Result};
use std::borrow::Cow;

pub struct Index<'s, 't> {
    text: &'s str,
    suffix_array: Cow<'t, [u32]>,
}

impl<'s, 't> Index<'s, 't> {
    pub fn from_parts<S>(text: &'s str, suffix_array: S) -> Result<Index<'s, 't>>
    where
        S: Into<Cow<'t, [u32]>>,
    {
        Ok(Index {
            text,
            suffix_array: suffix_array.into(),
        })
    }

    pub fn from_bytes(text: &'s str, bytes: &'t [u8]) -> Result<Index<'s, 't>> {
        Index::from_parts(text, unsafe { slice_from_bytes(bytes) })
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn suffix_array(&self) -> &[u32] {
        &self.suffix_array
    }
}
