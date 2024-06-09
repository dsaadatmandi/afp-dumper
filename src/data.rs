use std::collections::HashMap;
use std::error::Error;

pub fn keyword_store() -> Result<HashMap<String, Vec<u8>>, Box<dyn Error>> {
    let mut keywords: HashMap<String, Vec<u8>> = HashMap::new();

    let page_start = hex::decode("D3A8AF")?;
    let page_end = hex::decode("D3A9AF")?;

    keywords.insert("PageStart".to_string(), page_start);
    keywords.insert("PageEnd".to_string(), page_end);

    Ok(keywords)
}