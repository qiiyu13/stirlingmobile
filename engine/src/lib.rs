uniffi::setup_scaffolding!();

mod info;
mod merge;
mod pages;
mod rotate;
mod split;

pub use info::get_page_count;
pub use merge::{merge_pdfs, EngineError};
pub use pages::{extract_pages, remove_pages};
pub use rotate::rotate_pdf;
pub use split::split_pdf;
