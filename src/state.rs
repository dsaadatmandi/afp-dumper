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
    PageGroupEnd { offset: u64 },
}

pub struct StateMachine {
    state: AfpState,
    pub page_count: usize,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: AfpState::OutsideDocument,
            page_count: 0,
        }
    }

    pub fn feed(
        &mut self,
        boundary: AfpBoundary,
        offset: u64,
        events: &mut Vec<AfpEvent>,
    ) {
        match (self.state, boundary) {
            (AfpState::OutsideDocument, AfpBoundary::BeginDocument) => {
                self.state = AfpState::InDocument;
                events.push(AfpEvent::DocumentStart { offset });
            }
            (AfpState::InDocument, AfpBoundary::BeginPageGroup) => {
                self.state = AfpState::InPageGroup;
                events.push(AfpEvent::PageGroupStart { offset });
            }
            (AfpState::InPageGroup, AfpBoundary::BeginPage) => {
                self.state = AfpState::InPage;
                self.page_count += 1;
            }
            (AfpState::InPage, AfpBoundary::EndPage) => {
                self.state = AfpState::InPageGroup;
            }
            (AfpState::InPageGroup, AfpBoundary::EndPageGroup) => {
                self.state = AfpState::InDocument;
                events.push(AfpEvent::PageGroupEnd { offset: offset + 3 });
            }
            (AfpState::InDocument, AfpBoundary::EndDocument) => {
                self.state = AfpState::OutsideDocument;
                events.push(AfpEvent::DocumentEnd { offset: offset + 3 });
            }
            (AfpState::InDocument, AfpBoundary::BeginDocument) => {
                events.push(AfpEvent::DocumentEnd { offset });
                self.state = AfpState::InDocument;
                events.push(AfpEvent::DocumentStart { offset });
            }
            (AfpState::InPageGroup, AfpBoundary::EndDocument) => {
                self.state = AfpState::OutsideDocument;
                events.push(AfpEvent::PageGroupEnd { offset });
                events.push(AfpEvent::DocumentEnd { offset: offset + 3 });
            }
            (AfpState::InDocument, AfpBoundary::BeginPage) => {
                self.state = AfpState::InPage;
                self.page_count += 1;
            }
            (AfpState::InPage, AfpBoundary::EndDocument) => {
                self.state = AfpState::OutsideDocument;
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
                });
                events.push(AfpEvent::DocumentEnd {
                    offset: file_size,
                });
            }
            AfpState::InPageGroup => {
                events.push(AfpEvent::PageGroupEnd {
                    offset: file_size,
                });
                events.push(AfpEvent::DocumentEnd {
                    offset: file_size,
                });
            }
            AfpState::InDocument => {
                events.push(AfpEvent::DocumentEnd {
                    offset: file_size,
                });
            }
            AfpState::OutsideDocument => {}
        }
        self.state = AfpState::OutsideDocument;
    }
}
