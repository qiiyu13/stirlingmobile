uniffi::setup_scaffolding!();

mod merge;

pub use merge::{merge_pdfs, EngineError};
