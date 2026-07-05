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

### Finding 2 (fixed, engine-wide) — lopdf round-trip corrupts some PDFs

**Root cause found and fixed this session.** lopdf 0.34's `Document::save()` flattens the whole document into one fresh xref table but never clears a pre-existing `trailer["Prev"]` (and `trailer["XRefStm"]`) inherited from the source file's incremental-update / linearization chain. That stale offset — a byte position in the *original* file — survives verbatim into the newly written (differently-sized) file, where it points at nothing. Confirmed via a standalone repro (bare `lopdf::Document::load(path)?.save(output)`, zero tool code involved) on `image_test.pdf` (img2pdf-generated, linearized single-image PDF):
- Source trailer: `/Prev 1907 ... /Size 9` (2 xref sections — linearized first-page xref + main xref).
- Round-tripped output was only 1665 bytes but still carried `/Prev 1907` — `qpdf --check` → `file is damaged`, `xref not found at offset 1907`, forced to reconstruct. lopdf itself can't reload its own output (`Xref(PrevStart)`).
- Not specific to any Track A tool or `content_draw` — every engine tool that loads-and-saves this class of PDF was affected (merge, split, rotate, compress, etc.), since none of the ~30 `doc.save()` call sites across `engine/src/*.rs` went through a shared save path.

**Fix**: added `save_document(doc, path)` to `engine/src/content_util.rs` — strips `Prev`/`XRefStm` from the trailer before delegating to `doc.save()` — and swapped every lopdf `Document::save()` call site in the engine (~30 sites across add_text, annotations, auto_redact, compress, convert/convert_html/convert_xml, dedupe_pages, draw, extract_images, forms, merge, metadata, ocr, optimize, overlay, page_numbers, pages, pdfa, pfx_generate, rasterize, redact, rotate, sanitize, security, sign, split, stamp, watermark) to route through it. Non-Document `.save()` calls (image crate: extracted images, rasterized pages, diff images) were left untouched — not affected by this bug.

Chose a local helper over patching/vendoring lopdf itself: a fork means maintaining and re-patching lopdf on every version bump; a few lines in code we already own is smaller, removable if upstream ever fixes it, and reuses the existing `content_util.rs` shared-helpers module rather than adding a new one.

Verified: engine test suite still 89 passed / 0 failed (no regressions), and the original repro fixture now round-trips clean — `qpdf --check` passes with no reconstruction warning, and lopdf can reload its own output.

**Still open**: only `image_test.pdf` (img2pdf/linearized) was used to find and confirm the root cause. Root cause (stale `Prev`/`XRefStm`) is general enough that it should cover other producers with incremental-update chains (scanned/OCR PDFs, some Word/Acrobat exports), but that's inference, not tested. If time allows, build a small multi-producer corpus (LibreOffice, Word, Chrome print-to-PDF, Stirling server, scanned/OCR) and round-trip each to confirm no other corruption trigger exists.

## Next session
- [ ] Finish on-device verification: Annotations (untested), Draw on PDF (re-run on `test_doc.pdf`, not `image_test.pdf`)
- [x] Investigate Finding 2 (lopdf round-trip corruption) — root cause found, fixed via `save_document()` in `content_util.rs`, all call sites migrated, 89 tests still passing
- [ ] (optional) Broaden Finding 2 verification to a multi-producer PDF corpus, since only img2pdf/linearized was confirmed
- [ ] Decide on Finding 1's long-term fix (pdfium-based text extraction vs. accepting the lopdf limitation)
- [ ] Track B (7 Collabora tools) — not started. Plan already recorded: build Collabora Android viewer from source into a vendored AAR, single universal APK with runtime arm64 feature-gate (see plan file `hashed-riding-alpaca.md` for full detail, ADR-002, and `docs/03-jni-contract.md` §4 for the already-drafted `OfficeConverter` Kotlin interface).
- [x] Docs/roadmap sync (04-feature-catalog.md, 11-roadmap.md, 14-risk-register.md, 00-spec.md, 16-playstore-checklist.md) done: all 7 Track A tools flipped ✅ in the catalog, moved out of the v1.1 deferred list into v1, gate language updated 50/50 → 57/57, R12 added to risk register (resolved).
