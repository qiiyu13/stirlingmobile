use crate::content_util::save_document;
use crate::EngineError;

/// Losslessly optimizes the PDF at `input_path` (object stream generation +
/// linearization, matching Stirling server's qpdf-based optimize) and writes
/// the result to `output_path`. No image recompression — see `compress.rs`
/// for that.
#[uniffi::export]
pub fn optimize_lossless(input_path: String, output_path: String) -> Result<(), EngineError> {
    imp::run(&input_path, &output_path)
}

// Android: statically linked libqpdf via qpdfjob-c.h (ADR-003 Path A).
// See engine/build.rs for the link setup and engine/native/ for the vendored libs.
#[cfg(target_os = "android")]
mod imp {
    use crate::EngineError;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    extern "C" {
        fn qpdfjob_run_from_argv(argv: *const *const c_char) -> c_int;
    }

    const QPDF_EXIT_SUCCESS: c_int = 0;
    const QPDF_EXIT_WARNING: c_int = 3;

    pub(super) fn run(input_path: &str, output_path: &str) -> Result<(), EngineError> {
        let args = [
            "qpdf",
            "--object-streams=generate",
            "--linearize",
            input_path,
            output_path,
        ];
        let c_args: Vec<CString> = args
            .iter()
            .map(|a| CString::new(*a).expect("no NUL bytes in path/args"))
            .collect();
        let mut argv: Vec<*const c_char> = c_args.iter().map(|a| a.as_ptr()).collect();
        argv.push(std::ptr::null());

        let code = unsafe { qpdfjob_run_from_argv(argv.as_ptr()) };
        if code == QPDF_EXIT_SUCCESS || code == QPDF_EXIT_WARNING {
            Ok(())
        } else {
            Err(EngineError::WriteFailed {
                reason: format!("qpdf optimize failed with exit code {code}"),
            })
        }
    }
}

// Host (dev/test): shells out to the system qpdf CLI instead of the
// statically-linked lib, same tool the stress tests already require on PATH.
#[cfg(not(target_os = "android"))]
mod imp {
    use crate::EngineError;
    use std::process::Command;

    pub(super) fn run(input_path: &str, output_path: &str) -> Result<(), EngineError> {
        let status = Command::new("qpdf")
            .args([
                "--object-streams=generate",
                "--linearize",
                input_path,
                output_path,
            ])
            .status()
            .map_err(|e| EngineError::WriteFailed {
                reason: format!("failed to spawn qpdf: {e}"),
            })?;
        match status.code() {
            Some(0) | Some(3) => Ok(()),
            other => Err(EngineError::WriteFailed {
                reason: format!("qpdf optimize failed with exit code {other:?}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{content::Content, dictionary, Document, Object};
    use std::process::Command;

    fn one_page_pdf(path: &str) {
        let mut doc = Document::with_version("1.7");
        let content_id = doc.add_object(lopdf::Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => 1,
                "Kids" => vec![Object::Reference(page_id)],
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.max_id = doc.objects.len() as u32;
        save_document(&mut doc, path).unwrap();
    }

    #[test]
    fn optimize_lossless_produces_valid_pdf() {
        let dir = std::env::temp_dir();
        let input = dir.join("optimize_test_in.pdf");
        let output = dir.join("optimize_test_out.pdf");
        one_page_pdf(input.to_str().unwrap());

        optimize_lossless(
            input.to_str().unwrap().to_string(),
            output.to_str().unwrap().to_string(),
        )
        .unwrap();

        let status = Command::new("qpdf")
            .args(["--check", output.to_str().unwrap()])
            .status()
            .expect("qpdf not found on PATH");
        assert_ne!(
            status.code(),
            Some(2),
            "qpdf --check rejected optimized output"
        );
    }
}
