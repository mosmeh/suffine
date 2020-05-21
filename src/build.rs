use crate::{Index, Result};
use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};
use std::borrow::Cow;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use suffix::SuffixTable;
use tempfile::NamedTempFile;

pub struct IndexBuilder<'s> {
    text: &'s str,
    block_size: u32,
}

impl<'s> IndexBuilder<'s> {
    pub fn new(text: &'s str) -> IndexBuilder<'s> {
        IndexBuilder {
            text,
            block_size: u32::MAX,
        }
    }

    pub fn block_size(&mut self, block_size: u32) -> &mut Self {
        self.block_size = block_size.max(1);
        self
    }

    pub fn build_to_writer<W: std::io::Write>(&self, mut writer: W) -> Result<()> {
        match self.text.len() {
            0 => return Ok(()),
            1 => {
                writer.write_u32::<NativeEndian>(0)?;
                return Ok(());
            }
            _ => (),
        }

        if self.text.len() <= self.block_size as usize {
            build_suffix_array_to_writer(self.text, self.text.len(), writer)?;
        } else {
            let heap = sort_blocks(self.text, self.block_size)?;
            merge_blocks_to_writer(heap, writer)?;
        }

        Ok(())
    }

    pub fn build_in_memory(&self) -> Result<Index<'s, 'static>> {
        let sa = build_suffix_array_in_memory(self.text);
        Index::from_parts(self.text, Cow::Owned(sa))
    }
}

fn build_suffix_array_in_memory(text: &str) -> Vec<u32> {
    match text.len() {
        0 => return Vec::new(),
        1 => return vec![0],
        _ => (),
    }

    SuffixTable::new(text)
        .table()
        .iter()
        .filter(|&&x| text.is_char_boundary(x as usize))
        .copied()
        .collect()
}

fn build_suffix_array_to_writer<W: io::Write>(text: &str, len: usize, mut writer: W) -> Result<()> {
    let st = SuffixTable::new(text);
    let sa = st
        .table()
        .iter()
        .filter(|&&x| (x as usize) < len && text.is_char_boundary(x as usize));
    for x in sa {
        writer.write_u32::<NativeEndian>(*x)?;
    }

    Ok(())
}

struct Block<'a> {
    text: &'a str,
    reader: BufReader<File>,
    begin: usize,
    front_index: u32,
}

impl Eq for Block<'_> {}

impl PartialEq for Block<'_> {
    fn eq(&self, _: &Self) -> bool {
        unimplemented!()
    }
}

impl Ord for Block<'_> {
    fn cmp(&self, _: &Self) -> Ordering {
        unimplemented!()
    }
}

impl PartialOrd for Block<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.front_suffix()
                .as_bytes()
                .cmp(other.front_suffix().as_bytes()),
        )
    }
}

impl Block<'_> {
    fn front_suffix(&self) -> &str {
        &self.text[self.front_index as usize..]
    }

    fn next(mut self) -> Option<Self> {
        match self.reader.read_u32::<NativeEndian>() {
            Ok(x) => Some(Self {
                front_index: x,
                ..self
            }),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => None,
            _ => unreachable!(),
        }
    }
}

fn calc_tail_len(text: &str, pat: &str) -> Option<usize> {
    let mut occ_pos = 0;
    let mut prefix_len = 1;
    while !pat.is_char_boundary(prefix_len) {
        prefix_len += 1;
    }
    while let Some(i) = &text[occ_pos..].find(&pat[..prefix_len]) {
        if prefix_len == pat.len() {
            return None;
        }
        occ_pos += *i;
        prefix_len += 1;
        while !pat.is_char_boundary(prefix_len) {
            prefix_len += 1;
        }
    }
    Some(prefix_len)
}

fn sort_blocks(text: &str, block_size: u32) -> Result<BinaryHeap<Reverse<Block>>> {
    let mut heap = BinaryHeap::new();

    let mut begin = 0;
    while begin < text.len() {
        let end = {
            let mut i = (begin + block_size as usize).min(text.len());
            while !text.is_char_boundary(i) {
                i += 1;
            }
            i
        };
        let (end, end_with_tail) = if end == text.len() {
            (end, end)
        } else {
            match calc_tail_len(&text[begin..end], &text[end..]) {
                Some(l) => (end, end + l),
                None => (text.len(), text.len()),
            }
        };

        let file = NamedTempFile::new()?;
        {
            let writer = BufWriter::new(&file);
            build_suffix_array_to_writer(&text[begin..end_with_tail], end - begin, writer)?;
        }

        let block = Block {
            text: &text[begin..],
            reader: BufReader::new(file.reopen()?),
            begin,
            front_index: 0,
        };
        heap.push(Reverse(block.next().unwrap()));

        begin = end;
    }

    Ok(heap)
}

fn merge_blocks_to_writer<W: io::Write>(
    mut heap: BinaryHeap<Reverse<Block>>,
    mut writer: W,
) -> Result<()> {
    while let Some(Reverse(block)) = heap.pop() {
        let idx = block.front_index + block.begin as u32;
        writer.write_u32::<NativeEndian>(idx)?;

        if let Some(next) = block.next() {
            heap.push(Reverse(next));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{Index, IndexBuilder};
    use itertools::Itertools;

    fn check_suffix_array(text: &str, suffix_array: &[u32]) {
        let actual = itertools::sorted(suffix_array.iter()).map(|x| *x as usize);
        let expected = (0..text.len()).filter(|&x| text.is_char_boundary(x as usize));
        assert!(actual.eq(expected));

        let sorted = suffix_array
            .iter()
            .tuple_windows()
            .all(|(a, b)| &text[*a as usize..] < &text[*b as usize..]);
        assert!(sorted);
    }

    #[quickcheck]
    fn build_in_memory(text: String) {
        let index = IndexBuilder::new(&text).build_in_memory().unwrap();
        check_suffix_array(&text, &index.suffix_array());
    }

    #[quickcheck]
    fn build_to_writer_with_blocks(text: String, block_size: u32) {
        let mut buf = Vec::new();
        IndexBuilder::new(&text)
            .block_size(block_size)
            .build_to_writer(&mut buf)
            .unwrap();
        let index = Index::from_bytes(&text, &buf).unwrap();
        check_suffix_array(&text, &index.suffix_array());
    }

    #[quickcheck]
    fn build_to_writer_without_blocks(text: String) {
        let mut buf = Vec::new();
        IndexBuilder::new(&text)
            .block_size(u32::MAX)
            .build_to_writer(&mut buf)
            .unwrap();
        let index = Index::from_bytes(&text, &buf).unwrap();
        check_suffix_array(&text, &index.suffix_array());
    }
}
