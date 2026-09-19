mod boundary;
mod chunk;
mod features;
mod inspect;
mod patterns;
mod record;
mod state;
mod text;
mod triplet;
mod writer;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use boundary::{BoundaryDetector, HitKind};
use chunk::{ChunkPlanner, assemble_documents};
use features::Features;
use inspect::Inspector;
use record::RecordReader;
use state::StateMachine;
use text::EncodingMode;
use writer::OutputWriter;

const AFTER_HELP: &str = "\
Indexing always runs: every invocation reports how many documents, page groups
and pages the file holds. The other two passes are off until you ask for them.

  afp-dumper -i input.afp
      index only — no files written, nothing dumped

  afp-dumper -i input.afp -m 1048576
      also split into output/ at page group boundaries

  afp-dumper -i input.afp --dump-all --dump-out tags.txt
      also dump every NOP and TLE, with its position, to tags.txt

Dumping to the terminal stops above --dump-limit records; pass --dump-out to
write them to a file instead.";

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum EncodingArg {
    /// ascii first, ebcdic when the leading bytes are not printable ascii
    Auto,
    Ascii,
    Ebcdic,
}

impl From<EncodingArg> for EncodingMode {
    fn from(a: EncodingArg) -> Self {
        match a {
            EncodingArg::Auto => EncodingMode::Auto,
            EncodingArg::Ascii => EncodingMode::Ascii,
            EncodingArg::Ebcdic => EncodingMode::Ebcdic,
        }
    }
}

#[derive(Parser)]
#[command(name = "afp-dumper")]
#[command(about = "Interact efficiently with large AFP files")]
#[command(after_help = AFTER_HELP)]
struct Args {
    /// Path to the input AFP file
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// Approx max output file size in bytes — splits before reaching it
    #[arg(short, long, value_name = "BYTES", help_heading = "Splitting")]
    max_size: Option<u64>,

    /// Where split files are written
    #[arg(
        short,
        long,
        default_value = "output",
        value_name = "DIR",
        help_heading = "Splitting"
    )]
    output_dir: PathBuf,

    /// Dump the content of every NOP structured field
    #[arg(long, help_heading = "Inspection")]
    dump_nop: bool,

    /// Dump the attribute name and value of every TLE structured field
    #[arg(long, help_heading = "Inspection")]
    dump_tle: bool,

    /// Dump both NOP and TLE
    #[arg(long, help_heading = "Inspection")]
    dump_all: bool,

    /// Write the dump to a file instead of the terminal
    #[arg(long, value_name = "FILE", help_heading = "Inspection")]
    dump_out: Option<PathBuf>,

    /// Records to print to the terminal before refusing
    #[arg(
        long,
        default_value_t = 1000,
        value_name = "N",
        help_heading = "Inspection"
    )]
    dump_limit: usize,

    /// How to decode dumped text
    #[arg(
        long,
        value_enum,
        default_value_t = EncodingArg::Auto,
        value_name = "MODE",
        help_heading = "Inspection"
    )]
    encoding: EncodingArg,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let features = Features::new(
        args.max_size,
        args.dump_nop || args.dump_all,
        args.dump_tle || args.dump_all,
    );

    let file_size = File::open(&args.input)?.metadata()?.len();

    // pass 1: one sequential read finds every marker. NOP/TLE patterns are
    // only in the automaton when a dump was asked for
    let detector = BoundaryDetector::new(&features);
    let hits = detector.detect(File::open(&args.input)?);

    // the count is known before anything is written, so the terminal can be
    // protected without a partial dump
    let dump_total = hits
        .iter()
        .filter(|h| matches!(h.kind, HitKind::Nop | HitKind::Tle))
        .count();

    let mut inspector = match features.needs_record_decode() {
        false => None,
        true if args.dump_out.is_none() && dump_total > args.dump_limit => {
            eprintln!(
                "warning: found {} NOP/TLE record(s), more than the --dump-limit of {}",
                dump_total, args.dump_limit
            );
            eprintln!("         re-run with --dump-out <FILE>, or raise --dump-limit");
            None
        }
        true => Some(Inspector::new(
            args.dump_out.as_deref(),
            args.encoding.into(),
        )?),
    };

    let mut reader = match inspector {
        Some(_) => Some(RecordReader::open(&args.input)?),
        None => None,
    };

    // pass 1b: walk the hits in offset order, so the state machine has already
    // seen every boundary before a NOP/TLE at that offset asks for its position
    let mut state_machine = StateMachine::new();
    let mut events: Vec<state::AfpEvent> = Vec::with_capacity(hits.len());
    let mut rejected = 0usize;

    for hit in &hits {
        match hit.kind {
            HitKind::Boundary(boundary) => {
                state_machine.feed(boundary, hit.offset, &mut events);
            }
            HitKind::Nop | HitKind::Tle => {
                let (Some(inspector), Some(reader)) = (inspector.as_mut(), reader.as_mut()) else {
                    continue;
                };
                // a match can land inside image or text payload bytes, in
                // which case the surrounding framing does not check out
                match reader.read_at(hit.offset)? {
                    Some(rec) => inspector.record(state_machine.position(), &rec)?,
                    None => rejected += 1,
                }
            }
        }
    }
    state_machine.finish(file_size, &mut events);

    if let Some(inspector) = inspector.as_mut() {
        inspector.flush()?;
        eprintln!("Dumped {} NOP/TLE record(s)", inspector.count());
        if rejected > 0 {
            eprintln!(
                "note: skipped {} match(es) that were not structured field boundaries",
                rejected
            );
        }
    }

    let documents = assemble_documents(&events);

    let doc_count = documents.len();
    let group_count: usize = documents.iter().map(|d| d.page_groups.len()).sum();

    if features.index {
        println!(
            "Found {} document(s) containing {} page group(s) across {} page(s)",
            doc_count, group_count, state_machine.page_count
        );
    }

    let Some(max_size) = features.max_size else {
        return Ok(());
    };

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

    let chunks = ChunkPlanner::plan(&documents, max_size);

    println!("Splitting into {} output file(s)\n", chunks.len());

    let writer = OutputWriter::new(args.output_dir, &args.input);
    writer.write_chunks(&args.input, &preamble_bytes, &chunks)?;

    Ok(())
}
