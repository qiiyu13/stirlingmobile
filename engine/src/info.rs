use crate::EngineError;
use lopdf::Document;

/// Returns the page count of the PDF at `path`.
#[uniffi::export]
pub fn get_page_count(path: String) -> Result<u32, EngineError> {
    let doc = Document::load(&path).map_err(|e| EngineError::ReadFailed {
        path,
        reason: e.to_string(),
    })?;
    Ok(doc.get_pages().len() as u32)
}
