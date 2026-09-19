use aho_corasick::AhoCorasick;
use std::io;

use crate::features::Features;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    Boundary(AfpBoundary),
    Nop,
    Tle,
}

#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub kind: HitKind,
    pub offset: u64,
}

pub struct BoundaryDetector {
    ac: AhoCorasick,
    // parallel to the pattern list, so pattern index maps to the right kind
    // whichever optional patterns were compiled in
    kinds: Vec<HitKind>,
}

impl BoundaryDetector {
    pub fn new(features: &Features) -> Self {
        let mut patterns: Vec<&[u8]> = vec![
            DOC_START, DOC_END, PG_START, PG_END, PAGE_START, PAGE_END,
        ];
        let mut kinds = vec![
            HitKind::Boundary(AfpBoundary::BeginDocument),
            HitKind::Boundary(AfpBoundary::EndDocument),
            HitKind::Boundary(AfpBoundary::BeginPageGroup),
            HitKind::Boundary(AfpBoundary::EndPageGroup),
            HitKind::Boundary(AfpBoundary::BeginPage),
            HitKind::Boundary(AfpBoundary::EndPage),
        ];

        if features.dump_nop {
            patterns.push(NOP_SF);
            kinds.push(HitKind::Nop);
        }
        if features.dump_tle {
            patterns.push(TLE_SF);
            kinds.push(HitKind::Tle);
        }

        let ac = AhoCorasick::new(&patterns).expect("Failed to create Aho-Corasick automaton");
        Self { ac, kinds }
    }

    pub fn detect<R: io::Read>(&self, reader: R) -> Vec<Hit> {
        let mut results = Vec::with_capacity(1024);
        for mat in self.ac.stream_find_iter(reader) {
            let mat = match mat {
                Ok(m) => m,
                Err(_) => continue,
            };
            let Some(&kind) = self.kinds.get(mat.pattern().as_usize()) else {
                continue;
            };
            results.push(Hit {
                kind,
                offset: mat.start() as u64,
            });
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_patterns_are_absent_unless_requested() {
        let blob = [DOC_START, NOP_SF, TLE_SF, DOC_END].concat();

        let indexing_only = BoundaryDetector::new(&Features::default());
        let kinds: Vec<_> = indexing_only
            .detect(&blob[..])
            .iter()
            .map(|h| h.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                HitKind::Boundary(AfpBoundary::BeginDocument),
                HitKind::Boundary(AfpBoundary::EndDocument),
            ]
        );
    }

    #[test]
    fn pattern_index_stays_aligned_for_any_subset() {
        let blob = [DOC_START, NOP_SF, TLE_SF, DOC_END].concat();

        // tle only: the tle pattern takes the slot nop would have had
        let tle_only = BoundaryDetector::new(&Features::new(None, false, true));
        let kinds: Vec<_> = tle_only.detect(&blob[..]).iter().map(|h| h.kind).collect();
        assert_eq!(
            kinds,
            vec![
                HitKind::Boundary(AfpBoundary::BeginDocument),
                HitKind::Tle,
                HitKind::Boundary(AfpBoundary::EndDocument),
            ]
        );

        let both = BoundaryDetector::new(&Features::new(None, true, true));
        let kinds: Vec<_> = both.detect(&blob[..]).iter().map(|h| h.kind).collect();
        assert_eq!(
            kinds,
            vec![
                HitKind::Boundary(AfpBoundary::BeginDocument),
                HitKind::Nop,
                HitKind::Tle,
                HitKind::Boundary(AfpBoundary::EndDocument),
            ]
        );
    }

    #[test]
    fn hits_come_back_in_offset_order() {
        let blob = [DOC_START, PAGE_START, NOP_SF, PAGE_END, DOC_END].concat();
        let hits = BoundaryDetector::new(&Features::new(None, true, false)).detect(&blob[..]);
        let offsets: Vec<_> = hits.iter().map(|h| h.offset).collect();
        assert_eq!(offsets, vec![0, 3, 6, 9, 12]);
    }
}
