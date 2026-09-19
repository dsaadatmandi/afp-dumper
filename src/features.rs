// which passes run for this invocation. indexing is mandatory — it is what
// produces the document/page counts and the positions every other pass reports
// against — everything else is opt-in from the command line
#[derive(Debug, Clone, Copy)]
pub struct Features {
    pub index: bool,
    pub dump_nop: bool,
    pub dump_tle: bool,
    // splitting is on exactly when a size was given, and carries it
    pub max_size: Option<u64>,
}

impl Features {
    pub fn new(max_size: Option<u64>, dump_nop: bool, dump_tle: bool) -> Self {
        Self {
            index: true,
            dump_nop,
            dump_tle,
            max_size,
        }
    }

    // only when this is true does anything seek back into the file to decode
    // whole records
    pub fn needs_record_decode(&self) -> bool {
        self.dump_nop || self.dump_tle
    }
}

impl Default for Features {
    fn default() -> Self {
        Self::new(None, false, false)
    }
}
