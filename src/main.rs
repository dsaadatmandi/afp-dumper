mod structured_field_identifiers;

use std::error::Error;
use std::fs::File;
use aho_corasick::AhoCorasick;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    // let args: Vec<String> = env::args().collect();
    // let arg1 = &args[1];
    let arg1: &str = "/Users/milad/code/afp-dumper/data/01_Health_Coverage.afp";

    // opens file in read only - alternatives present for reading entire file
    let f = File::open(arg1)?;
    // construct new array
    // let mut buffer = Vec::new();

    let breader = std::io::BufReader::new(f);

    // read all of file into array
    // f.read_to_end(&mut buffer)?;

    let patterns = &[
        b"\x5A\x00\x00", // Start of Document
        b"\x5A\xFF\xFF", // End of Document
        b"\x5B\x00\x00", // Start of Page
        b"\x5B\xFF\xFF", // End of Page
        b"\xD3\x00\x00", // Begin Structured Field
        b"\xD3\xFF\xFF", // End Structured Field
    ];

    let ac = AhoCorasick::new(patterns)
    .expect("Failed to create Aho-Corasick automaton");

    for mat in ac.stream_find_iter(breader) {
        let mat = mat?;
        println!("{:?}", mat);
    }

    Ok(())


}
