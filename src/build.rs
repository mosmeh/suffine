use crate::Result;
use byteorder::{ByteOrder, NativeEndian, ReadBytesExt, WriteBytesExt};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use suffix::SuffixTable;
use tempfile::NamedTempFile;

#[derive(PartialEq, Debug)]
pub struct VecWrapper<T>(pub Vec<T>);

pub trait IntBuffer<T, O: ByteOrder> {
    fn write(&mut self, n: T) -> Result<()>;
}

impl<T> IntBuffer<T, NativeEndian> for &mut VecWrapper<T> {
    fn write(&mut self, n: T) -> Result<()> {
        self.0.push(n);
        Ok(())
    }
}

impl<W, O> IntBuffer<u32, O> for W
where
    W: Write,
    O: ByteOrder,
{
    fn write(&mut self, n: u32) -> Result<()> {
        self.write_u32::<O>(n).map_err(Into::into)
    }
}

pub fn build_suffix_array<B, O>(text: &str, block_size: u32, mut buffer: B) -> Result<usize>
where
    B: IntBuffer<u32, O>,
    O: ByteOrder,
{
    match text.len() {
        0 => return Ok(0),
        1 => {
            buffer.write(0)?;
            return Ok(1);
        }
        _ => (),
    }

    if text.len() <= block_size as usize {
        build_suffix_array_in_memory(text, text.len(), buffer)
    } else {
        let heap = sort_blocks(text, block_size)?;
        merge_blocks(heap, buffer)
    }
}

fn build_suffix_array_in_memory<B, O>(text: &str, len: usize, mut buffer: B) -> Result<usize>
where
    B: IntBuffer<u32, O>,
    O: ByteOrder,
{
    let st = SuffixTable::new(text);
    let sa = st
        .table()
        .iter()
        .filter(|&&x| (x as usize) < len && text.is_char_boundary(x as usize));
    let mut num_written = 0;
    for x in sa {
        buffer.write(*x)?;
        num_written += 1;
    }

    Ok(num_written)
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
            let mut writer = BufWriter::new(&file);
            build_suffix_array_in_memory::<_, NativeEndian>(
                &text[begin..end_with_tail],
                end - begin,
                &mut writer,
            )?;
            writer.flush()?;
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

fn merge_blocks<B, O>(mut heap: BinaryHeap<Reverse<Block>>, mut buffer: B) -> Result<usize>
where
    B: IntBuffer<u32, O>,
    O: ByteOrder,
{
    let mut num_written = 0;
    while let Some(Reverse(block)) = heap.pop() {
        let idx = block.front_index + block.begin as u32;
        buffer.write(idx)?;
        num_written += 1;

        if let Some(next) = block.next() {
            heap.push(Reverse(next));
        }
    }

    Ok(num_written)
}

#[cfg(test)]
mod tests {
    use crate::build::{build_suffix_array, VecWrapper};
    use itertools::Itertools;
    use quickcheck::TestResult;

    fn check_suffix_array(text: &str, suffix_array: &[u32]) {
        let actual = suffix_array.iter().sorted().map(|x| *x as usize);
        let expected = (0..text.len()).filter(|&x| text.is_char_boundary(x as usize));
        assert!(actual.eq(expected));

        let sorted = suffix_array
            .iter()
            .tuple_windows()
            .all(|(a, b)| text[*a as usize..] < text[*b as usize..]);
        assert!(sorted);
    }

    #[quickcheck]
    fn build(text: String, block_size: u32) -> TestResult {
        if block_size == 0 {
            return TestResult::discard();
        }

        let mut buf_a = VecWrapper(Vec::new());
        build_suffix_array(&text, block_size, &mut buf_a).unwrap();

        let mut buf_b = VecWrapper(Vec::new());
        build_suffix_array(&text, u32::MAX, &mut buf_b).unwrap();

        assert_eq!(buf_a, buf_b);

        check_suffix_array(&text, &buf_a.0);

        TestResult::passed()
    }
}
