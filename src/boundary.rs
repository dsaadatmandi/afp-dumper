use aho_corasick::AhoCorasick;
use std::io;

use crate::patterns::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfpBoundary {
    BeginDocument,
    EndDocument,
    BeginPageGroup,
    EndPageGroup,
    BeginPage,
    EndPage,
}

pub struct BoundaryDetector {
    ac: AhoCorasick,
}

impl BoundaryDetector {
    pub fn new() -> Self {
        let patterns = &[
            DOC_START, DOC_END, PG_START, PG_END, PAGE_START, PAGE_END,
        ];
        let ac = AhoCorasick::new(patterns).expect("Failed to create Aho-Corasick automaton");
        Self { ac }
    }

    pub fn detect<R: io::Read>(&self, reader: R) -> Vec<(AfpBoundary, u64)> {
        let mut results = Vec::with_capacity(1024);
        for mat in self.ac.stream_find_iter(reader) {
            let mat = match mat {
                Ok(m) => m,
                Err(_) => continue,
            };
            let boundary = match mat.pattern().as_usize() {
                0 => AfpBoundary::BeginDocument,
                1 => AfpBoundary::EndDocument,
                2 => AfpBoundary::BeginPageGroup,
                3 => AfpBoundary::EndPageGroup,
                4 => AfpBoundary::BeginPage,
                5 => AfpBoundary::EndPage,
                _ => continue,
            };
            results.push((boundary, mat.start() as u64));
        }
        results
    }
}
