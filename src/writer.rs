use std::fs;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::chunk::OutputChunk;
use crate::patterns::{PF_END, PF_START};

const EDT: &[u8] = b"\xD3\xA9\xA8";

pub struct OutputWriter {
    output_dir: PathBuf,
    input_stem: String,
}

impl OutputWriter {
    pub fn new(output_dir: PathBuf, input_path: &Path) -> Self {
        let input_stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
            .to_string();
        Self {
            output_dir,
            input_stem,
        }
    }

    pub fn write_chunks(
        &self,
        source: &Path,
        preamble: &[u8],
        chunks: &[OutputChunk],
    ) -> io::Result<()> {
        fs::create_dir_all(&self.output_dir)?;

        let needs_epf = preamble
            .windows(PF_START.len())
            .any(|w| w == PF_START);

        let mut source_file = fs::File::open(source)?;

        for (i, chunk) in chunks.iter().enumerate() {
            let output_path = self
                .output_dir
                .join(format!("{}_{:03}.afp", self.input_stem, i));
            let output_file = fs::File::create(&output_path)?;
            let mut output = BufWriter::with_capacity(64 * 1024, output_file);

            let mut bytes_written: u64 = 0;

            if !preamble.is_empty() {
                output.write_all(preamble)?;
                bytes_written += preamble.len() as u64;
            }

            let (prologue_start, prologue_end) = chunk.prologue;
            if prologue_end > prologue_start {
                source_file.seek(SeekFrom::Start(prologue_start))?;
                let mut prologue_bytes =
                    Read::by_ref(&mut source_file).take(prologue_end - prologue_start);
                let n = io::copy(&mut prologue_bytes, &mut output)?;
                bytes_written += n;
            }

            let (pg_start, pg_end) = chunk.pg_range;
            if pg_end > pg_start {
                source_file.seek(SeekFrom::Start(pg_start))?;
                let mut pg_bytes =
                    Read::by_ref(&mut source_file).take(pg_end - pg_start);
                let n = io::copy(&mut pg_bytes, &mut output)?;
                bytes_written += n;
            }

            if chunk.needs_edt {
                output.write_all(EDT)?;
                bytes_written += EDT.len() as u64;
            }

            if needs_epf {
                output.write_all(PF_END)?;
                bytes_written += PF_END.len() as u64;
            }

            output.flush()?;

            println!(
                "{}  ({:.1} KB)",
                output_path.display(),
                bytes_written as f64 / 1024.0
            );
        }

        Ok(())
    }
}
