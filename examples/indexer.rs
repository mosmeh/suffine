use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use suffine::{Index, IndexBuilder};

#[derive(Serialize, Deserialize)]
struct Article {
    id: String,
    title: String,
    url: String,
    text: String,
}

#[derive(Serialize, Deserialize)]
struct Database {
    index: Index,
    doc_titles: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let article_filename = env::args()
        .nth(1)
        .ok_or("filename containing articles required")?;
    let db_filename = env::args().nth(2).ok_or("database filename required")?;

    println!("Reading...");
    let file = File::open(article_filename)?;

    let mut doc_titles: Vec<String> = Vec::new();
    let mut index_builder: IndexBuilder = IndexBuilder::new();

    for line in BufReader::new(file).lines() {
        if let Ok(article) = serde_json::from_str::<Article>(&line?) {
            doc_titles.push(article.title);
            index_builder.add(&article.text);
        }
    }

    println!("Building index...");
    let db = Database {
        index: index_builder.build(),
        doc_titles,
    };

    println!("Writing...");
    File::create(db_filename)?.write_all(&bincode::serialize(&db)?)?;

    Ok(())
}
