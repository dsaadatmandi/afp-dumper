use crate::state::AfpEvent;

#[derive(Debug, Clone)]
pub struct DocumentSpan {
    pub start: u64,
    pub end: u64,
}

impl DocumentSpan {
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Debug)]
pub struct OutputChunk {
    pub documents: Vec<DocumentSpan>,
}

pub struct ChunkPlanner;

impl ChunkPlanner {
    pub fn plan(events: &[AfpEvent], max_bytes: u64) -> Vec<OutputChunk> {
        let documents = Self::extract_documents(events);

        let mut chunks = Vec::new();
        let mut current_docs = Vec::new();
        let mut current_size: u64 = 0;

        for doc in documents {
            let doc_size = doc.size();

            if !current_docs.is_empty() && current_size + doc_size > max_bytes {
                chunks.push(OutputChunk {
                    documents: std::mem::take(&mut current_docs),
                });
                current_size = 0;
            }

            current_size += doc_size;
            current_docs.push(doc);
        }

        if !current_docs.is_empty() {
            chunks.push(OutputChunk {
                documents: current_docs,
            });
        }

        chunks
    }

    fn extract_documents(events: &[AfpEvent]) -> Vec<DocumentSpan> {
        let mut documents = Vec::new();
        let mut start: Option<u64> = None;

        for event in events {
            match event {
                AfpEvent::DocumentStart { offset } => {
                    start = Some(*offset);
                }
                AfpEvent::DocumentEnd { offset } => {
                    if let Some(s) = start.take() {
                        documents.push(DocumentSpan {
                            start: s,
                            end: *offset,
                        });
                    }
                }
            }
        }

        documents
    }
}
