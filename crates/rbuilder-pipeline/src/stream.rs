//! Bounded-channel streaming between parallel extractors and sequential graph merge.

use crate::parallel::with_pool;
use crossbeam::channel::{bounded, Receiver};
use rbuilder_error::Result;
use rbuilder_extraction::{ExtractionTail, Extractor, FileExtraction, GraphBuilder};
use rbuilder_registry::LanguageRegistry;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

/// Default in-flight extraction cap (~1024 file buffers max between extract and merge).
pub const DEFAULT_STREAM_CHANNEL_CAPACITY: usize = 1024;

/// Run parallel extractors into a bounded channel while the caller consumes on the main thread.
pub fn start_parallel_extraction(
    thread_count: Option<usize>,
    registry: Arc<LanguageRegistry>,
    files: Arc<Vec<PathBuf>>,
    capacity: usize,
    on_file_done: impl Fn() + Send + Sync + 'static,
) -> Receiver<FileExtraction> {
    let (tx, rx) = bounded(capacity);
    std::thread::spawn(move || {
        with_pool(thread_count, || {
            files.par_iter().for_each(|path| {
                let extractor = Extractor::new(Arc::clone(&registry));
                if let Ok(extraction) = extractor.extract_file(path) {
                    let _ = tx.send(extraction);
                }
                on_file_done();
            });
        });
    });
    rx
}

/// Extract in parallel, merge pass-1 immediately, and retain only relation tails for pass 2.
pub fn stream_into_graph(
    thread_count: Option<usize>,
    extractor: &Extractor,
    registry: Arc<LanguageRegistry>,
    files: &[PathBuf],
    capacity: usize,
    builder: &mut GraphBuilder,
    on_file_done: impl Fn() + Send + Sync + 'static,
) -> Result<(usize, Vec<ExtractionTail>)> {
    let files = Arc::new(files.to_vec());
    let file_count = files.len();
    let rx = start_parallel_extraction(thread_count, registry, files, capacity, on_file_done);

    let mut tails = Vec::with_capacity(file_count);
    let mut files_processed = 0usize;
    while let Ok(mut extraction) = rx.recv() {
        tails.push(extractor.populate_pass1(&mut extraction, builder)?);
        files_processed += 1;
    }

    Ok((files_processed, tails))
}
