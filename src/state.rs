use std::fmt;

use crate::boundary::AfpBoundary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfpState {
    OutsideDocument,
    InDocument,
    InPageGroup,
    InPage,
}

#[derive(Debug, Clone)]
pub enum AfpEvent {
    DocumentStart { offset: u64 },
    DocumentEnd { offset: u64 },
    PageGroupStart { offset: u64 },
    PageGroupEnd { offset: u64, needs_eng: bool },
}

// where in the document hierarchy a record sits. group and page indexes are
// per document, so they restart at every BDT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Preamble,
    Document {
        doc: usize,
    },
    PageGroup {
        doc: usize,
        group: usize,
    },
    Page {
        doc: usize,
        group: Option<usize>,
        page: usize,
    },
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Position::Preamble => write!(f, "preamble"),
            Position::Document { doc } => write!(f, "doc {doc}"),
            Position::PageGroup { doc, group } => write!(f, "doc {doc} group {group}"),
            Position::Page {
                doc,
                group: Some(group),
                page,
            } => write!(f, "doc {doc} group {group} page {page}"),
            Position::Page {
                doc,
                group: None,
                page,
            } => write!(f, "doc {doc} page {page}"),
        }
    }
}

pub struct StateMachine {
    state: AfpState,
    pub page_count: usize,
    // running counters, used to hand out the next index
    doc_seq: usize,
    group_seq: usize,
    page_seq: usize,
    // what is currently open, if anything
    cur_doc: Option<usize>,
    cur_group: Option<usize>,
    cur_page: Option<usize>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: AfpState::OutsideDocument,
            page_count: 0,
            doc_seq: 0,
            group_seq: 0,
            page_seq: 0,
            cur_doc: None,
            cur_group: None,
            cur_page: None,
        }
    }

    pub fn position(&self) -> Position {
        let Some(doc) = self.cur_doc else {
            return Position::Preamble;
        };
        if let Some(page) = self.cur_page {
            return Position::Page {
                doc,
                group: self.cur_group,
                page,
            };
        }
        if let Some(group) = self.cur_group {
            return Position::PageGroup { doc, group };
        }
        Position::Document { doc }
    }

    fn open_document(&mut self) {
        self.cur_doc = Some(self.doc_seq);
        self.doc_seq += 1;
        self.group_seq = 0;
        self.page_seq = 0;
        self.cur_group = None;
        self.cur_page = None;
    }

    fn close_document(&mut self) {
        self.cur_doc = None;
        self.cur_group = None;
        self.cur_page = None;
    }

    fn open_page_group(&mut self) {
        self.cur_group = Some(self.group_seq);
        self.group_seq += 1;
        self.cur_page = None;
    }

    fn open_page(&mut self) {
        self.cur_page = Some(self.page_seq);
        self.page_seq += 1;
        self.page_count += 1;
    }

    pub fn feed(&mut self, boundary: AfpBoundary, offset: u64, events: &mut Vec<AfpEvent>) {
        match (self.state, boundary) {
            (AfpState::OutsideDocument, AfpBoundary::BeginDocument) => {
                self.state = AfpState::InDocument;
                self.open_document();
                events.push(AfpEvent::DocumentStart { offset });
            }
            (AfpState::InDocument, AfpBoundary::BeginPageGroup) => {
                self.state = AfpState::InPageGroup;
                self.open_page_group();
                events.push(AfpEvent::PageGroupStart { offset });
            }
            (AfpState::InPageGroup, AfpBoundary::BeginPage) => {
                self.state = AfpState::InPage;
                self.open_page();
            }
            (AfpState::InPage, AfpBoundary::EndPage) => {
                self.state = AfpState::InPageGroup;
                self.cur_page = None;
            }
            (AfpState::InPageGroup, AfpBoundary::EndPageGroup) => {
                self.state = AfpState::InDocument;
                self.cur_group = None;
                events.push(AfpEvent::PageGroupEnd {
                    offset: offset + 3,
                    needs_eng: false,
                });
            }
            (AfpState::InDocument, AfpBoundary::EndDocument) => {
                self.state = AfpState::OutsideDocument;
                self.close_document();
                events.push(AfpEvent::DocumentEnd { offset: offset + 3 });
            }
            (AfpState::InDocument, AfpBoundary::BeginDocument) => {
                events.push(AfpEvent::DocumentEnd { offset });
                self.state = AfpState::InDocument;
                self.open_document();
                events.push(AfpEvent::DocumentStart { offset });
            }
            (AfpState::InPageGroup, AfpBoundary::EndDocument) => {
                self.state = AfpState::OutsideDocument;
                self.close_document();
                events.push(AfpEvent::PageGroupEnd {
                    offset,
                    needs_eng: true,
                });
                events.push(AfpEvent::DocumentEnd { offset: offset + 3 });
            }
            (AfpState::InDocument, AfpBoundary::BeginPage) => {
                self.state = AfpState::InPage;
                self.open_page();
            }
            (AfpState::InPage, AfpBoundary::EndDocument) => {
                self.state = AfpState::OutsideDocument;
                self.close_document();
                events.push(AfpEvent::DocumentEnd { offset: offset + 3 });
            }
            _ => {}
        }
    }

    pub fn finish(&mut self, file_size: u64, events: &mut Vec<AfpEvent>) {
        match self.state {
            AfpState::InPage => {
                events.push(AfpEvent::PageGroupEnd {
                    offset: file_size,
                    needs_eng: true,
                });
                events.push(AfpEvent::DocumentEnd { offset: file_size });
            }
            AfpState::InPageGroup => {
                events.push(AfpEvent::PageGroupEnd {
                    offset: file_size,
                    needs_eng: true,
                });
                events.push(AfpEvent::DocumentEnd { offset: file_size });
            }
            AfpState::InDocument => {
                events.push(AfpEvent::DocumentEnd { offset: file_size });
            }
            AfpState::OutsideDocument => {}
        }
        self.state = AfpState::OutsideDocument;
        self.close_document();
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AfpBoundary::*;

    fn run(boundaries: &[AfpBoundary]) -> StateMachine {
        let mut sm = StateMachine::new();
        let mut events = Vec::new();
        for (i, b) in boundaries.iter().enumerate() {
            sm.feed(*b, i as u64 * 10, &mut events);
        }
        sm
    }

    #[test]
    fn before_the_first_document_is_the_preamble() {
        assert_eq!(StateMachine::new().position(), Position::Preamble);
        assert_eq!(run(&[BeginPage]).position(), Position::Preamble);
    }

    #[test]
    fn reports_nesting_depth() {
        assert_eq!(
            run(&[BeginDocument]).position(),
            Position::Document { doc: 0 }
        );
        assert_eq!(
            run(&[BeginDocument, BeginPageGroup]).position(),
            Position::PageGroup { doc: 0, group: 0 }
        );
        assert_eq!(
            run(&[BeginDocument, BeginPageGroup, BeginPage]).position(),
            Position::Page {
                doc: 0,
                group: Some(0),
                page: 0
            }
        );
    }

    #[test]
    fn pages_without_a_page_group_omit_it() {
        let sm = run(&[BeginDocument, BeginPage]);
        assert_eq!(
            sm.position(),
            Position::Page {
                doc: 0,
                group: None,
                page: 0
            }
        );
        assert_eq!(sm.position().to_string(), "doc 0 page 0");
    }

    #[test]
    fn group_and_page_indexes_restart_per_document() {
        let sm = run(&[
            BeginDocument,
            BeginPageGroup,
            BeginPage,
            EndPage,
            EndPageGroup,
            EndDocument,
            BeginDocument,
            BeginPageGroup,
            BeginPage,
        ]);
        assert_eq!(
            sm.position(),
            Position::Page {
                doc: 1,
                group: Some(0),
                page: 0
            }
        );
        // the file-wide total keeps counting across documents
        assert_eq!(sm.page_count, 2);
    }

    #[test]
    fn indexes_advance_within_a_document() {
        let sm = run(&[
            BeginDocument,
            BeginPageGroup,
            BeginPage,
            EndPage,
            EndPageGroup,
            BeginPageGroup,
            BeginPage,
            EndPage,
            BeginPage,
        ]);
        assert_eq!(
            sm.position(),
            Position::Page {
                doc: 0,
                group: Some(1),
                page: 2
            }
        );
        assert_eq!(sm.position().to_string(), "doc 0 group 1 page 2");
    }

    #[test]
    fn an_implicit_document_boundary_advances_the_index() {
        // BDT while already inside a document closes the previous one
        let sm = run(&[BeginDocument, BeginDocument]);
        assert_eq!(sm.position(), Position::Document { doc: 1 });
    }

    #[test]
    fn closing_a_document_returns_to_the_preamble_level() {
        assert_eq!(
            run(&[BeginDocument, EndDocument]).position(),
            Position::Preamble
        );
        assert_eq!(
            run(&[BeginDocument, BeginPageGroup, EndDocument]).position(),
            Position::Preamble
        );
        assert_eq!(
            run(&[BeginDocument, BeginPage, EndDocument]).position(),
            Position::Preamble
        );
    }

    #[test]
    fn leaving_a_page_or_group_drops_back_one_level() {
        assert_eq!(
            run(&[BeginDocument, BeginPageGroup, BeginPage, EndPage]).position(),
            Position::PageGroup { doc: 0, group: 0 }
        );
        assert_eq!(
            run(&[
                BeginDocument,
                BeginPageGroup,
                BeginPage,
                EndPage,
                EndPageGroup
            ])
            .position(),
            Position::Document { doc: 0 }
        );
    }
}
