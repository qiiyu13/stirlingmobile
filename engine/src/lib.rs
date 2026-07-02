uniffi::setup_scaffolding!();

mod compress;
mod info;
mod merge;
mod pages;
mod rotate;
mod split;

pub use compress::{compress_pdf_by_level, compress_pdf_custom, compress_pdf_to_target_size};
pub use info::{describe_images, get_page_count};
pub use merge::{merge_pdfs, EngineError};
pub use pages::{extract_pages, remove_pages};
pub use rotate::rotate_pdf;
pub use split::split_pdf;
