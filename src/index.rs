use crate::{slice_from_bytes, Result};
use std::borrow::Cow;

pub struct Index<'a, 'b> {
    text: &'a str,
    suffix_array: Cow<'b, [u32]>,
}

impl<'a, 'b> Index<'a, 'b> {
    pub fn from_parts<S>(text: &'a str, suffix_array: S) -> Result<Index<'a, 'b>>
    where
        S: Into<Cow<'b, [u32]>>,
    {
        Ok(Index {
            text,
            suffix_array: suffix_array.into(),
        })
    }

    pub fn from_bytes(text: &'a str, bytes: &'b [u8]) -> Result<Index<'a, 'b>> {
        Index::from_parts(text, unsafe { slice_from_bytes(bytes) })
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn suffix_array(&self) -> &[u32] {
        &self.suffix_array
    }

    pub fn find_positions(&self, query: &str) -> &[u32] {
        if self.text.is_empty() || query.is_empty() {
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
    use crate::IndexBuilder;
    use itertools::Itertools;

    fn find_positions(text: &str) {
        let index = IndexBuilder::new(text).build().unwrap();

        assert!(index.find_positions("").is_empty());
        assert!(index.find_positions(&format!("{}$", text)).is_empty());

        for query_len in 1..text.chars().count() {
            for query in text
                .chars()
                .chunks(query_len)
                .into_iter()
                .map(|c| c.collect::<String>())
            {
                let actual =
                    itertools::sorted(index.find_positions(&query).iter()).map(|x| *x as usize);
                let expected = (0..text.len() - query.len() + 1).filter(|&i| {
                    text.is_char_boundary(i)
                        && text.is_char_boundary(i + query.len())
                        && &text[i..i + query.len()] == query
                });
                assert!(actual.eq(expected));
            }
        }
    }

    #[quickcheck]
    fn find_positions_qc(text: String) {
        find_positions(&text);
    }

    #[test]
    fn nonexistence() {
        let index = IndexBuilder::new("ab").build().unwrap();
        assert!(index.find_positions("c").is_empty());
        assert!(index.find_positions("ba").is_empty());
        assert!(index.find_positions("bc").is_empty());
    }

    #[test]
    fn exotic_characters() {
        let text = "あ\0😅吉𠮷ééがが";
        find_positions(text);
    }
}
