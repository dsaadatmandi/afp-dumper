use std::io::Read;
use std::fs::File;
use std::error::Error;
use std::collections::HashMap;
use log::info;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    // opens file in read only - alternatives present for reading entire file
    let mut f = File::open("./data/01_Health_Coverage.afp")?;
    // construct new array
    let mut buffer  = Vec::new();

    // read all of file into array
    // why mutable reference
    f.read_to_end(&mut buffer)?;

    // save reference to hex decode byte string to var
    let page_start = &hex::decode("D3A8AF")?;
    let page_end = &hex::decode("D3A9AF")?;

    // new hashmap to store results
    let mut lookup_map: HashMap<String, Vec<usize>> = HashMap::default();

    // create size 3 bytes window slices
    for (i, w) in buffer.windows(3).enumerate() {
        println!("{:?}", w);
        if w == page_start {
            // insert to hash map byte location of matched string
            lookup_map.entry("PageStart".to_string()).or_default().push(i);
        };
        if w == page_end {
            // insert to hash map byte location of matched string
            lookup_map.entry("PageEnd".to_string()).or_default().push(i);
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
