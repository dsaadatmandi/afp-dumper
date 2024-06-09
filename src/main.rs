use std::io::Read;
use std::fs::File;
use std::error::Error;
use std::collections::HashMap;
use log::info;
use std::env;

mod data;
use data::keyword_store;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let arg1 = &args[1];

    // opens file in read only - alternatives present for reading entire file
    let mut f = File::open(arg1)?;
    // construct new array
    let mut buffer  = Vec::new();

    // read all of file into array
    f.read_to_end(&mut buffer)?;

    // get keywords to look for
    let keywords = keyword_store()?;

    // new hashmap to store results
    let mut lookup_map: HashMap<String, Vec<usize>> = HashMap::default();

    // create size 3 bytes window slices
    for (i, w) in buffer.windows(3).enumerate() {
        // iterate over keywords store hashmap
        for (key, value) in &keywords {
            // byte sequence found insert index into hashmap
            if w == value {
                lookup_map.entry(key.to_string()).or_default().push(i)
            }
        };

    };

    if lookup_map["PageStart"].len() != lookup_map["PageEnd"].len() {
        panic!("Uneven PageStart and PageEnd found. Error in file.")
    } else {
        info!("PageStart count match PageEnd count");
    }

    for (key, value) in lookup_map {
        println!("{}: {:?}", key, value);
    }

    Ok(())
}
