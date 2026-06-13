use crate::boundary::AfpBoundary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfpState {
    OutsideDocument,
    InDocument,
    InPage,
}

#[derive(Debug, Clone)]
pub enum AfpEvent {
    DocumentStart { offset: u64 },
    DocumentEnd { offset: u64 },
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
            (AfpState::InDocument, AfpBoundary::BeginPage) => {
                self.state = AfpState::InPage;
                self.page_count += 1;
            }
            (AfpState::InPage, AfpBoundary::EndPage) => {
                self.state = AfpState::InDocument;
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
            (AfpState::InPage, AfpBoundary::EndDocument) => {
                self.state = AfpState::OutsideDocument;
                events.push(AfpEvent::DocumentEnd { offset: offset + 3 });
            }
            _ => {}
        }
    }

    pub fn finish(&mut self, file_size: u64, events: &mut Vec<AfpEvent>) {
        if self.state == AfpState::InDocument || self.state == AfpState::InPage {
            events.push(AfpEvent::DocumentEnd {
                offset: file_size,
            });
        }
        self.state = AfpState::OutsideDocument;
    }
}
