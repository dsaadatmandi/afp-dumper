mod structured_field_identifiers;

use log::info;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use structured_field_identifiers::keyword_store;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let arg1 = &args[1];

    // opens file in read only - alternatives present for reading entire file
    let mut f = File::open(arg1)?;
    // construct new array
    let mut buffer = Vec::new();

    // read all of file into array
    f.read_to_end(&mut buffer)?;

    /*
    NEW IMPLEMENTATON:
    change implementation from read_to_end to read
    keep last buffer when reading next
    new loop logic:
    while loop to read entire file
    1. look for start doc
    2. look for start page and end doc
    3. look for structured fields and end page
    4. back to 2

    able to skip 3 if looking for specific page, only count start page
     */

    // get keywords to look for
    let keywords = keyword_store()?;

    // new hashmap to store results
    let mut lookup_map: HashMap<&str, Vec<u32>> = HashMap::default();

    // create size 3 bytes window slices
    for (i, w) in buffer.windows(3).enumerate() {
        // iterate over keywords store hashmap
        for (key, value) in &keywords {
            // byte sequence found insert index into hashmap
            if w == value {
                lookup_map
                    .entry(key)
                    .or_default()
                    .push(i.try_into().unwrap())
            }
        }
    }

    if lookup_map["PageStart"].len() != lookup_map["PageEnd"].len() {
        panic!("Uneven PageStart and PageEnd found. Error in file.")
    } else {
        info!("PageStart count match PageEnd count");
    }

    println!(
        "Found {} pages\nSelect page:",
        lookup_map["PageStart"].len()
    );
    let g: usize = rprompt::read_reply().unwrap().parse()?;

    println!(
        "Start byte: {} \nEnd byte: {}",
        lookup_map["PageStart"][g - 1],
        lookup_map["PageEnd"][g - 1]
    );

    for (key, value) in lookup_map {
        println!("{}: {:?}", key, value);
    }

    Ok(())
}

pub fn get_pages(
    start_page: &u8,
    end_page: &u8,
    mut indexed_elements: HashMap<&str, Vec<u8>>,
) -> Result<HashMap<&str, Vec<u8>>, Box<dyn Error>> {
    let index_start = start_page - 1;
    let index_end = end_page - 1;
    let start_all = indexed_elements["PageStart"].get(index_start..index_end);

    indexed_elements.entry("PageStart").or_default().insert(
        "PageStart",
        indexed_elements["PageStart"][start_page - 1..end_page - 1].to_vec(),
    );
    indexed_elements["PageStart"][1..3].to_vec();

    Ok((indexed_elements))
}
