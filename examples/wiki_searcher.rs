use ansi_term::{Colour, Style};
use itertools::Itertools;
use memmap::Mmap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use suffine::{Index, MultiDocIndexBuilder, Result};

const FIRST_N_OCCURRENCES: usize = 3;
const TOP_K_ARTICLES: usize = 10;
const OFFSET: usize = 50;

fn main() -> Result<()> {
    let index_filename_prefix = std::env::args()
        .nth(1)
        .ok_or("index filename prefix required")?;

    let text_file = File::open(format!("{}.text", index_filename_prefix))?;
    let text_mmap = unsafe { Mmap::map(&text_file)? };
    let text = std::str::from_utf8(&text_mmap)?;

    let index_file = File::open(format!("{}.index", index_filename_prefix))?;
    let index_mmap = unsafe { Mmap::map(&index_file)? };

    let index = Index::from_bytes(text, &index_mmap)?;
    let multi_doc_index = MultiDocIndexBuilder::new(&index).delimiter("\0").build();

    let title_file = File::open(format!("{}.title", index_filename_prefix))?;
    let title_reader = BufReader::new(title_file);
    let titles: Vec<String> = bincode::deserialize_from(title_reader)?;

    let highlighted = Style::new().bold().fg(Colour::Green);

    print!("> ");
    io::stdout().flush()?;

    for query in io::stdin().lock().lines() {
        let query = query?;

        let hits = multi_doc_index
            .find_positions(&query)
            .into_iter()
            .into_group_map()
            .into_iter()
            .sorted_by_key(|(_, p)| p.len())
            .rev()
            .take(TOP_K_ARTICLES);

        for (doc_id, positions) in hits {
            println!("{}", highlighted.paint(&titles[doc_id as usize]));

            let doc_text = multi_doc_index.doc(doc_id).ok_or("document not found")?;

            for pos in positions.iter().take(FIRST_N_OCCURRENCES) {
                let pos = *pos as usize;

                let mut begin = pos.saturating_sub(OFFSET);
                while !doc_text.is_char_boundary(begin) {
                    begin -= 1;
                }

                let mut end = (pos + query.len())
                    .saturating_add(OFFSET)
                    .min(doc_text.len());
                while !doc_text.is_char_boundary(end) {
                    end += 1;
                }

                println!(
                    "{}{}{}",
                    &doc_text[begin..pos].replace("\n", " "),
                    highlighted.paint(&query),
                    &doc_text[pos + query.len()..end].replace("\n", " "),
                );
            }
        }

        print!("\n> ");
        io::stdout().flush()?;
    }

    Ok(())
}
