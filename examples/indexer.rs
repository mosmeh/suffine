use memmap::Mmap;
use std::fs::File;
use std::io::{BufWriter, Write};
use suffine::{IndexBuilder, Result};

fn main() -> Result<()> {
    let text_filename = std::env::args().nth(1).ok_or("input filename required")?;
    let index_filename = std::env::args().nth(2).ok_or("output filename required")?;

    let text_mmap = unsafe { Mmap::map(&File::open(text_filename)?)? };
    let text = unsafe { std::str::from_utf8_unchecked(&text_mmap) };

    let mut writer = BufWriter::new(File::create(index_filename)?);
    IndexBuilder::new(&text)
        .block_size(1024 * 1024 * 1024) // 1G
        .build_to_writer(&mut writer)?;
    writer.flush()?;

    Ok(())
}
