use std::fs;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::chunk::OutputChunk;

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

            for doc in &chunk.documents {
                source_file.seek(SeekFrom::Start(doc.start))?;
                let mut doc_bytes = Read::by_ref(&mut source_file).take(doc.size());
                let n = io::copy(&mut doc_bytes, &mut output)?;
                bytes_written += n;
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
