use crate::build::{build_suffix_array, IntBuffer, VecWrapper};
use crate::Result;
use byteorder::{BigEndian, ByteOrder, LittleEndian, NativeEndian, ReadBytesExt, WriteBytesExt};
use itertools::Itertools;
use std::borrow::Cow;
use std::io::Write;
use std::mem;

#[derive(Clone, Debug, PartialEq)]
pub struct Index<'a, 'b> {
    text: &'a str,
    suffix_array: Cow<'b, [u32]>,
}

impl<'a, 'b> Index<'a, 'b> {
    pub fn from_bytes(text: &'a str, bytes: &'b [u8]) -> Result<Index<'a, 'b>> {
        if text.len() > u32::MAX as usize {
            return Err(crate::Error::TextTooLong);
        }

        let suffix_array = if bytes.is_empty() {
            &[]
        } else {
            bytemuck::try_cast_slice(&bytes).or(Err(crate::Error::InvalidIndex))?
        };
        if suffix_array.len() > text.len() {
            return Err(crate::Error::InvalidIndex);
        }

        Ok(Index {
            text,
            suffix_array: std::borrow::Cow::Borrowed(suffix_array),
        })
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn suffix_array(&self) -> &[u32] {
        &self.suffix_array
    }

    pub fn find_positions(&self, query: &str) -> &[u32] {
        if self.text.is_empty() || query.is_empty() || query.len() > self.text.len() {
            return &[];
        }
        let first_suffix = &self.text[self.suffix_array[0] as usize..];
        let last_suffix = &self.text[self.suffix_array[self.suffix_array.len() - 1] as usize..];
        if (query < first_suffix && !first_suffix.starts_with(query)) || query > last_suffix {
            return &[];
        }

        let start = binary_search(&self.suffix_array, |&i| query <= &self.text[i as usize..]);
        let end = start
            + binary_search(&self.suffix_array[start..], |&i| {
                !self.text[i as usize..].starts_with(query)
            });

        if start > end {
            &[]
        } else {
            &self.suffix_array[start..end]
        }
    }
}

impl<'a, 'b> From<Index<'a, 'b>> for Cow<'b, Index<'a, 'b>> {
    fn from(index: Index<'a, 'b>) -> Cow<'b, Index<'a, 'b>> {
        Cow::Owned(index)
    }
}

impl<'a, 'b> From<&'b Index<'a, 'b>> for Cow<'b, Index<'a, 'b>> {
    fn from(index: &'b Index<'a, 'b>) -> Cow<'b, Index<'a, 'b>> {
        Cow::Borrowed(index)
    }
}

impl<'a, 'b> From<Cow<'b, Index<'a, 'b>>> for Index<'a, 'b> {
    fn from(index: Cow<'b, Index<'a, 'b>>) -> Index<'a, 'b> {
        index.into_owned()
    }
}

#[derive(Clone)]
pub struct IndexBuilder<'a> {
    text: &'a str,
    block_size: u32,
}

impl<'a> IndexBuilder<'a> {
    pub fn new(text: &'a str) -> IndexBuilder<'a> {
        IndexBuilder {
            text,
            block_size: u32::MAX,
        }
    }

    pub fn block_size(&mut self, block_size: u32) -> &mut Self {
        self.block_size = block_size;
        self
    }

    pub fn build(&self) -> Result<Index<'a, 'static>> {
        let mut sa = VecWrapper(Vec::new());
        self.build_to_buffer(&mut sa)?;
        Ok(Index {
            text: self.text,
            suffix_array: Cow::Owned(sa.0),
        })
    }

    pub fn build_to_writer_little_endian<W: Write>(&self, writer: W) -> Result<()> {
        self.build_to_buffer::<W, LittleEndian>(writer)
    }

    pub fn build_to_writer_big_endian<W: Write>(&self, writer: W) -> Result<()> {
        self.build_to_buffer::<W, BigEndian>(writer)
    }

    pub fn build_to_writer_native_endian<W: Write>(&self, writer: W) -> Result<()> {
        self.build_to_buffer::<W, NativeEndian>(writer)
    }

    fn build_to_buffer<B, O>(&self, buffer: B) -> Result<()>
    where
        B: IntBuffer<u32, O>,
        O: ByteOrder,
    {
        if self.text.len() > u32::MAX as usize {
            return Err(crate::Error::TextTooLong);
        }
        if self.block_size == 0 {
            return Err(crate::Error::InvalidOption(
                "block size cannot be 0".to_string(),
            ));
        }
        build_suffix_array(self.text, self.block_size, buffer)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct MultiDocIndex<'a, 'b> {
    index: Cow<'b, Index<'a, 'b>>,
    offsets: Cow<'b, [u32]>,
    delim_len: u32,
}

impl<'a, 'b> MultiDocIndex<'a, 'b> {
    pub fn from_bytes(text: &'a str, bytes: &'b [u8]) -> Result<MultiDocIndex<'a, 'b>> {
        /* format:
               index.suffix_array
               offsets
               footer
        */
        const FOOTER_SIZE: usize = mem::size_of::<u32>()
            * (
                // index.suffix_array.len()
                1
                // offsets.len()
                + 1
                // delim_len
                + 1
            );

        // footer
        let mut cursor = std::io::Cursor::new(&bytes[bytes.len() - FOOTER_SIZE..]);
        let sa_len = cursor.read_u32::<NativeEndian>()? as usize;
        let offsets_len = cursor.read_u32::<NativeEndian>()? as usize;
        let delim_len = cursor.read_u32::<NativeEndian>()?;

        let sa_size = mem::size_of::<u32>() * sa_len;
        let offsets_size = mem::size_of::<u32>() * offsets_len;

        if bytes.len() != sa_size + offsets_size + FOOTER_SIZE {
            return Err(crate::Error::InvalidIndex);
        }

        // body
        let sa_bytes = &bytes[0..sa_size];
        let index = Index::from_bytes(text, sa_bytes)?;

        let offsets_bytes = &bytes[sa_size..sa_size + offsets_size];
        let offsets = bytemuck::try_cast_slice(&offsets_bytes)
            .or_else(|_| Err(crate::Error::InvalidIndex))?;

        Ok(MultiDocIndex {
            index: Cow::Owned(index),
            offsets: Cow::Borrowed(&offsets),
            delim_len,
        })
    }

    pub fn index(&self) -> &Index<'a, 'b> {
        &self.index
    }

    pub fn find_positions(&self, query: &str) -> Vec<(u32, u32)> {
        self.index
            .find_positions(&query)
            .iter()
            .filter_map(|p| {
                self.doc_id_from_range(*p, *p + query.len() as u32)
                    .map(|doc_id| {
                        let pos_in_doc = p - self.offsets[doc_id as usize];
                        (doc_id, pos_in_doc)
                    })
            })
            .collect()
    }

    pub fn doc(&self, doc_id: u32) -> Option<&str> {
        let doc_id = doc_id as usize;
        if doc_id >= self.offsets.len() {
            return None;
        }
        let begin = self.offsets[doc_id] as usize;
        let end = if doc_id == self.offsets.len() - 1 {
            self.index.text().len()
        } else {
            (self.offsets[doc_id + 1] - self.delim_len) as usize
        };
        Some(&self.index.text()[begin..end])
    }

    fn doc_id_from_range(&self, begin: u32, end: u32) -> Option<u32> {
        match self.offsets.binary_search(&begin) {
            Ok(x) if x == self.offsets.len() - 1 || end + self.delim_len <= self.offsets[x + 1] => {
                Some(x as u32)
            }
            Err(x) if x == self.offsets.len() || end + self.delim_len <= self.offsets[x] => {
                Some((x - 1) as u32)
            }
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct MultiDocIndexBuilder<'a, 'b> {
    index: Cow<'b, Index<'a, 'b>>,
    delimiter: String,
}

impl<'a, 'b> MultiDocIndexBuilder<'a, 'b> {
    pub fn new<I>(index: I) -> Self
    where
        I: Into<Cow<'b, Index<'a, 'b>>>,
    {
        Self {
            index: index.into(),
            delimiter: "\n".to_string(),
        }
    }

    pub fn delimiter(&mut self, delimiter: &str) -> &mut Self {
        self.delimiter = delimiter.to_string();
        self
    }

    pub fn build(&self) -> Result<MultiDocIndex<'a, 'b>> {
        Ok(MultiDocIndex {
            index: self.index.clone(),
            offsets: Cow::Owned(self.calc_offsets()?),
            delim_len: self.delimiter.len() as u32,
        })
    }

    pub fn build_to_writer_little_endian<W: Write>(&self, writer: W) -> Result<()> {
        self.build_to_writer::<W, LittleEndian>(writer)
    }

    pub fn build_to_writer_big_endian<W: Write>(&self, writer: W) -> Result<()> {
        self.build_to_writer::<W, BigEndian>(writer)
    }

    pub fn build_to_writer_native_endian<W: Write>(&self, writer: W) -> Result<()> {
        self.build_to_writer::<W, NativeEndian>(writer)
    }

    fn build_to_writer<W, O>(&self, mut writer: W) -> Result<()>
    where
        W: Write,
        O: ByteOrder,
    {
        // See MultiDocIndex::from_bytes for format

        // body
        let offsets = self.calc_offsets()?;
        for x in self.index.suffix_array().iter().chain(offsets.iter()) {
            writer.write_u32::<O>(*x)?;
        }

        // footer
        writer.write_u32::<O>(self.index.suffix_array().len() as u32)?;
        writer.write_u32::<O>(offsets.len() as u32)?;
        writer.write_u32::<O>(self.delimiter.len() as u32)?;

        Ok(())
    }

    fn calc_offsets(&self) -> Result<Vec<u32>> {
        if self.delimiter.is_empty() {
            return Err(crate::Error::InvalidOption(
                "delimiter cannot be empty string".to_string(),
            ));
        }

        let offsets = [0]
            .iter()
            .copied()
            .chain(
                self.index
                    .find_positions(&self.delimiter)
                    .iter()
                    .sorted()
                    .dedup_by(|&a, &b| b - a < self.delimiter.len() as u32)
                    .map(|x| x + self.delimiter.len() as u32),
            )
            .collect();
        Ok(offsets)
    }
}

fn binary_search<T, F>(xs: &[T], mut pred: F) -> usize
where
    F: FnMut(&T) -> bool,
{
    let (mut left, mut right) = (0, xs.len());
    while left < right {
        let mid = (left + right) / 2;
        if pred(&xs[mid]) {
            right = mid;
        } else {
            left = mid + 1;
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use crate::{Index, IndexBuilder, MultiDocIndex, MultiDocIndexBuilder};
    use itertools::Itertools;
    use quickcheck::TestResult;

    fn find_positions_naive(text: &str, query: &str) -> Vec<usize> {
        if text.len() < query.len() {
            return Vec::new();
        }

        (0..=text.len() - query.len())
            .filter(|&i| {
                text.is_char_boundary(i)
                    && text.is_char_boundary(i + query.len())
                    && &text[i..i + query.len()] == query
            })
            .collect()
    }

    fn check_positions(text: &str) {
        let index = IndexBuilder::new(text).build().unwrap();

        assert!(index.find_positions("").is_empty());
        assert!(index.find_positions(&format!("{}$", text)).is_empty());

        for end in 1..=text.len() {
            if !text.is_char_boundary(end) {
                continue;
            }
            for begin in 0..end {
                if !text.is_char_boundary(begin) {
                    continue;
                }
                let query = &text[begin..end];
                let actual = index
                    .find_positions(&query)
                    .iter()
                    .sorted()
                    .map(|x| *x as usize);
                let expected = find_positions_naive(text, &query);
                assert!(actual.eq(expected));
            }
        }
    }

    #[quickcheck]
    fn find_positions(text: String) {
        check_positions(&text);
    }

    #[test]
    fn exotic_characters() {
        let text = "あ\0😅吉𠮷ééがが";
        check_positions(text);
    }

    #[test]
    fn nonexistence() {
        let index = IndexBuilder::new("ab").build().unwrap();
        assert!(index.find_positions("c").is_empty());
        assert!(index.find_positions("ba").is_empty());
        assert!(index.find_positions("bc").is_empty());
    }

    #[quickcheck]
    fn deserialize_index(text: String) {
        let a = IndexBuilder::new(&text).build().unwrap();

        let mut buf = Vec::new();
        IndexBuilder::new(&text)
            .build_to_writer_native_endian(&mut buf)
            .unwrap();
        let b = Index::from_bytes(&text, &buf).unwrap();

        assert_eq!(a, b);
    }

    #[quickcheck]
    fn deserialize_multi_doc_index(texts: Vec<String>, delim: String) -> TestResult {
        if delim.is_empty() {
            return TestResult::discard();
        }

        let text = texts.iter().join(&delim);
        let index = IndexBuilder::new(&text).build().unwrap();

        let a = MultiDocIndexBuilder::new(&index).build().unwrap();

        let mut buf = Vec::new();
        MultiDocIndexBuilder::new(&index)
            .build_to_writer_native_endian(&mut buf)
            .unwrap();
        let b = MultiDocIndex::from_bytes(&text, &buf).unwrap();

        assert_eq!(a, b);

        TestResult::passed()
    }

    #[quickcheck]
    fn multi_doc_basic(texts: Vec<String>, delim: String) -> TestResult {
        if delim.is_empty() {
            return TestResult::discard();
        }
        let text = texts.iter().join(&delim);
        let texts: Vec<&str> = text.split(&delim).collect();

        let index = IndexBuilder::new(&text).build().unwrap();
        let multi_doc_index = MultiDocIndexBuilder::new(index)
            .delimiter(&delim)
            .build()
            .unwrap();

        assert!(multi_doc_index.find_positions("").is_empty());

        assert!(texts
            .iter()
            .enumerate()
            .all(|(i, t)| &multi_doc_index.doc(i as u32).unwrap() == t));
        assert!(multi_doc_index.doc(texts.len() as u32).is_none());

        TestResult::passed()
    }

    #[quickcheck]
    fn multi_doc_extra(texts: Vec<String>, delim: String) -> TestResult {
        if delim.is_empty() {
            return TestResult::discard();
        }
        let text = texts.iter().join(&delim);
        if text.len() > 100 {
            return TestResult::discard();
        }
        let texts: Vec<&str> = text.split(&delim).collect();

        let index = IndexBuilder::new(&text).build().unwrap();
        let multi_doc_index = MultiDocIndexBuilder::new(index)
            .delimiter(&delim)
            .build()
            .unwrap();

        for t in texts.iter() {
            for end in 1..=t.len() {
                if !t.is_char_boundary(end) {
                    continue;
                }
                for begin in 0..end {
                    if !t.is_char_boundary(begin) {
                        continue;
                    }
                    let query = &t[begin..end];

                    let actual = multi_doc_index
                        .find_positions(&query)
                        .iter()
                        .copied()
                        .sorted();
                    let expected = texts
                        .iter()
                        .enumerate()
                        .map(|(i, u)| {
                            find_positions_naive(u, &query)
                                .into_iter()
                                .map(|p| (i as u32, p as u32))
                                .collect::<Vec<(_, _)>>()
                                .into_iter()
                        })
                        .flatten();
                    assert!(actual.eq(expected));
                }
            }
        }

        TestResult::passed()
    }
}
