use anyhow::{anyhow, Result};
use memmap::Mmap;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use suffine::IndexBuilder;

#[derive(Serialize, Deserialize)]
struct Article {
    id: String,
    title: String,
    url: String,
    text: String,
}

fn main() -> Result<()> {
    let json_filename = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("input json filename required"))?;
    let index_filename_prefix = std::env::args()
        .nth(2)
        .ok_or_else(|| anyhow!("output filename prefix required"))?;

    let text_filename = format!("{}.text", index_filename_prefix);
    {
        println!("Preprocessing...");
        let json_file = File::open(json_filename)?;

        let text_file = File::create(&text_filename)?;
        let mut text_writer = BufWriter::new(&text_file);

        let mut titles: Vec<String> = Vec::new();
        for line in BufReader::new(json_file).lines() {
            if let Ok(article) = serde_json::from_str::<Article>(&line?) {
                titles.push(article.title);
                write!(text_writer, "{}\0", article.text)?;
            }
        }

        text_writer.flush()?;

        let title_file = File::create(format!("{}.title", index_filename_prefix))?;
        let mut title_writer = BufWriter::new(title_file);
        bincode::serialize_into(&mut title_writer, &titles)?;
        title_writer.flush()?;
    }

    {
        println!("Building index...");
        let text_file = File::open(&text_filename)?;
        let text_mmap = unsafe { Mmap::map(&text_file)? };
        let text = unsafe { std::str::from_utf8_unchecked(&text_mmap) };

        let index_file = File::create(format!("{}.index", index_filename_prefix))?;
        let mut index_writer = BufWriter::new(index_file);

        IndexBuilder::new(text)
            .block_size(1024 * 1024 * 1024) // 1G
            .build_to_writer_native_endian(&mut index_writer)?;

        index_writer.flush()?;
    }

    Ok(())
}
