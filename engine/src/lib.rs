uniffi::setup_scaffolding!();

mod info;
mod merge;
mod split;

pub use info::get_page_count;
pub use merge::{merge_pdfs, EngineError};
pub use split::split_pdf;
