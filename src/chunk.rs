use crate::state::AfpEvent;

#[derive(Debug, Clone)]
pub struct PageGroup {
    pub start: u64,
    pub end: u64,
    pub contains_edt: bool,
}

impl PageGroup {
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub start: u64,
    pub prologue_end: u64,
    pub page_groups: Vec<PageGroup>,
    pub end: u64,
}

impl Document {
    pub fn prologue_size(&self) -> u64 {
        self.prologue_end.saturating_sub(self.start)
    }
}

#[derive(Debug)]
pub struct OutputChunk {
    pub prologue: (u64, u64),
    pub page_groups: Vec<PageGroup>,
    pub pg_range: (u64, u64),
    pub needs_edt: bool,
}

pub fn assemble_documents(events: &[AfpEvent]) -> Vec<Document> {
    let mut documents = Vec::new();
    let mut current: Option<DocumentInProgress> = None;

    for event in events {
        match event {
            AfpEvent::DocumentStart { offset } => {
                if let Some(doc) = current.take() {
                    documents.push(doc.finalize());
                }
                current = Some(DocumentInProgress {
                    start: *offset,
                    prologue_end: None,
                    page_groups: Vec::new(),
                    end: 0,
                });
            }
            AfpEvent::PageGroupStart { offset } => {
                if let Some(ref mut doc) = current {
                    if doc.prologue_end.is_none() {
                        doc.prologue_end = Some(*offset);
                    }
                    doc.page_groups.push(PageGroupInProgress {
                        start: *offset,
                        end: 0,
                    });
                }
            }
            AfpEvent::PageGroupEnd { offset } => {
                if let Some(ref mut doc) = current {
                    if let Some(pg) = doc.page_groups.last_mut() {
                        pg.end = *offset;
                    }
                }
            }
            AfpEvent::DocumentEnd { offset } => {
                if let Some(ref mut doc) = current {
                    doc.end = *offset;
                }
            }
        }
    }

    if let Some(doc) = current.take() {
        documents.push(doc.finalize());
    }

    documents
}

struct DocumentInProgress {
    start: u64,
    prologue_end: Option<u64>,
    page_groups: Vec<PageGroupInProgress>,
    end: u64,
}

struct PageGroupInProgress {
    start: u64,
    end: u64,
}

impl DocumentInProgress {
    fn finalize(self) -> Document {
        let has_explicit_groups = !self.page_groups.is_empty();

        let prologue_end = if has_explicit_groups {
            self.prologue_end.unwrap_or(self.start)
        } else {
            self.start
        };

        let mut page_groups: Vec<PageGroup> = self
            .page_groups
            .into_iter()
            .map(|p| PageGroup {
                start: p.start,
                end: p.end,
                contains_edt: false,
            })
            .collect();

        if page_groups.is_empty() {
            page_groups.push(PageGroup {
                start: self.start,
                end: self.end,
                contains_edt: true,
            });
        }

        Document {
            start: self.start,
            prologue_end,
            page_groups,
            end: self.end,
        }
    }
}

pub struct ChunkPlanner;

impl ChunkPlanner {
    pub fn plan(documents: &[Document], max_bytes: u64) -> Vec<OutputChunk> {
        let edt_overhead: u64 = 3;
        let mut chunks = Vec::new();
        let mut current_pgs: Vec<PageGroup> = Vec::new();
        let mut current_size: u64 = 0;
        let mut current_doc_start: u64 = 0;
        let mut current_doc_prologue_end: u64 = 0;
        let mut current_needs_edt: bool = false;

        for doc in documents {
            if !current_pgs.is_empty() {
                chunks.push(Self::make_chunk(
                    &mut current_pgs,
                    current_doc_start,
                    current_doc_prologue_end,
                    current_needs_edt,
                ));
                current_size = 0;
            }

            for pg in &doc.page_groups {
                let edt_cost: u64 = if pg.contains_edt { 0 } else { edt_overhead };
                let cost = pg.size()
                    + if current_pgs.is_empty() {
                        doc.prologue_size() + edt_cost
                    } else {
                        0
                    };

                if !current_pgs.is_empty() && current_size + cost > max_bytes {
                    chunks.push(Self::make_chunk(
                        &mut current_pgs,
                        current_doc_start,
                        current_doc_prologue_end,
                        current_needs_edt,
                    ));
                    current_size = 0;
                }

                if current_pgs.is_empty() {
                    current_doc_start = doc.start;
                    current_doc_prologue_end = doc.prologue_end;
                    current_needs_edt = !pg.contains_edt;
                    current_size = doc.prologue_size() + edt_cost;
                }

                current_size += pg.size();
                current_pgs.push(pg.clone());
            }
        }

        if !current_pgs.is_empty() {
            chunks.push(Self::make_chunk(
                &mut current_pgs,
                current_doc_start,
                current_doc_prologue_end,
                current_needs_edt,
            ));
        }

        chunks
    }

    fn make_chunk(
        pgs: &mut Vec<PageGroup>,
        doc_start: u64,
        prologue_end: u64,
        needs_edt: bool,
    ) -> OutputChunk {
        let pg_range = (
            pgs.first().map_or(0, |p| p.start),
            pgs.last().map_or(0, |p| p.end),
        );
        OutputChunk {
            prologue: (doc_start, prologue_end),
            pg_range,
            page_groups: std::mem::take(pgs),
            needs_edt,
        }
    }
}
