mod structured_field_identifiers;

use std::error::Error;
use std::fs::File;
use aho_corasick::AhoCorasick;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let arg1: &str = "/Users/milad/code/afp-dumper/data/01_Health_Coverage.afp";

    let f = File::open(arg1)?;

    let breader = std::io::BufReader::new(f);

    let keywords = structured_field_identifiers::keyword_store()?;
    let page_start = &keywords["PageStart"];
    let page_end = &keywords["PageEnd"];

    let patterns = &[
        b"\x5A\x00\x00", // Start of Document
        b"\x5A\xFF\xFF", // End of Document
        page_start.as_slice(), // Begin Page (BPG) - D3A8AF
        page_end.as_slice(),   // End Page (EPG) - D3A9AF
        b"\xD3\x00\x00", // Begin Structured Field
        b"\xD3\xFF\xFF", // End Structured Field
    ];

    let ac = AhoCorasick::new(patterns)
    .expect("Failed to create Aho-Corasick automaton");

    for mat in ac.stream_find_iter(breader) {
        let mat = mat?;
        match mat.pattern().as_usize() {
            0 => println!("Start of Document at {}", mat.start()),
            1 => println!("End of Document at {}", mat.start()),
            2 => println!("Begin Page (BPG) at {}", mat.start()),
            3 => println!("End Page (EPG) at {}", mat.start()),
            4 => println!("Begin Structured Field at {}", mat.start()),
            5 => println!("End Structured Field at {}", mat.start()),
            _ => unreachable!(),
        }
    }

    Ok(())


}
