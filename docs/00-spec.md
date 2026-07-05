# Stirling Mobile — Functional Specification

> **Status:** Draft v1.0
> **Target:** Android 8+ (API 26+), Play Store publication
> **Principle:** 100% offline, zero server dependency, open-source

---

## 1. Product Vision

A mobile-native port of [Stirling PDF](https://github.com/Stirling-Tools/stirling-pdf) for Android. Every PDF operation runs on-device — no cloud, no sign-up, no internet permission required. Same tool breadth as the server version, same open-source values, better performance.

Published to Google Play Store as free software.

---

## 2. User Personas

| Persona | Need | Key workflow |
|---|---|---|
| Student | Merge lecture slides, compress for sharing | Merge → Compress → Share |
| Professional | Sign contracts, redact sensitive info | Sign → Redact → Export |
| Office worker | Convert Office docs to PDF, extract pages | Office→PDF, Split, Reorder |
| Researcher | OCR scanned papers, extract tables | OCR → Extract text/tables |
| Casual user | View PDFs, rotate pages, fill forms | View, Rotate, Form fill |

---

## 3. Functional Requirements

### 3.1 Core PDF Viewer

| ID | Requirement | Priority |
|---|---|---|
| F-001 | Render PDF pages with GPU acceleration | P0 |
| F-002 | Pinch-to-zoom, scroll, page navigation | P0 |
| F-003 | Thumbnail sidebar with page previews | P0 |
| F-004 | Text selection and search within document | P1 |
| F-005 | Jump to page by number | P1 |
| F-006 | Bookmark/navigation pane (PDF outlines) | P2 |

### 3.2 Page Operations

| ID | Requirement | Input | Output | Priority |
|---|---|---|---|---|
| F-010 | Merge multiple PDFs into one | N PDFs | 1 PDF | P0 |
| F-011 | Split PDF by pages/ranges | 1 PDF | N PDFs | P0 |
| F-012 | Rotate pages (0/90/180/270) | 1 PDF | 1 PDF | P0 |
| F-013 | Remove pages | 1 PDF | 1 PDF | P0 |
| F-014 | Extract pages | 1 PDF | 1 PDF | P0 |
| F-015 | Reorder pages (drag-and-drop) | 1 PDF | 1 PDF | P0 |
| F-016 | Crop pages by margin/bbox | 1 PDF | 1 PDF | P1 |
| F-017 | Multi-page layout (N-up) | 1 PDF | 1 PDF | P1 |
| F-018 | Scale pages by percentage | 1 PDF | 1 PDF | P2 |
| F-019 | Adjust contrast/brightness | 1 PDF | 1 PDF | P2 |
| F-020 | Auto-split by size/bookmark/section | 1 PDF | N PDFs | P2 |

### 3.3 Conversion

| ID | Requirement | Input | Output | Priority |
|---|---|---|---|---|
| F-030 | PDF to images (PNG, JPEG, WebP) | PDF | Images | P0 |
| F-031 | Images to PDF | Images | PDF | P0 |
| F-032 | Office to PDF (DOCX, XLSX, PPTX) | Office | PDF | P0 |
| F-033 | PDF to Office (DOCX, XLSX) | PDF | Office | P1 |
| F-034 | HTML to PDF | HTML | PDF | P1 |
| F-035 | Markdown to PDF | MD | PDF | P1 |
| F-036 | PDF to PDF/A (1b, 2b, 3b) | PDF | PDF/A | P1 |
| F-037 | PDF to text extraction | PDF | TXT | P1 |
| F-038 | PDF to CSV (table extraction) | PDF | CSV | P2 |
| F-039 | PDF to XML | PDF | XML | P2 |
| F-040 | EPUB/MOBI to PDF | eBook | PDF | P3 |

### 3.4 Security

| ID | Requirement | Priority |
|---|---|---|
| F-050 | Add password protection (AES-128/256) | P0 |
| F-051 | Remove password protection | P0 |
| F-052 | Digital signature (PKCS#12, visible + invisible) | P1 |
| F-053 | Certify document (signing certificate) | P1 |
| F-054 | Redact content (true removal, not overlay) | P1 |
| F-055 | Sanitize metadata and hidden data | P1 |
| F-056 | Auto-redact by pattern (email, phone, SSN, regex) | P2 |

### 3.5 Edit & Annotate

| ID | Requirement | Priority |
|---|---|---|
| F-060 | Add text to PDF (custom font, size, color) | P1 |
| F-061 | Add image/stamp to PDF | P1 |
| F-062 | Add watermark (text or image, opacity, rotation) | P1 |
| F-063 | Add page numbers (position, format, style) | P1 |
| F-064 | Draw on PDF (freehand, lines, rectangles) | P2 |
| F-065 | Add annotations (highlight, underline, strikethrough, note) | P2 |

### 3.6 OCR

| ID | Requirement | Priority |
|---|---|---|
| F-070 | OCR scanned PDF to searchable PDF | P1 |
| F-071 | OCR with language selection (match Stirling's 40+ languages) | P1 |
| F-072 | OCR to text output (TXT) | P2 |
| F-073 | Handwriting recognition via ML Kit fallback | P2 |

### 3.7 Forms

| ID | Requirement | Priority |
|---|---|---|
| F-080 | Fill AcroForm fields | P1 |
| F-081 | Flatten form fields | P1 |
| F-082 | Extract form field data (JSON/CSV) | P2 |
| F-083 | Create new form fields | P3 |

### 3.8 Compression & Optimization

| ID | Requirement | Priority |
|---|---|---|
| F-090 | Compress PDF (lossy: image downscale/recompress) | P0 |
| F-091 | Compress PDF (lossless: stream optimization) | P1 |
| F-092 | PDF/A conversion and validation | P1 |
| F-093 | PDF linearization (fast web view) | P2 |
| F-094 | Remove duplicate pages | P2 |

### 3.9 Metadata & Info

| ID | Requirement | Priority |
|---|---|---|
| F-100 | View PDF properties (pages, size, author, dates) | P0 |
| F-101 | Edit PDF metadata (title, author, subject, keywords) | P1 |
| F-102 | Auto-rename PDF by content | P2 |
| F-103 | Extract PDF info to JSON | P2 |

### 3.10 Tools (Misc)

| ID | Requirement | Priority |
|---|---|---|
| F-110 | Compare two PDFs (visual diff) | P2 |
| F-111 | Overlay PDFs (watermark-like merging) | P2 |
| F-112 | Detect blank pages | P2 |
| F-113 | Extract images from PDF | P2 |

### 3.11 Workflow

| ID | Requirement | Priority |
|---|---|---|
| F-120 | Multi-tool pipeline (chain tools without re-import) | P1 |
| F-121 | Undo last operation | P1 |
| F-122 | Recent files list | P1 |
| F-123 | Export to device storage / SAF | P0 |
| F-124 | Share sheet integration | P0 |
| F-125 | Open from file manager / SAF | P0 |

---

## 4. Non-Functional Requirements

### 4.1 Performance

| ID | Requirement | Target |
|---|---|---|
| NF-001 | First page render time | < 500ms for 100-page PDF |
| NF-002 | Merge 2x 50MB PDFs | < 5 seconds (benchmarked as "Merge 2 PDFs (NF-002)" in 07-performance-budget.md) |
| NF-003 | Large file support | Up to 500MB, stream-based |
| NF-004 | Memory ceiling | < 512MB active, stable under 1GB |
| NF-005 | APK size (v1) | < 40MB (Rust engine + Tesseract English, no Collabora — this is the only build v1 ships) |
| NF-006 | APK size (v1.1) | < 80MB (adds Collabora SDK when Office↔PDF ships, see 11-roadmap.md) |
| NF-007 | Battery impact (10-min merge session) | < 5% on mid-range device |

### 4.2 Reliability

| ID | Requirement |
|---|---|
| NF-010 | No crash on malformed/corrupted PDF input |
| NF-011 | No data loss — original files never modified |
| NF-012 | Graceful degradation: tool unavailable? show why, don't crash |
| NF-013 | Auto-save unsaved work before process death |

### 4.3 Privacy & Security

| ID | Requirement |
|---|---|
| NF-020 | No INTERNET permission in base app; OCR language-pack download (F-071) ships as a separate dynamic feature module that declares its own INTERNET permission, scoped to tessdata fetch only |
| NF-021 | All files processed in app-private storage only |
| NF-022 | No analytics, no telemetry, no crash reporting SDK |
| NF-023 | Temporary files wiped on session end |
| NF-024 | Android data safety section: "No data collected" |

### 4.4 Accessibility

| ID | Requirement |
|---|---|
| NF-030 | TalkBack support on all interactive elements |
| NF-031 | Minimum touch target 48dp |
| NF-032 | Sufficient color contrast (WCAG AA) |
| NF-033 | Pinch-to-zoom works with accessibility magnification |

### 4.5 Internationalization

| ID | Requirement |
|---|---|
| NF-040 | UI translated to 40+ languages (match Stirling) |
| NF-041 | RTL layout support (Arabic, Hebrew, Farsi) |
| NF-042 | OCR language packs downloadable on-demand |

---

## 5. Out of Scope (v1)

- iOS release (architecture supports it, not built in v1)
- Cloud sync / Google Drive / Dropbox integration
- Real-time collaboration
- Audio/video PDF annotations
- EPUB/MOBI to PDF conversion
- XFA form support (proprietary Adobe format, no open-source parser exists)
- PDF portfolio/collection handling

---

## 6. Acceptance Criteria

Each feature is considered complete when:

1. **Functional**: Produces correct output verified against reference PDFBox output
2. **Performance**: Meets non-functional targets on a 2019 mid-range device (Snapdragon 730G, 4GB RAM)
3. **Stability**: 1000 iterations of the same operation produces no crash or corruption
4. **Accessibility**: Passes TalkBack navigation of the feature's UI
5. **Offline**: Works with airplane mode enabled

**Release gate: all 57 v1 tools (see 04-feature-catalog.md) must individually pass all five criteria above before Play Store submission.** (Originally 50; 7 tools — "Track A", docs/17-track-a-native-tools-status.md — were pulled forward from v1.1 into v1 during W20.) Not a majority, not "the important ones" — 57/57. No phased or partial-tool-set release. This is a hard blocker on Phase 6 (11-roadmap.md) and on Pre-Submission Testing (16-playstore-checklist.md §5), not just a Phase 5 milestone label.

---

## 7. Success Metrics

| Metric | Target |
|---|---|
| Play Store rating | >= 4.5 stars at 6 months |
| Crash-free session rate | >= 99.5% |
| Tool accuracy parity vs Stirling server | >= 95% across all tools |
| APK download size | <= 40MB at v1 launch (NF-005); <= 80MB after v1.1 adds Collabora (NF-006) |
| Monthly active users (6 months) | >= 10,000 |
