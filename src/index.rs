use crate::build::{build_suffix_array, VecWrapper};
use crate::Result;
use itertools::Itertools;
use std::borrow::Cow;
use std::io;

#[derive(Clone)]
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
        if self.text.len() > u32::MAX as usize {
            return Err(crate::Error::TextTooLong);
        }

        let mut sa = VecWrapper(Vec::new());
        build_suffix_array(self.text, self.block_size, &mut sa)?;
        Ok(Index {
            text: self.text,
            suffix_array: Cow::Owned(sa.0),
        })
    }

    pub fn build_to_writer<W: io::Write>(&self, writer: W) -> Result<()> {
        if self.text.len() > u32::MAX as usize {
            return Err(crate::Error::TextTooLong);
        }
        if self.block_size == 0 {
            return Err(crate::Error::InvalidOption(
                "block size cannot be 0".to_string(),
            ));
        }

        build_suffix_array(self.text, self.block_size, writer).map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct MultiDocIndex<'a, 'b> {
    index: Cow<'b, Index<'a, 'b>>,
    offsets: Vec<u32>,
    delim_len: u32,
}

impl<'a, 'b> MultiDocIndex<'a, 'b> {
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
        if self.delimiter.is_empty() {
            return Err(crate::Error::InvalidOption(
                "delimiter cannot be empty string".to_string(),
            ));
        }

        let delim_len = self.delimiter.len() as u32;
        let offsets: Vec<u32> = [0]
            .iter()
            .copied()
            .chain(
                self.index
                    .find_positions(&self.delimiter)
                    .iter()
                    .sorted()
                    .dedup_by(|&a, &b| b - a < delim_len)
                    .map(|x| x + delim_len),
            )
            .collect();

        Ok(MultiDocIndex {
            index: self.index.clone(),
            offsets,
            delim_len,
        })
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
    use crate::{IndexBuilder, MultiDocIndexBuilder};
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
