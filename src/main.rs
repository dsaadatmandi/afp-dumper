mod boundary;
mod chunk;
mod patterns;
mod state;
mod writer;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use clap::Parser;
use boundary::BoundaryDetector;
use chunk::{ChunkPlanner, assemble_documents};
use state::StateMachine;
use writer::OutputWriter;

#[derive(Parser)]
#[command(name = "afp-split")]
#[command(about = "Split AFP documents by page group boundary with max output size")]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long, default_value = "output")]
    output_dir: PathBuf,

    #[arg(short, long)]
    max_size: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let file_size = File::open(&args.input)?.metadata()?.len();

    // read file and find all markers
    let detector = BoundaryDetector::new();
    let source = File::open(&args.input)?;
    let boundaries = detector.detect(source);

    let mut state_machine = StateMachine::new();
    let mut events: Vec<state::AfpEvent> = Vec::with_capacity(boundaries.len());

    for (boundary, offset) in boundaries {
        state_machine.feed(boundary, offset, &mut events);
    }
    state_machine.finish(file_size, &mut events);

    let documents = assemble_documents(&events);

    let doc_count = documents.len();
    let group_count: usize = documents
        .iter()
        .map(|d| d.page_groups.len())
        .sum();

    println!(
        "Found {} document(s) containing {} page group(s) across {} page(s)",
        doc_count, group_count, state_machine.page_count
    );

    if doc_count == 0 {
        println!("Nothing to split.");
        return Ok(());
    }

    let mut first_doc_offset: u64 = 0;
    for event in &events {
        if let state::AfpEvent::DocumentStart { offset } = event {
            first_doc_offset = *offset;
            break;
        }
    }

    let preamble_bytes = if first_doc_offset > 0 {
        let mut f = File::open(&args.input)?;
        let mut buf = vec![0u8; first_doc_offset as usize];
        f.read_exact(&mut buf)?;
        buf
    } else {
        Vec::new()
    };

    let chunks = ChunkPlanner::plan(&documents, args.max_size);

    println!("Splitting into {} output file(s)\n", chunks.len());

    let writer = OutputWriter::new(args.output_dir, &args.input);
    writer.write_chunks(&args.input, &preamble_bytes, &chunks)?;

    Ok(())
}
