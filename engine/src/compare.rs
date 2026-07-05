//! Visual page-by-page PDF diff (F-110): rasterize both PDFs at `dpi` and
//! flag pages whose rendered pixels differ, producing a red-highlighted diff
//! PNG for each mismatching page.

use crate::rasterize::bind_pdfium;
use crate::EngineError;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use pdfium_render::prelude::*;
use std::path::Path;

/// Per-channel byte difference below which two pixels are considered the same
/// (absorbs rasterization/anti-aliasing noise, not real content changes).
const DIFF_THRESHOLD: i32 = 24;

#[derive(Debug, Clone, uniffi::Record)]
pub struct PageComparison {
    pub page: u32,
    pub identical: bool,
    pub diff_image_path: Option<String>,
}

/// Compares `input_path_a` and `input_path_b` page-by-page. Returns one
/// [`PageComparison`] per page (using the longer document's page count); a
/// page present in only one document is reported as non-identical with no
/// diff image.
/// # ponytail: whole-page pixel diff at fixed dpi, not a structural/text
/// diff — good enough to flag "these pages differ" and show roughly where.
#[uniffi::export]
pub fn tool_compare(
    input_path_a: String,
    input_path_b: String,
    pdfium_lib_dir: String,
    dpi: u32,
    output_dir: String,
) -> Result<Vec<PageComparison>, EngineError> {
    let pdfium = bind_pdfium(&pdfium_lib_dir)?;

    let doc_a = pdfium
        .load_pdf_from_file(&input_path_a, None)
        .map_err(|e| EngineError::ReadFailed {
            path: input_path_a.clone(),
            reason: e.to_string(),
        })?;
    let doc_b = pdfium
        .load_pdf_from_file(&input_path_b, None)
        .map_err(|e| EngineError::ReadFailed {
            path: input_path_b.clone(),
            reason: e.to_string(),
        })?;

    let images_a = render_pages(&doc_a, dpi)?;
    let images_b = render_pages(&doc_b, dpi)?;
    let total = images_a.len().max(images_b.len());

    let mut results = Vec::with_capacity(total);
    for i in 0..total {
        let page = (i + 1) as u32;
        let comparison = match (images_a.get(i), images_b.get(i)) {
            (Some(a), Some(b)) => {
                let (identical, diff) = diff_image(a, b);
                let diff_image_path = if identical {
                    None
                } else {
                    let path = Path::new(&output_dir)
                        .join(format!("page_{page}_diff.png"))
                        .to_string_lossy()
                        .into_owned();
                    diff.save(&path).map_err(|e| EngineError::WriteFailed {
                        reason: format!("failed to save diff for page {page}: {e}"),
                    })?;
                    Some(path)
                };
                PageComparison {
                    page,
                    identical,
                    diff_image_path,
                }
            }
            _ => PageComparison {
                page,
                identical: false,
                diff_image_path: None,
            },
        };
        results.push(comparison);
    }

    Ok(results)
}

fn render_pages(document: &PdfDocument, dpi: u32) -> Result<Vec<DynamicImage>, EngineError> {
    let mut images = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let target_width = (page.width().value / 72.0 * dpi as f32).round().max(1.0) as i32;
        let config = PdfRenderConfig::new().set_target_width(target_width);
        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| EngineError::WriteFailed {
                reason: format!("failed to render page {}: {e}", index + 1),
            })?;
        images.push(bitmap.as_image());
    }
    Ok(images)
}

/// Compares two rendered pages pixel-by-pixel (resizing `b` to `a`'s
/// dimensions if the page sizes differ). Returns whether they're identical
/// and a diff image: differing pixels in red, matching pixels in greyscale.
fn diff_image(a: &DynamicImage, b: &DynamicImage) -> (bool, RgbaImage) {
    let (w, h) = a.dimensions();
    let b_resized = if b.dimensions() == (w, h) {
        b.clone()
    } else {
        b.resize_exact(w, h, image::imageops::FilterType::Triangle)
    };
    let a_rgba = a.to_rgba8();
    let b_rgba = b_resized.to_rgba8();

    let mut out = RgbaImage::new(w, h);
    let mut identical = true;
    for (x, y, pa) in a_rgba.enumerate_pixels() {
        let pb = b_rgba.get_pixel(x, y);
        let differs = (0..3).any(|c| (pa[c] as i32 - pb[c] as i32).abs() > DIFF_THRESHOLD);
        if differs {
            identical = false;
            out.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        } else {
            let gray = ((pa[0] as u32 + pa[1] as u32 + pa[2] as u32) / 3) as u8;
            out.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
        }
    }
    (identical, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba as PixelRgba};

    fn solid(w: u32, h: u32, color: [u8; 4]) -> DynamicImage {
        DynamicImage::ImageRgba8(ImageBuffer::from_fn(w, h, |_, _| PixelRgba(color)))
    }

    #[test]
    fn identical_images_report_no_diff() {
        let a = solid(10, 10, [10, 20, 30, 255]);
        let b = solid(10, 10, [10, 20, 30, 255]);
        let (identical, _) = diff_image(&a, &b);
        assert!(identical);
    }

    #[test]
    fn differing_images_flag_red_pixels() {
        let a = solid(10, 10, [10, 20, 30, 255]);
        let b = solid(10, 10, [200, 200, 200, 255]);
        let (identical, diff) = diff_image(&a, &b);
        assert!(!identical);
        assert_eq!(*diff.get_pixel(0, 0), PixelRgba([255, 0, 0, 255]));
    }

    #[test]
    fn tool_compare_fails_cleanly_without_pdfium_lib() {
        let result = tool_compare(
            "/nonexistent/a.pdf".to_string(),
            "/nonexistent/b.pdf".to_string(),
            "/nonexistent/dir".to_string(),
            150,
            std::env::temp_dir().to_string_lossy().into_owned(),
        );
        assert!(result.is_err());
    }
}
