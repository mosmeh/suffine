use ansi_term::{Colour, Style};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use suffine::IndexBuilder;

const OFFSET: usize = 50;
const TOP_K_ARTICLES: usize = 10;
const FIRST_N_OCCURRENCES: usize = 3;

#[derive(Serialize, Deserialize)]
struct Article {
    id: String,
    title: String,
    url: String,
    text: String,
}

fn main() {
    let file = File::open("articles.json").unwrap();

    let mut doc_titles: Vec<String> = Vec::new();
    let mut index_builder: IndexBuilder = IndexBuilder::new();
    for line in BufReader::new(file).lines() {
        if let Ok(article) = serde_json::from_str::<Article>(&line.unwrap()) {
            doc_titles.push(article.title);
            index_builder.add(&article.text);
        }
    }

    let index = index_builder.build();

    for query in BufReader::new(io::stdin()).lines() {
        let query = query.unwrap();

        let mut map: Vec<(usize, Vec<usize>)> = index
            .search(&query)
            .into_iter()
            .into_group_map()
            .into_iter()
            .collect();
        map.sort_by_cached_key(|(_, p)| p.len());

        for (doc_id, positions) in map.into_iter().rev().take(TOP_K_ARTICLES) {
            println!("{}", Style::new().bold().paint(&doc_titles[doc_id]));

            let doc_text = index.document(doc_id).unwrap();

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
    }
}
