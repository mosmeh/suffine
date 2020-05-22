use ansi_term::{Colour, Style};
use memmap::Mmap;
use std::fs::File;
use std::io::{self, BufRead, Write};
use suffine::{Index, Result};

const FIRST_N_OCCURRENCES: usize = 10;
const OFFSET: usize = 50;

fn main() -> Result<()> {
    let text_filename = std::env::args().nth(1).ok_or("text filename required")?;
    let index_filename = std::env::args().nth(2).ok_or("index filename required")?;

    let text_mmap = unsafe { Mmap::map(&File::open(text_filename)?)? };
    let text = unsafe { std::str::from_utf8_unchecked(&text_mmap) };

    let index_mmap = unsafe { Mmap::map(&File::open(index_filename)?)? };
    let index = Index::from_bytes(&text, &index_mmap)?;

    let highlighted = Style::new().bold().fg(Colour::Green);

    print!("> ");
    io::stdout().flush()?;

    for query in io::stdin().lock().lines() {
        let query = query?;

        for p in index
            .find_positions(&query)
            .iter()
            .take(FIRST_N_OCCURRENCES)
            .map(|x| *x as usize)
        {
            let mut begin = p.saturating_sub(OFFSET);
            while !text.is_char_boundary(begin) {
                begin -= 1;
            }

            let mut end = (p + query.len()).saturating_add(OFFSET).min(text.len());
            while !text.is_char_boundary(end) {
                end += 1;
            }

            println!(
                "{}{}{}",
                &text[begin..p].replace("\n", " "),
                highlighted.paint(&query),
                &text[p + query.len()..end].replace("\n", " "),
            );
        }

        print!("\n> ");
        io::stdout().flush()?;
    }

    Ok(())
}
