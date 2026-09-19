use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use crate::patterns;
use crate::record::SfRecord;
use crate::state::Position;
use crate::text::{self, EncodingMode};
use crate::triplet::{self, FQN_ATTRIBUTE_NAME, T_ATTR_VALUE, T_FQN};

// each line is the position and the structured field's data, nothing else.
// introducer fields, declared code pages, qualifiers and any other triplet
// that describes the field rather than carrying its content are not printed
pub struct Inspector {
    sink: Box<dyn Write>,
    encoding: EncodingMode,
    count: usize,
}

impl Inspector {
    pub fn new(out: Option<&Path>, encoding: EncodingMode) -> io::Result<Self> {
        let sink: Box<dyn Write> = match out {
            Some(path) => Box::new(BufWriter::new(File::create(path)?)),
            None => Box::new(BufWriter::new(io::stdout())),
        };
        Ok(Self {
            sink,
            encoding,
            count: 0,
        })
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }

    pub fn record(&mut self, pos: Position, rec: &SfRecord) -> io::Result<()> {
        match &rec.type_id[..] {
            id if id == patterns::NOP_SF => self.nop(pos, rec),
            id if id == patterns::TLE_SF => self.tle(pos, rec),
            _ => Ok(()),
        }
    }

    // NOP data is UndfData — free-form bytes with no architectural definition
    // — so the whole body is the content
    fn nop(&mut self, pos: Position, rec: &SfRecord) -> io::Result<()> {
        let (text, _) = text::decode(&rec.data, self.encoding);
        let body = text.trim_end();

        if body.is_empty() {
            writeln!(self.sink, "{pos}: NOP")?;
        } else {
            // one output line per line of content, each keeping the position
            for line in body.split('\n') {
                writeln!(
                    self.sink,
                    "{pos}: NOP {}",
                    text::escape(line.trim_end_matches('\r'))
                )?;
            }
        }

        self.count += 1;
        Ok(())
    }

    // a TLE's content is the attribute it tags the page or page group with:
    // the name from the X'02' FQN and the value from X'36'
    fn tle(&mut self, pos: Position, rec: &SfRecord) -> io::Result<()> {
        let mut name = String::new();
        let mut value = String::new();

        for t in triplet::triplets(&rec.data) {
            match t.id {
                T_FQN => {
                    if let Some(fqn) = t.as_fqn() {
                        if fqn.fqn_type == FQN_ATTRIBUTE_NAME {
                            name = self.render(fqn.name);
                        }
                    }
                }
                T_ATTR_VALUE => {
                    if let Some(raw) = t.as_attr_value() {
                        value = self.render(raw);
                    }
                }
                _ => {}
            }
        }

        writeln!(self.sink, "{pos}: TLE {name} = {value}")?;
        self.count += 1;
        Ok(())
    }

    fn render(&self, bytes: &[u8]) -> String {
        let (text, _) = text::decode(bytes, self.encoding);
        text::escape(text.trim_end())
    }
}
