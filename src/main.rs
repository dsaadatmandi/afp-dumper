mod pattern_handlers;
mod structured_field_identifiers;

use aho_corasick::AhoCorasick;
use pattern_handlers::*;
use std::error::Error;
use std::fs::{File};
use std::io::{BufReader};
use structured_field_identifiers::*;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let arg1: &str = "/Users/milad/code/afp-dumper/data/01_Health_Coverage.afp";

    let f = File::open(arg1)?;

    let breader = BufReader::new(f);

    // let data = std::fs::read(arg1)?;

    let patterns = &[DOC_START, DOC_END, PAGE_START, PAGE_END, SF_START, SF_END, NOT_SURE, TEXT_START, TEXT_DATA];

    let ac = AhoCorasick::new(patterns).expect("Failed to create Aho-Corasick automaton");

    let mut page_count: usize = 0;

    for mat in ac.stream_find_iter(breader) {
        let mat = mat.expect("Could not unpack value from AC iterator");

        match mat.pattern().as_usize() {
            0 => start_document_handler(mat.start()),
            1 => end_document_handler(mat.start()),
            2 => start_page_handler(mat.start(), &mut page_count),
            3 => end_page_handler(mat.start()),
            4 => start_structured_field_handler(mat.start()),
            5 => end_structured_field_handler(mat.start()),
            6 => println!("Not Sure at {}", mat.start()),
            7 => println!("Text Start at {}", mat.start()),
            8 => println!("Text Data at {}", mat.start()),
            _ => unreachable!(),
        }
    }

    println!("Total Pages: {page_count}");

    Ok(())
}
