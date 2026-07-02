# Stirling Mobile — Glossary

> **Status:** Draft v1.0

---

## Domain Terms

| Term | Definition |
|---|---|
| **SAF** | Storage Access Framework — Android API for file picking. Users choose files, app gets a content URI. No storage permission needed on API 30+. |
| **JNI** | Java Native Interface — the bridge between Kotlin/Java and native C/C++/Rust code. Our Kotlin code calls Rust functions via JNI. |
| **NDK** | Native Development Kit — Android toolchain for compiling C/C++/Rust to `.so` shared libraries. |
| **uniffi** | Mozilla's tool for auto-generating language bindings from Rust. Generates Kotlin JNI wrappers from a `.udl` file. |
| **lopdf** | A Rust library for reading, writing, and manipulating PDF files. Our PDF core — equivalent to Stirling server's PDFBox. |
| **pdfium** | Google's PDF rendering engine (used in Chrome). We use the Rust `pdfium-render` crate as our PDF viewer. |
| **qpdf** | C++ library/CLI for content-preserving PDF transformations (optimization, linearization, encryption). We rewrite its optimization logic in Rust. |
| **Collabora Office SDK** | LibreOffice compiled for Android. Provides Office↔PDF conversion. MPL 2.0 licensed. |
| **Tesseract** | Open-source OCR engine. We use the NDK-compiled version (`tess-two`). Same engine Stirling server uses. |
| **WeasyPrint** | Python HTML/CSS→PDF converter used by Stirling server. We replace it with Android WebView `printToPdf()`. |
| **Jetpack Compose** | Google's declarative UI framework for Android. Our entire UI is built with it. |
| **Material 3** | Google's latest design system (Material You). Supports dynamic colors from wallpaper. |
| **Coroutine** | Kotlin's concurrency primitive. Lightweight threads for async operations. |
| **ViewModel** | Android Architecture Component. Survives configuration changes, manages UI state. |
| **Room** | Android's SQLite abstraction layer. Used for file history persistence. |
| **DataStore** | Android's key-value storage. Used for user preferences. |
| **APK** | Android Package — the installable file format. |
| **AAB** | Android App Bundle — Play Store's publishing format. Enables dynamic delivery. |
| **DP** | Density-independent pixel. Android's responsive unit. 1dp = 1px at 160dpi. |
| **SP** | Scale-independent pixel. Like dp but scales with user's font size setting. |

## PDF-Specific Terms

| Term | Definition |
|---|---|
| **PDFBox** | Apache PDFBox — Java library Stirling server uses for PDF manipulation. Our equivalent is Rust's `lopdf`. |
| **Linearization** | Reorganizing a PDF file for fast web viewing (byte-range loading). Also called "Fast Web View." |
| **Object Stream** | A compressed container in PDF for bundling small objects. Reduces file size. |
| **Cross-Reference Table (xref)** | PDF structure that maps object numbers to file byte offsets. Critical for fast random access. |
| **AcroForm** | Adobe's form technology in PDF. Fillable fields, checkboxes, radio buttons, dropdowns. |
| **XFA** | XML Forms Architecture — Adobe's proprietary dynamic form format. Closed-source, no open-source parser exists. Out of scope. |
| **PDF/A** | ISO-standardized PDF subset for long-term archiving. Levels: 1b, 2b, 3b. Self-contained, no external dependencies. |
| **XMP** | Extensible Metadata Platform — XML-based metadata embedded in PDF files (author, title, dates, keywords). |
| **Content Stream** | The sequence of drawing commands in a PDF page. Redaction modifies this stream to remove content. |
| **FlateDecode** | PDF's term for DEFLATE/zlib compression. Most common stream compression in PDFs. |
| **DCTDecode** | PDF's term for JPEG compression. Used for embedded images. |
| **PKCS#12** | File format for storing private keys with certificates (.pfx/.p12). Used for digital signatures. |
| **CSPRNG** | Cryptographically Secure Pseudo-Random Number Generator. Used for encryption keys, nonces. |

## Project Terms

| Term | Definition |
|---|---|
| **P0 / P1 / P2 / P3** | Priority levels. P0 = must-have for v1 release. P3 = nice-to-have, no timeline. |
| **ADR** | Architecture Decision Record — a document capturing a significant architectural decision and its rationale. |
| **Parity** | How closely our output matches Stirling server's output for the same input. We target 95%+. |
| **Fidelity test** | Automated test comparing mobile engine output byte-for-byte against Stirling PDFBox reference output. |
| **Round-trip** | A test pattern: input→process→reverse process→should match original. E.g., split then merge must equal original (page count and content). |
| **Pipeline** | A chain of tool operations applied sequentially to the same file(s) without re-importing. |
| **FileContext** | The Kotlin state manager that tracks active files and pipeline state across tool switches. |
