//! Headless stress/fidelity harness. No device needed.
//! Runs each tool N times over randomly generated PDFs, asserting:
//!   - no panics / no Err on valid input
//!   - output is structurally valid (`qpdf --check`)
//!   - text content survives where it's supposed to (`pdftotext` diff)
//! ponytail: qpdf --check + pdftotext stand in for the roadmap's "fidelity
//! vs PDFBox" oracle since no PDFBox jar is available on this box.
//!
//! Iteration count: `STRESS_ITERS` env var, default 200 (roadmap asks for
//! 1000; bump the env var for a full run, default kept fast for everyday use).

use lopdf::{content::Content, content::Operation, dictionary, Document, Object};
use std::path::{Path, PathBuf};
use std::process::Command;

fn iters() -> u32 {
    std::env::var("STRESS_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200)
}

// xorshift32, deterministic per seed, no extra dependency.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.next() % (hi - lo + 1)
    }
}

fn work_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("stirling_stress");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Builds an n-page PDF with a distinct, pdftotext-extractable marker per page
/// (base-14 Helvetica, no embedding needed).
fn build_pdf(path: &Path, n: u32, marker_prefix: &str) {
    let mut doc = Document::with_version("1.7");
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for i in 1..=n {
        let text = format!("{marker_prefix}-{i}");
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 24.into()]),
                Operation::new("Td", vec![72.into(), 700.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(lopdf::Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Contents" => content_id,
        });
        kids.push(Object::Reference(page_id));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => n as i64,
            "Kids" => kids,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(path).unwrap();
}

/// Like `build_pdf`, but pads each page's content stream with junk `Tj` ops
/// to approach a target on-disk file size — needed to reproduce NF-002
/// (2x 50MB / ~250 pages) instead of the KB-sized fixtures the correctness
/// stress tests use.
fn build_pdf_sized(path: &Path, n: u32, marker_prefix: &str, target_bytes: usize) {
    let mut doc = Document::with_version("1.7");
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    let pad_per_page = target_bytes / n as usize;
    let mut rng = Rng(0x5eed);
    for i in 1..=n {
        let text = format!("{marker_prefix}-{i}");
        // Random (incompressible) padding — a repeated-byte fill would let
        // Flate crush the stream back to nothing, which understates real
        // merge cost on image-heavy PDFs. Random bytes keep the on-disk
        // size honest through save_document's compression.
        let junk: Vec<u8> = (0..pad_per_page.max(1))
            .map(|_| rng.next() as u8)
            .collect();
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 24.into()]),
                Operation::new("Td", vec![72.into(), 700.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
                // opaque padding — never rendered, just bulks up the stream
                // so on-disk size approximates a real-world scanned/image PDF
                Operation::new("%pad", vec![Object::string_literal(junk)]),
            ],
        };
        let content_id = doc.add_object(lopdf::Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Contents" => content_id,
        });
        kids.push(Object::Reference(page_id));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => n as i64,
            "Kids" => kids,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(path).unwrap();
}

fn qpdf_check(path: &Path) -> bool {
    Command::new("qpdf")
        .arg("--check")
        .arg(path)
        .status()
        .expect("qpdf not found on PATH")
        .success()
}

fn qpdf_check_with_password(path: &Path, password: &str) -> bool {
    Command::new("qpdf")
        .arg(format!("--password={password}"))
        .arg("--check")
        .arg(path)
        .status()
        .expect("qpdf not found on PATH")
        .success()
}

fn pdftotext(path: &Path) -> String {
    let out = Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .expect("pdftotext not found on PATH");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn stress_merge() {
    let dir = work_dir();
    let mut rng = Rng(1);
    for i in 0..iters() {
        let a_n = rng.range(1, 8);
        let b_n = rng.range(1, 8);
        let a = dir.join(format!("merge_a_{i}.pdf"));
        let b = dir.join(format!("merge_b_{i}.pdf"));
        let out = dir.join(format!("merge_out_{i}.pdf"));
        build_pdf(&a, a_n, "A");
        build_pdf(&b, b_n, "B");

        stirling_engine::merge_pdfs(
            vec![a.to_string_lossy().into_owned(), b.to_string_lossy().into_owned()],
            out.to_string_lossy().into_owned(),
        )
        .unwrap_or_else(|e| panic!("merge failed on iter {i}: {e}"));

        assert!(qpdf_check(&out), "qpdf --check failed on iter {i}");
        assert_eq!(stirling_engine::get_page_count(out.to_string_lossy().into_owned()).unwrap(), a_n + b_n);

        let text = pdftotext(&out);
        for p in 1..=a_n {
            assert!(text.contains(&format!("A-{p}")), "missing A-{p} on iter {i}");
        }
        for p in 1..=b_n {
            assert!(text.contains(&format!("B-{p}")), "missing B-{p} on iter {i}");
        }
    }
}

#[test]
fn stress_split() {
    let dir = work_dir();
    let mut rng = Rng(2);
    for i in 0..iters() {
        let n = rng.range(2, 12);
        let split_after = rng.range(1, n - 1);
        let input = dir.join(format!("split_in_{i}.pdf"));
        build_pdf(&input, n, "P");

        let parts = stirling_engine::split_pdf(
            input.to_string_lossy().into_owned(),
            vec![split_after],
            dir.to_string_lossy().into_owned(),
        )
        .unwrap_or_else(|e| panic!("split failed on iter {i}: {e}"));

        assert_eq!(parts.len(), 2);
        let part1 = Path::new(&parts[0]);
        let part2 = Path::new(&parts[1]);
        assert!(qpdf_check(part1), "part1 invalid on iter {i}");
        assert!(qpdf_check(part2), "part2 invalid on iter {i}");
        assert_eq!(
            stirling_engine::get_page_count(parts[0].clone()).unwrap(),
            split_after
        );
        assert_eq!(
            stirling_engine::get_page_count(parts[1].clone()).unwrap(),
            n - split_after
        );

        let text1 = pdftotext(part1);
        assert!(text1.contains(&format!("P-{split_after}")));
        assert!(!text1.contains(&format!("P-{}", split_after + 1)));
    }
}

#[test]
fn stress_rotate() {
    let dir = work_dir();
    let mut rng = Rng(3);
    let angles = [0, 90, 180, 270, -90];
    for i in 0..iters() {
        let n = rng.range(1, 6);
        let angle = angles[(rng.next() as usize) % angles.len()];
        let input = dir.join(format!("rotate_in_{i}.pdf"));
        let out = dir.join(format!("rotate_out_{i}.pdf"));
        build_pdf(&input, n, "R");

        stirling_engine::rotate_pdf(
            input.to_string_lossy().into_owned(),
            angle,
            out.to_string_lossy().into_owned(),
        )
        .unwrap_or_else(|e| panic!("rotate failed on iter {i}: {e}"));

        assert!(qpdf_check(&out), "qpdf --check failed on iter {i}");
        assert_eq!(stirling_engine::get_page_count(out.to_string_lossy().into_owned()).unwrap(), n);
        // rotation doesn't touch text content
        let text = pdftotext(&out);
        assert!(text.contains(&format!("R-{n}")));
    }
}

#[test]
fn stress_remove_extract() {
    let dir = work_dir();
    let mut rng = Rng(4);
    for i in 0..iters() {
        let n = rng.range(3, 10);
        let drop_page = rng.range(1, n);
        let input = dir.join(format!("re_in_{i}.pdf"));
        build_pdf(&input, n, "X");

        let removed = dir.join(format!("re_removed_{i}.pdf"));
        stirling_engine::remove_pages(
            input.to_string_lossy().into_owned(),
            vec![drop_page],
            removed.to_string_lossy().into_owned(),
        )
        .unwrap_or_else(|e| panic!("remove failed on iter {i}: {e}"));
        assert!(qpdf_check(&removed));
        assert_eq!(
            stirling_engine::get_page_count(removed.to_string_lossy().into_owned()).unwrap(),
            n - 1
        );
        let removed_text = pdftotext(&removed);
        assert!(!removed_text.contains(&format!("X-{drop_page}\n")));

        let extracted = dir.join(format!("re_extracted_{i}.pdf"));
        stirling_engine::extract_pages(
            input.to_string_lossy().into_owned(),
            vec![drop_page],
            extracted.to_string_lossy().into_owned(),
        )
        .unwrap_or_else(|e| panic!("extract failed on iter {i}: {e}"));
        assert!(qpdf_check(&extracted));
        assert_eq!(stirling_engine::get_page_count(extracted.to_string_lossy().into_owned()).unwrap(), 1);
        let extracted_text = pdftotext(&extracted);
        assert!(extracted_text.contains(&format!("X-{drop_page}")));
    }
}

#[test]
fn stress_compress() {
    let dir = work_dir();
    let mut rng = Rng(5);
    for i in 0..iters() {
        let n = rng.range(1, 4);
        let level = rng.range(1, 9) as u8;
        let input = dir.join(format!("compress_in_{i}.pdf"));
        let out = dir.join(format!("compress_out_{i}.pdf"));
        build_pdf(&input, n, "C");

        stirling_engine::compress_pdf_by_level(input.to_string_lossy().into_owned(), level, out.to_string_lossy().into_owned())
            .unwrap_or_else(|e| panic!("compress failed on iter {i}: {e}"));

        assert!(qpdf_check(&out), "qpdf --check failed on iter {i}");
        assert_eq!(stirling_engine::get_page_count(out.to_string_lossy().into_owned()).unwrap(), n);
    }
}

#[test]
fn stress_password_round_trip() {
    let dir = work_dir();
    let mut rng = Rng(6);
    for i in 0..iters() {
        let n = rng.range(1, 5);
        let password = format!("pw-{}", rng.next());
        let input = dir.join(format!("pw_in_{i}.pdf"));
        let protected = dir.join(format!("pw_protected_{i}.pdf"));
        let recovered = dir.join(format!("pw_recovered_{i}.pdf"));
        build_pdf(&input, n, "S");
        let original_text = pdftotext(&input);

        stirling_engine::add_password(
            input.to_string_lossy().into_owned(),
            password.clone(),
            String::new(),
            protected.to_string_lossy().into_owned(),
        )
        .unwrap_or_else(|e| panic!("add_password failed on iter {i}: {e}"));

        assert!(
            qpdf_check_with_password(&protected, &password),
            "protected pdf invalid on iter {i}"
        );
        // wrong password must be rejected
        assert!(stirling_engine::remove_password(
            protected.to_string_lossy().into_owned(),
            "wrong-password".to_string(),
            recovered.to_string_lossy().into_owned(),
        )
        .is_err());

        stirling_engine::remove_password(
            protected.to_string_lossy().into_owned(),
            password,
            recovered.to_string_lossy().into_owned(),
        )
        .unwrap_or_else(|e| panic!("remove_password failed on iter {i}: {e}"));

        assert!(qpdf_check(&recovered), "recovered pdf invalid on iter {i}");
        assert_eq!(pdftotext(&recovered), original_text, "text mismatch on iter {i}");
    }
}

/// W21 profiling: real wall-clock number for NF-002
/// (docs/07-performance-budget.md — "Merge 2 PDFs (NF-002): 2x 50MB,
/// ~250 pages each, target <5s"). `#[ignore]`d — heavy (builds ~100MB of
/// fixtures) and not a correctness check, run explicitly:
///   cargo test --release --test stress bench_merge_nf002 -- --ignored --nocapture
#[test]
#[ignore]
fn bench_merge_nf002() {
    let dir = work_dir();
    let a = dir.join("nf002_a.pdf");
    let b = dir.join("nf002_b.pdf");
    let out = dir.join("nf002_out.pdf");
    let target_bytes = 50 * 1024 * 1024;
    build_pdf_sized(&a, 250, "A", target_bytes);
    build_pdf_sized(&b, 250, "B", target_bytes);

    let a_size = std::fs::metadata(&a).unwrap().len();
    let b_size = std::fs::metadata(&b).unwrap().len();

    let start = std::time::Instant::now();
    stirling_engine::merge_pdfs(
        vec![a.to_string_lossy().into_owned(), b.to_string_lossy().into_owned()],
        out.to_string_lossy().into_owned(),
    )
    .expect("merge failed");
    let elapsed = start.elapsed();

    let out_size = std::fs::metadata(&out).unwrap().len();
    assert!(qpdf_check(&out), "qpdf --check failed on merged output");
    assert_eq!(
        stirling_engine::get_page_count(out.to_string_lossy().into_owned()).unwrap(),
        500
    );

    println!(
        "NF-002: merge {}MB + {}MB -> {}MB in {:.3}s (target <5s)",
        a_size / 1024 / 1024,
        b_size / 1024 / 1024,
        out_size / 1024 / 1024,
        elapsed.as_secs_f64()
    );
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "NF-002 regression: merge took {:.3}s, budget is 5s",
        elapsed.as_secs_f64()
    );
}

/// W21 profiling: "Split pages | 50MB, 200 pages | < 2s" (07-performance-budget.md §1).
#[test]
#[ignore]
fn bench_split_50mb() {
    let dir = work_dir();
    let input = dir.join("split_in.pdf");
    build_pdf_sized(&input, 200, "S", 50 * 1024 * 1024);
    let in_size = std::fs::metadata(&input).unwrap().len();

    let start = std::time::Instant::now();
    let parts = stirling_engine::split_pdf(
        input.to_string_lossy().into_owned(),
        vec![100],
        dir.to_string_lossy().into_owned(),
    )
    .expect("split failed");
    let elapsed = start.elapsed();

    assert_eq!(parts.len(), 2);
    for p in &parts {
        assert!(qpdf_check(Path::new(p)), "qpdf --check failed on {p}");
    }

    println!(
        "Split: {}MB / 200 pages -> {} parts in {:.3}s (target <2s)",
        in_size / 1024 / 1024,
        parts.len(),
        elapsed.as_secs_f64()
    );
    assert!(
        elapsed.as_secs_f64() < 2.0,
        "Split regression: took {:.3}s, budget is 2s",
        elapsed.as_secs_f64()
    );
}

/// W21 profiling: "Rotate all pages | 50MB, 200 pages | < 3s" (07-performance-budget.md §1).
#[test]
#[ignore]
fn bench_rotate_50mb() {
    let dir = work_dir();
    let input = dir.join("rotate_in.pdf");
    let out = dir.join("rotate_out.pdf");
    build_pdf_sized(&input, 200, "R", 50 * 1024 * 1024);
    let in_size = std::fs::metadata(&input).unwrap().len();

    let start = std::time::Instant::now();
    stirling_engine::rotate_pdf(
        input.to_string_lossy().into_owned(),
        90,
        out.to_string_lossy().into_owned(),
    )
    .expect("rotate failed");
    let elapsed = start.elapsed();

    assert!(qpdf_check(&out), "qpdf --check failed on rotated output");

    println!(
        "Rotate: {}MB / 200 pages in {:.3}s (target <3s)",
        in_size / 1024 / 1024,
        elapsed.as_secs_f64()
    );
    assert!(
        elapsed.as_secs_f64() < 3.0,
        "Rotate regression: took {:.3}s, budget is 3s",
        elapsed.as_secs_f64()
    );
}
