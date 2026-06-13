pub fn start_page_handler(position: usize, page_count: &mut usize) {
    *page_count += 1;
    println!("Begin Page (BPG) at {position}");
}

pub fn end_page_handler(position: usize) {
    println!("End Page (EPG) at {position}");
}

pub fn start_document_handler(position: usize) {
    println!("Start of Document at {position}");
}

pub fn end_document_handler(position: usize) {
    println!("End of Document at {position}");
}

pub fn start_structured_field_handler(position: usize) {
    println!("Begin Structured Field at {position}");
}

pub fn end_structured_field_handler(position: usize) {
    println!("End Structured Field at {position}");
}
