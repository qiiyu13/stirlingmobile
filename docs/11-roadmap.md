# Stirling Mobile — Roadmap

> **Status:** Draft v1.2 — revised for AI-orchestrated solo development
> **Team size:** 1 human (orchestrator + reviewer) directing AI agents for implementation. See P2, 14-risk-register.md.
> **Model:** AI writes code; human reviews, verifies on real devices, and makes judgment calls (security review, PDF fidelity, UX). Week estimates below compress the *writing* phases, not the *verification* phases — see §Timeline for why.
> **v1 scope cut:** Collabora Office↔PDF (7 tools) deferred to v1.1; qpdf handled via `libqpdf` NDK wrapper from day one, not a custom rewrite; OCR ships English-only, other languages post-v1. See 04-feature-catalog.md for the full v1/v1.1 split.

---

## Phase 0: Foundation (Weeks 1-3)

**Goal:** Rust engine skeleton + Kotlin app scaffold + JNI bridge working

| Week | Deliverable |
|---|---|
| W1 | Rust workspace setup: `engine/` crate, cargo-ndk build, CI with cargo test. Android project scaffold: Gradle, Compose, navigation shell, empty screens. AI generates both in parallel; human wires them together and reviews the JNI boundary. |
| W2 | Rust PDF parser: lopdf integration, parse PDF, extract metadata, count pages. JNI bridge proof-of-concept: `engine_init()` and `pdf_get_info()` working end-to-end. |
| W2-3 | Rust PDF render: pdfium integration, render page to PNG, get thumbnail. Kotlin viewer screen: pinch-to-zoom, scroll, thumbnails. |
| W3 | File repository: SAF import, app-private storage, temp lifecycle, Room DB. Home screen: recent files, tool grid, empty states. |
| **Milestone:** PDF viewer working, file import/export working, app navigable |

---

## Phase 1: Core Tools (Weeks 4-8)

**Goal:** P0 tools (merge, split, rotate, extract, compress, password) shipped

| Week | Deliverable |
|---|---|
| W4 | `pages_merge`, `pages_split`, `pages_rotate`, `pages_remove`, `pages_extract` — Rust + tests. Matching screens — Kotlin UI + ViewModels. AI generates tool + screen + test together per tool; human reviews each PR. |
| W5 | `optimize_compress` (lossy), `security_add_password`, `security_remove_password` — Rust + screens |
| W6-7 | Integration testing: 1000 iterations of each P0 tool, no crashes. AI writes the fuzz/property harness; runs unattended, human reviews failures only. |
| W7-8 | Fidelity testing: output vs Stirling PDFBox reference. **Human bottleneck**: visually verifying PDF output isn't corrupted can't be delegated to the AI that wrote the code — needs independent eyes. |
| **Milestone:** 7 tools working, verified, production-quality |

---

## Phase 2: Conversion Tools (Weeks 9-10)

**Goal:** PDF↔image, image→PDF, MD→PDF, HTML→PDF — no Collabora/Office in v1

| Week | Deliverable |
|---|---|
| W9 | `convert_pdf_to_images`, `convert_images_to_pdf`, `convert_markdown_to_pdf` — Rust + screens |
| W9-10 | Android WebView HTML→PDF converter — Kotlin |
| **Milestone:** Non-Office conversion tools (4 tools) working. Office↔PDF (Collabora, 7 tools) explicitly out of v1 — see 04-feature-catalog.md |

---

## Phase 3: Security & Content Tools (Weeks 11-14)

**Goal:** Sign, redact, watermark, page numbers, sanitize, metadata

| Week | Deliverable |
|---|---|
| W11 | `security_sign`, `security_certify` — Rust (ring + rcgen) + screens, PFX file handling |
| W12 | `content_redact`, `content_auto_redact` — Rust + redaction area picker UI |
| W13 | `content_watermark_text`, `content_watermark_image`, `content_page_numbers` — Rust + screens |
| W13-14 | `security_sanitize`, `metadata_edit`, `metadata_extract` — Rust + screens. **Human bottleneck**: crypto/signing code (ring, rcgen, PKCS#12) gets careful manual security review regardless of who wrote it — this isn't optional review, per 09-security.md. |
| **Milestone:** Security & content tools (11 tools) working |

---

## Phase 4: OCR & Forms (Weeks 15-18)

**Goal:** Tesseract integration (English only), OCR PDF, form fill/flatten

| Week | Deliverable |
|---|---|
| W15-16 | Tesseract NDK integration: compile for Android, JNI bridge, English `traineddata` bundled. **Human bottleneck**: native crashes in a C library need on-device repro and bisection — AI can propose fixes but can't run the physical device cycle. |
| W16-17 | OCR PDF tool: Rust orchestrates Tesseract, overlays text layer. OCR screen: progress, preview — Kotlin. No language picker in v1; download-pack mechanism deferred to v1.2. |
| W17-18 | `forms_fill`, `forms_flatten`, `forms_extract_data` — Rust + screens |
| **Milestone:** OCR (English) + forms (5 tools) working |

---

## Phase 5: Advanced & Polish (Weeks 19-23)

**Goal:** Remaining tools, pipeline, polish, testing, Play Store prep

| Week | Deliverable |
|---|---|
| W19 | ✅ `pages_reorder`, `pages_n_up`, `pages_crop`, `pages_scale`, `tool_compare`, `tool_overlay` — Rust + screens (all 6 done, verified on x86_64 emulator) |
| W19-20 | ✅ `optimize_lossless` via `libqpdf` NDK wrapper (ADR-003 Path A, decided upfront — no custom rewrite, no go/no-go checkpoint needed; qpdf+zlib+libjpeg cross-compiled for all 3 ABIs, verified on x86_64 emulator) |
| W20 | ✅ Pipeline system: FileContext + undo wired into all 22 applicable tool screens (multi-tool chain via auto-consume/push), Kotlin. |
| W20 | ✅ PDF/A conversion + validation — Rust (`convert_pdf_to_pdfa`/`pdfa_validate`: OutputIntent + hand-built sRGB ICC profile, XMP `pdfaid` metadata, font-embedding check). |
| W20 | ✅ 7 native tools pulled forward from v1.1 ("Track A" — PDF to XML/HTML, Add text, Draw on PDF, Annotations, Extract images, Remove duplicate pages), all native Rust/uniffi, no Collabora dependency. See docs/17-track-a-native-tools-status.md. |
| W21 | Performance optimization: profiling, memory tuning, benchmark pass. **Human bottleneck**: judging whether a benchmark regression matters requires product judgment, not just code review. |
| W21 | Accessibility audit: TalkBack pass on all screens. **Human bottleneck**: actually using a screen reader end-to-end, not something to delegate. |
| W22 | i18n: strings extracted, 3-5 languages translated (not the full 40 — see 10-i18n.md; remainder is a v1.1 community-translation task) |
| W22-23 | Play Store assets: screenshots, feature graphic, description, privacy policy, trademark/naming check (M4, 14-risk-register.md) |
| W23 | Internal alpha, bug bash |
| **Release gate (hard blocker, not just a milestone):** all 57 v1 tools individually pass every acceptance criterion in 00-spec.md §6. If any tool fails, Phase 6 does not start for the app as a whole — fix and re-verify that tool, don't ship around it. No partial/staged tool rollout for v1. |

---

## Phase 6: Launch (Weeks 24-26)

**Goal:** Real users, real feedback — this phase is calendar-bound, not code-writing-bound. AI orchestration doesn't compress it: testers take real days to install, use, and report bugs.

**Entry condition:** the Phase 5 release gate above is met — 57/57 tools passing. If it slips, Phase 6 slips with it; weeks below are not a fixed calendar commitment independent of that gate.

| Week | Deliverable |
|---|---|
| W24-25 | Closed alpha: small tester group, bug fixes |
| W25 | Open beta: Play Store open testing track |
| W26 | Production release: submit for review |

---

## Post-Launch Roadmap

### v1.1 (Months 6-8 post-launch-start)
- Collabora Office SDK integration: Office↔PDF (7 tools) — the only tools remaining in v1.1 scope (the other 7 originally listed here — PDF to XML/HTML, Add text, Draw on PDF, Annotations, Extract images, Remove duplicate pages — were pulled forward into v1 during W20, see 04-feature-catalog.md and docs/17-track-a-native-tools-status.md). Budget 6-8 weeks: AI compresses the integration glue, but the SDK-bug-hunting risk (R2) is unchanged by AI orchestration — that's still on-device native debugging, same as Phase 4's Tesseract bottleneck

### v1.2
- Download additional OCR languages on demand (mechanism already built in v1; this adds the language picker + remaining `traineddata` packs)
- Remaining i18n languages beyond the v1 3-5, ideally via community translation (Crowdin, 10-i18n.md) rather than solo effort
- Auto-rename by content
- PDF linearization (fast web view)
- EPUB/MOBI to PDF
- Create new form fields

### v2.0
- iOS release (SwiftUI + shared Rust engine)
- XFA form support (if open-source parser found)
- AI-powered features (on-device via MediaPipe/ML Kit, not cloud)

---

## Timeline

**Why phases compress unevenly.** AI-orchestrated development speeds up code that's well-specified and reviewable: Rust tool implementations, Kotlin screens, JNI glue, test harnesses. It does not speed up work gated on a human doing something only a human can do:

| Bottleneck type | Compresses with AI? | Where it shows up |
|---|---|---|
| Writing new code to spec | Yes, significantly | Phases 0-3, most of Phase 5 |
| Automated test/fuzz runs | Yes | Phase 1 integration testing |
| Visual/manual PDF fidelity check | No — needs independent human eyes | Phase 1, ongoing |
| Security review of crypto/signing code | No — mandatory regardless of author | Phase 3 |
| On-device native crash debugging | Little — still requires physical repro cycles | Phase 4 (Tesseract), v1.1 (Collabora) |
| Accessibility/UX judgment | No | Phase 5 |
| Real-user alpha/beta feedback loops | No — calendar-bound, not code-bound | Phase 6 |

| Scope | Weeks | Months |
|---|---|---|
| v1 (this roadmap, Collabora cut) | ~26 weeks | ~6.5 months |
| + risk buffer (native-debugging bottlenecks, first-time NDK toolchain friction: +15%) | ~30 weeks | **~7 months** |

For comparison: the original 2-3 human-dev plan estimated 26/34 weeks for a *larger* scope (including Collabora). The hand-typed-solo estimate was ~36-41 weeks for the *cut* scope. AI orchestration brings the cut-scope timeline back down to roughly the original team estimate — not because AI eliminates the bottlenecks in the table above, but because it removes the "one person hand-typing everything else" tax that hand-typed-solo carried on top of them.

**Re-adding Collabora to v1** still isn't recommended — its bottleneck (on-device native SDK debugging) is the one AI orchestration compresses least. Better to land it as v1.1 once the core 50 tools are stable, per the shortened post-launch timeline above.
