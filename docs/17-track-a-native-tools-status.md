# Track A — 7 Native Tools: Status & Findings

> Session snapshot. Written to resume testing/fixes in the next session.
> Context: user decided to pull all 14 deferred v1.1 tools forward before W21
> (see plan `hashed-riding-alpaca.md` / chat history). This covers **Track A**
> only (7 native Rust/uniffi tools, no new SDK). Track B (7 Collabora Office
> tools) has not been started — see "Next session" at the bottom.

## Implemented (Rust + uniffi + Kotlin, wired into MainActivity)

1. PDF to XML — `engine/src/convert_xml.rs` → `convertPdfToXml`
2. PDF to Single HTML — `engine/src/convert_html.rs` → `convertPdfToHtml`
3. Extract Images — `engine/src/extract_images.rs` → `extractImages`
4. Remove Duplicate Pages — `engine/src/dedupe_pages.rs` → `pagesDetectDuplicates` / `pagesRemoveDuplicates`
5. Add Text (positioned) — `engine/src/add_text.rs` → `contentAddText`
6. Draw on PDF — `engine/src/draw.rs` → `contentDraw`, plus new `pdfPageSize` helper
7. Annotations (highlight/underline/strikeout/note) — `engine/src/annotations.rs` → `contentAddAnnotation`

All 3 ABIs (arm64-v8a, armeabi-v7a, x86_64) rebuilt via `cargo ndk`, bindings regenerated. Engine test suite: 89 passed, 0 failed.

## Emulator testing done this session

Used AVD `test_forms` (x86_64). Test fixtures created ad hoc (not committed — regenerate if needed):
- `test_doc.pdf` — LibreOffice-generated (soffice --convert-to pdf from a .txt), embeds a TrueType subset font. **Use this for realistic-PDF testing.**
- `dupe_test.pdf` — `pdfunite test_doc.pdf test_doc.pdf` (2 identical pages, for dedupe testing)
- `image_test.pdf` — `img2pdf` from a magick-generated JPEG (single image page). **Do not use this for round-trip testing of any lopdf-based tool — see Finding 2.**

### Verified working end-to-end (device + byte-level output inspection)
- **Extract Images**: extracted a valid JPEG (confirmed via `file`, correct 200x150 dimensions) from `image_test.pdf`.
- **Remove Duplicate Pages**: `dupe_test.pdf` (2 pages) → detected page 2 as duplicate, removed → output is 1 page (verified with `qpdf --show-npages`).
- **Add Text**: appended `/SMaddF 14 Tf` / `72 700 Td` / `(Stamped) Tj` to page content stream without disturbing existing content (verified by dumping the page content stream).
- **Draw on PDF**: verified via direct Rust chain repro (`dedupe → add_text → draw` on `test_doc.pdf`) — all three steps produce valid, qpdf-clean output with strokes correctly scaled into page-point space. The on-device UI test of this specific tool actually exercised the wrong file (stale file-picker selection landed on `image_test.pdf` instead of `test_doc.pdf`) and hit Finding 2 below — that failure is **not** a Draw-tool bug, see Finding 2.
- **PDF to XML / PDF to HTML**: after Finding 1 was fixed, both now honestly report `[text extraction failed: ...]` per page instead of silently emitting empty content.

### Not yet tested on-device this session
- **Annotations** (highlight/underline/strikeout/note) — only unit-tested (3 Rust tests pass: `adds_highlight_annotation`, `adds_note_with_text`, `rejects_unknown_kind`). Not yet driven through the emulator UI.
- **Draw on PDF** — not yet re-verified through the actual UI on the correct file (only verified via direct Rust repro, see above). Re-run on-device with `test_doc.pdf`, not `image_test.pdf`.

## Findings

### Finding 1 (fixed) — silent empty output on text-extraction failure
`convert_pdf_to_xml` / `convert_pdf_to_html` originally did `doc.extract_text(&[n]).unwrap_or_default()`. On `test_doc.pdf` (LibreOffice-exported, embedded TrueType font), lopdf's `extract_text` fails per-page with `ToUnicode CMap error: Could not parse ToUnicodeCMap` — a real limitation in lopdf 0.34 for this font's CMap encoding. The `unwrap_or_default()` swallowed that into a **silently empty page**, which reads as "this page has no text" instead of "extraction failed."

**Fixed**: both tools now emit `[text extraction failed: {reason}]` in place of the page's text when extraction fails, so the failure is visible instead of silent. This is a partial fix — the underlying lopdf limitation on this font/CMap shape is not fixed, and is likely to affect a meaningful fraction of real-world PDFs (LibreOffice/Word exports commonly embed subsetted TrueType/Type0 fonts). **Next session**: assess how common this failure is across a broader PDF corpus; if it's common, PDF-to-XML/HTML may need a different text-extraction path (e.g. via pdfium, which is already a dependency for rasterization) rather than lopdf's built-in extractor.

### Finding 2 (confirmed, NOT fixed, pre-existing, engine-wide) — lopdf round-trip corrupts some PDFs
While chasing what looked like a Draw-tool bug, isolated a **much bigger, pre-existing issue**: a bare `lopdf::Document::load(path)?.save(output)` — **zero modifications, no calls into any tool code at all** — corrupts `image_test.pdf` (an img2pdf-generated single-image PDF). Confirmed:
- `qpdf --check` on the round-tripped output: `file is damaged`, `xref not found`, forced to reconstruct the xref table.
- lopdf **cannot even reload the file it just wrote** (`Document::load` on the round-tripped output returns `Err(Trailer)`).
- Reproduces with the original file completely unmodified — this is not specific to `content_draw`, `content_add_text`, or any Track A tool. It would hit **any existing engine tool** that loads-and-saves this class of PDF (merge, split, rotate, compress, etc. — all of them).

Root cause not yet investigated — likely lopdf mishandling this PDF's xref format (img2pdf tends to produce compact/modern PDF structures; possibly a cross-reference stream lopdf doesn't fully round-trip). PDF version also gets silently downgraded (1.3 in the output vs whatever img2pdf wrote).

**This is a real risk for the "50/50 (now 64/64) tools must pass 5 criteria" release gate** (docs/00-spec.md:230) — it means some legitimately-produced, valid PDFs will come out corrupted (recoverable by qpdf/most readers' xref-reconstruction fallback, but not a clean file) from tools that have nothing to do with the bug's actual cause. **Next session priorities**:
1. Build a small corpus of PDFs from different producers (img2pdf, LibreOffice, Word, Chrome print-to-PDF, Stirling server itself, scanned/OCR PDFs) and round-trip each through a bare `load→save` to find the actual trigger condition (object streams? xref streams? specific PDF version? linearization?).
2. Decide whether this is fixable via lopdf configuration/API (there may be a save option to avoid whatever's broken) or needs a lopdf version bump / patch / fallback repair step (e.g. shell out to `qpdf --check`+repair on save, since qpdf is already a build dependency via `optimize.rs`'s libqpdf wrapper).
3. This is a **cross-cutting engine risk**, not scoped to Track A — flag to whoever owns the overall roadmap decision before treating any tool as "done" for the 64/64 gate.

## Next session
- [ ] Finish on-device verification: Annotations (untested), Draw on PDF (re-run on `test_doc.pdf`, not `image_test.pdf`)
- [ ] Investigate Finding 2 (lopdf round-trip corruption) — cross-cutting, likely the higher-priority item
- [ ] Decide on Finding 1's long-term fix (pdfium-based text extraction vs. accepting the lopdf limitation)
- [ ] Track B (7 Collabora tools) — not started. Plan already recorded: build Collabora Android viewer from source into a vendored AAR, single universal APK with runtime arm64 feature-gate (see plan file `hashed-riding-alpaca.md` for full detail, ADR-002, and `docs/03-jni-contract.md` §4 for the already-drafted `OfficeConverter` Kotlin interface).
- [ ] Docs/roadmap sync (04-feature-catalog.md, 11-roadmap.md, 14-risk-register.md, 00-spec.md) still pending — hold off until Track A is fully verified and Finding 2 has at least a triage decision, since the "50/50 tools passing" gate language will need to reflect Finding 2's risk either way.
