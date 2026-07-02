use crate::EngineError;
use lopdf::Document;
use std::path::Path;

/// Splits the PDF at `input_path` after each page number in `split_after_pages`
/// (1-indexed, e.g. `[3, 6]` on a 10-page doc yields pages 1-3, 4-6, 7-10).
/// Writes `part_1.pdf`, `part_2.pdf`, ... into `output_dir` and returns their paths.
#[uniffi::export]
pub fn split_pdf(
    input_path: String,
    split_after_pages: Vec<u32>,
    output_dir: String,
) -> Result<Vec<String>, EngineError> {
    let doc = Document::load(&input_path).map_err(|e| EngineError::ReadFailed {
        path: input_path.clone(),
        reason: e.to_string(),
    })?;

    let total_pages = doc.get_pages().len() as u32;
    if total_pages == 0 {
        return Err(EngineError::ReadFailed {
            path: input_path,
            reason: "document has no pages".to_string(),
        });
    }

    let ranges = split_ranges(total_pages, &split_after_pages);

    let mut output_paths = Vec::with_capacity(ranges.len());
    for (index, (lo, hi)) in ranges.into_iter().enumerate() {
        let mut part = doc.clone();
        let to_delete: Vec<u32> = (1..=total_pages).filter(|p| *p < lo || *p > hi).collect();
        part.delete_pages(&to_delete);
        part.prune_objects();
        part.renumber_objects();

        let output_path = Path::new(&output_dir)
            .join(format!("part_{}.pdf", index + 1))
            .to_string_lossy()
            .into_owned();
        part.save(&output_path)
            .map_err(|e| EngineError::WriteFailed {
                reason: e.to_string(),
            })?;
        output_paths.push(output_path);
    }

    Ok(output_paths)
}

/// Turns 1-indexed split points into inclusive `(lo, hi)` page ranges covering
/// `1..=total_pages`. Out-of-range or duplicate split points are ignored.
fn split_ranges(total_pages: u32, split_after_pages: &[u32]) -> Vec<(u32, u32)> {
    let mut boundaries: Vec<u32> = split_after_pages
        .iter()
        .copied()
        .filter(|&p| p > 0 && p < total_pages)
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut ranges = Vec::with_capacity(boundaries.len() + 1);
    let mut start = 1;
    for boundary in boundaries {
        ranges.push((start, boundary));
        start = boundary + 1;
    }
    ranges.push((start, total_pages));
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_split_points_yields_single_range() {
        assert_eq!(split_ranges(10, &[]), vec![(1, 10)]);
    }

    #[test]
    fn split_points_yield_contiguous_ranges() {
        assert_eq!(split_ranges(10, &[3, 6]), vec![(1, 3), (4, 6), (7, 10)]);
    }

    #[test]
    fn out_of_range_and_duplicate_points_are_ignored() {
        assert_eq!(split_ranges(5, &[0, 3, 3, 5, 99]), vec![(1, 3), (4, 5)]);
    }
}
