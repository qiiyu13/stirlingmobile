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
