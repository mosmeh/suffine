use ansi_term::{Color, Style};
use anyhow::Result;
use clap::{clap_app, value_t, ArgMatches};
use memmap::Mmap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use suffine::{Index, IndexBuilder, MultiDocIndexBuilder};

fn get_filenames(matches: &ArgMatches) -> Result<(PathBuf, PathBuf)> {
    let text_filename = value_t!(matches, "FILE", PathBuf)?;
    let index_filename = value_t!(matches, "index", PathBuf).unwrap_or_else(|_| {
        let mut index_filename = text_filename.clone();
        index_filename.set_extension("suffine-index");
        index_filename
    });

    Ok((text_filename, index_filename))
}

fn open_and_map<P: AsRef<Path>>(path: P) -> Result<Mmap> {
    let file = File::open(&path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    Ok(mmap)
}

fn index(matches: &ArgMatches) -> Result<()> {
    let (text_filename, index_filename) = get_filenames(&matches)?;
    let block_size = value_t!(matches, "block", u32)
        .map(|x| x * 1024 * 1024)
        .unwrap_or(u32::MAX);

    let text_mmap = open_and_map(&text_filename)?;
    let text = unsafe { std::str::from_utf8_unchecked(&text_mmap) };
    let index_file = File::create(&index_filename)?;
    let mut writer = BufWriter::new(index_file);

    IndexBuilder::new(&text)
        .block_size(block_size)
        .build_to_writer_native_endian(&mut writer)?;

    writer.flush()?;

    Ok(())
}

fn search(matches: &ArgMatches) -> Result<()> {
    let (text_filename, index_filename) = get_filenames(&matches)?;
    let query = value_t!(matches, "QUERY", String)?;
    let delimiter = value_t!(matches, "delimiter", String).unwrap_or_else(|_| "\n".to_string());
    let count = value_t!(matches, "count", usize).unwrap_or(usize::MAX);

    let text_mmap = open_and_map(&text_filename)?;
    let text = unsafe { std::str::from_utf8_unchecked(&text_mmap) };
    let index_mmap = open_and_map(&index_filename)?;

    let index = Index::from_bytes(text, &index_mmap)?;
    let multi_doc_index = MultiDocIndexBuilder::new(index)
        .delimiter(&delimiter)
        .build()?;

    let highlighted = Style::new().bold().fg(Color::Green);

    for (doc_id, pos) in multi_doc_index.find_positions(&query).iter().take(count) {
        if let Some(doc_text) = multi_doc_index.doc(*doc_id) {
            let pos = *pos as usize;
            println!(
                "{}{}{}",
                &doc_text[..pos],
                highlighted.paint(&query),
                &doc_text[pos + query.len()..],
            );
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let matches = clap_app!(suffine =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: env!("CARGO_PKG_AUTHORS"))
        (about: env!("CARGO_PKG_DESCRIPTION"))
        (@setting SubcommandRequiredElseHelp)
        (@subcommand index =>
            (@arg FILE: * "File containing the text to index")
            (@arg index: -i --index +takes_value "Suffine index filepath")
            (@arg block: -b --block +takes_value "Block size in MB. By default index is built in single large block")
        )
        (@subcommand search =>
            (@arg FILE: * "File containing the text")
            (@arg QUERY: * -q --query +takes_value "Query string")
            (@arg index: -i --index +takes_value "Suffine index filepath")
            (@arg delimiter: -d --delimiter +takes_value "String used to separate items. Defaults to newline character")
            (@arg count: -n +takes_value "Outputs first <count> hits")
        )
    )
    .get_matches();

    match matches.subcommand() {
        ("index", Some(m)) => index(m)?,
        ("search", Some(m)) => search(m)?,
        _ => unreachable!(),
    };

    Ok(())
}
