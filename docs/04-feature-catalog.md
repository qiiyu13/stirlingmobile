# Stirling Mobile — Feature Catalog

> **Status:** Draft v1.0
> **Mapping:** Every Stirling PDF tool → Mobile implementation status

---

## Legend

| Symbol | Meaning |
|---|---|
| ✅ | Implemented in Rust engine — v1 |
| 📱 | Implemented in Kotlin (Android APIs) — v1 |
| 🤝 | Collabora Office SDK — **cut from v1, deferred to v1.1** (solo-dev scope cut, see P2/M4 in 14-risk-register.md and 11-roadmap.md) |
| ⏳ | Planned, not started — v1.1+ |

---

## 1. Page Operations

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 1 | Merge PDFs | `/api/v1/general/merge-pdfs` | ✅ `pages_merge` | PageOps |
| 2 | Split PDF | `/api/v1/general/split-pages` | ✅ `pages_split` | PageOps |
| 3 | Split by Size | `/api/v1/general/split-by-size` | ✅ `pages_split` | PageOps |
| 4 | Rotate Pages | `/api/v1/general/rotate-pages` | ✅ `pages_rotate` | PageOps |
| 5 | Remove Pages | `/api/v1/general/remove-pages` | ✅ `pages_remove` | PageOps |
| 6 | Extract Pages | `/api/v1/general/extract-pages` | ✅ `pages_extract` | PageOps |
| 7 | Reorder Pages | `/api/v1/general/reorder-pages` | ✅ `pages_reorder` | PageOps |
| 8 | Crop Pages | `/api/v1/general/crop` | ✅ `pages_crop` | PageOps |
| 9 | Multi-Page Layout | `/api/v1/general/multi-page-layout` | ✅ `pages_n_up` | PageOps |
| 10 | Scale Pages | `/api/v1/general/scale-pages` | ✅ `pages_scale` | PageOps |
| 11 | Adjust Contrast | `/api/v1/misc/adjust-contrast` | ✅ ContentOps | ContentOps |
| 12 | Auto-Split PDF | `/api/v1/general/auto-split-pdf` | ✅ `pages_split` | PageOps |
| 13 | Split by Sections | `/api/v1/general/split-by-sections` | ✅ `pages_split` | PageOps |

## 2. Conversion

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 14 | PDF to Images | `/api/v1/convert/pdf-to-img` | ✅ `convert_pdf_to_images` | Convert |
| 15 | Images to PDF | `/api/v1/convert/img-to-pdf` | ✅ `convert_images_to_pdf` | Convert |
| 16 | PDF to Word | `/api/v1/convert/pdf-to-docx` | 🤝 Collabora | Office (v1.1) |
| 17 | PDF to Excel | `/api/v1/convert/pdf-to-xlsx` | 🤝 Collabora | Office (v1.1) |
| 18 | PDF to PPTX | `/api/v1/convert/pdf-to-pptx` | 🤝 Collabora | Office (v1.1) |
| 19 | Word to PDF | `/api/v1/convert/docx-to-pdf` | 🤝 Collabora | Office (v1.1) |
| 20 | Excel to PDF | `/api/v1/convert/xlsx-to-pdf` | 🤝 Collabora | Office (v1.1) |
| 21 | PPTX to PDF | `/api/v1/convert/pptx-to-pdf` | 🤝 Collabora | Office (v1.1) |
| 22 | HTML to PDF | `/api/v1/convert/html-to-pdf` | 📱 WebView | Convert |
| 23 | Markdown to PDF | `/api/v1/convert/md-to-pdf` | ✅ `convert_markdown_to_pdf` | Convert |
| 24 | PDF to Text | `/api/v1/convert/pdf-to-text` | ✅ `pdf_extract_text` | View |
| 25 | PDF to CSV | `/api/v1/convert/pdf-to-csv` | ✅ `convert_pdf_to_csv` | Convert |
| 26 | PDF to XML | `/api/v1/convert/pdf-to-xml` | ⏳ | Convert |
| 27 | PDF to PDF/A | `/api/v1/misc/pdf-to-pdfa` | ✅ `convert_pdf_to_pdfa` | Convert |
| 28 | PDF/A Validation | `/api/v1/misc/validate-pdfa` | ✅ `convert_pdfa_validate` | Convert |
| 29 | PDF to Single HTML | `/api/v1/convert/pdf-to-html` | ⏳ | Convert |
| 30 | File to PDF (generic) | `/api/v1/convert/file-to-pdf` | 🤝 Collabora | Office (v1.1) |

## 3. Security

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 31 | Add Password | `/api/v1/security/add-password` | ✅ `security_add_password` | Security |
| 32 | Remove Password | `/api/v1/security/remove-password` | ✅ `security_remove_password` | Security |
| 33 | Change Permissions | `/api/v1/security/change-permissions` | ✅ Security | Security |
| 34 | Sign PDF | `/api/v1/security/sign` | ✅ `security_sign` | Security |
| 35 | Certify PDF | `/api/v1/security/cert-sign` | ✅ `security_certify` | Security |
| 36 | Redact PDF | `/api/v1/security/redact` | ✅ `content_redact` | ContentOps |
| 37 | Auto Redact | `/api/v1/security/auto-redact` | ✅ `content_auto_redact` | ContentOps |
| 38 | Sanitize PDF | `/api/v1/security/sanitize-pdf` | ✅ `security_sanitize` | Security |

## 4. Edit & Annotate

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 39 | Add Watermark (Text) | `/api/v1/misc/add-watermark` | ✅ `content_watermark_text` | ContentOps |
| 40 | Add Watermark (Image) | `/api/v1/misc/add-image-watermark` | ✅ `content_watermark_image` | ContentOps |
| 41 | Add Page Numbers | `/api/v1/misc/add-page-numbers` | ✅ `content_page_numbers` | ContentOps |
| 42 | Add Image/Stamp | `/api/v1/misc/add-image` | ✅ `content_add_image` | ContentOps |
| 43 | Add Text | `/api/v1/misc/add-text` | ⏳ | ContentOps |
| 44 | Draw on PDF | — | ⏳ | ContentOps |
| 45 | Annotations | — | ⏳ | ContentOps |

## 5. OCR

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 46 | OCR PDF | `/api/v1/misc/ocr-pdf` | ✅ Tesseract NDK | OCR |
| 47 | OCR to Text | — | ✅ Tesseract NDK | OCR |

v1 (solo): English-language pack bundled only. Additional languages (of the 40+ listed in 10-i18n.md §6) are downloadable post-v1 — the download mechanism ships in v1, the language-pack QA matrix does not.

## 6. Forms

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 48 | Fill Forms | `/api/v1/misc/fill-forms` | ✅ `forms_fill` | Forms |
| 49 | Flatten Forms | `/api/v1/misc/flatten-forms` | ✅ `forms_flatten` | Forms |
| 50 | Extract Form Data | — | ✅ `forms_extract_data` | Forms |

## 7. Compression & Optimization

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 51 | Compress PDF | `/api/v1/misc/compress-pdf` | ✅ `optimize_compress` | Optimize |
| 52 | Optimize Lossless | — | ✅ `optimize_lossless` | Optimize |

v1 (solo): implemented as a thin Rust wrapper over `libqpdf` compiled for NDK (ADR-003 Path A), decided upfront rather than attempting a custom stream optimizer first. Cuts the R3 rewrite-risk and the Week-22 go/no-go checkpoint out of the plan entirely — see 11-roadmap.md.
| 53 | Remove Blank Pages | — | ✅ `pages_remove_blank` | PageOps |
| 54 | Detect Blank Pages | — | ✅ `pages_detect_blank` | PageOps |
| 55 | Remove Duplicates | — | ⏳ | PageOps |

## 8. Metadata & Info

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 56 | Get Info | `/api/v1/misc/get-info` | ✅ `pdf_get_info` | View |
| 57 | Edit Metadata | `/api/v1/misc/update-metadata` | ✅ `metadata_edit` | Metadata |
| 58 | Auto Rename | `/api/v1/misc/auto-rename` | ✅ `tool_auto_rename` | Metadata |
| 59 | Extract Images | `/api/v1/misc/extract-images` | ⏳ | Convert |

## 9. Compare & Overlay

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 60 | Compare PDFs | `/api/v1/general/compare` | ✅ `tool_compare` | Compare |
| 61 | Overlay PDFs | `/api/v1/general/overlay` | ✅ `tool_overlay` | Compare |

## 10. Workflow

| # | Tool Name | Stirling Endpoint | Mobile Impl | Module |
|---|---|---|---|---|
| 62 | Pipeline | `/api/v1/pipeline` | 📱 Kotlin FileContext | Workflow |
| 63 | Undo | — | 📱 Kotlin FileContext | Workflow |
| 64 | Recent Files | — | 📱 Room DB | UX |

---

## Summary

| Status | Count |
|---|---|
| ✅ Rust engine — ships v1 | 46 |
| 📱 Kotlin / Android API — ships v1 | 4 |
| 🤝 Collabora Office SDK — deferred v1.1 | 7 |
| ⏳ Planned, not started — deferred v1.1 | 7 |

Total: 64 rows in this catalog (including sub-variants that split one Stirling server endpoint into several mobile-side tools; the Stirling server itself has ~55 distinct endpoints, not directly comparable 1:1 to this count).

**v1 (solo) coverage: 50/64 (78%)** — ✅46 + 📱4. Collabora (Office↔PDF) is cut from v1 entirely; it's a multi-week LibreOffice-on-Android integration that doesn't pay for itself on a solo timeline. See P2/R2/M4 in 14-risk-register.md and 11-roadmap.md.

14 tools deferred to v1.1:
- PDF to Word / Excel / PPTX (Collabora)
- Word / Excel / PPTX to PDF (Collabora)
- File to PDF, generic (Collabora)
- PDF to XML
- PDF to HTML
- Add text (positioned)
- Draw on PDF
- Annotation tools (highlight, underline, strikethrough, note)
- Extract images from PDF
- Remove duplicate pages
