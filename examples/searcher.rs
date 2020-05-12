use ansi_term::{Colour, Style};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use suffine::Index;

const OFFSET: usize = 50;
const TOP_K_ARTICLES: usize = 10;
const FIRST_N_OCCURRENCES: usize = 3;

#[derive(Serialize, Deserialize)]
struct Database {
    index: Index,
    doc_titles: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_filename = env::args().nth(1).ok_or("database filename required")?;

    let reader = BufReader::new(File::open(db_filename)?);
    let db: Database = bincode::deserialize_from(reader)?;

    print!("> ");
    io::stdout().flush()?;
    for query in BufReader::new(io::stdin()).lines() {
        let query = query?;

        let mut map: Vec<(usize, Vec<usize>)> = db
            .index
            .search(&query)
            .into_iter()
            .into_group_map()
            .into_iter()
            .collect();
        map.sort_by_cached_key(|(_, p)| p.len());

        for (doc_id, positions) in map.into_iter().rev().take(TOP_K_ARTICLES) {
            println!("{}", Style::new().bold().paint(&db.doc_titles[doc_id]));

            let doc_text = db.index.document(doc_id).ok_or("document not found")?;

            for pos in positions.iter().take(FIRST_N_OCCURRENCES) {
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
                    "{}{}{}{}{}",
                    if begin > 0 { "..." } else { "" },
                    &doc_text[begin..*pos].replace("\n", ""),
                    Style::new().bold().fg(Colour::Green).paint(&query),
                    &doc_text[pos + query.len()..end].replace("\n", ""),
                    if end < doc_text.len() { "..." } else { "" }
                );
            }
        }

        println!();
        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
