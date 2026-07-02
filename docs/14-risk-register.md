# Stirling Mobile — Risk Register

> **Status:** Draft v1.0
> **Review cadence:** Weekly during active development

| Probability | Impact | Action |
|---|---|---|
| Low (<20%) | Minor (days delay) | Monitor |
| Medium (20-60%) | Moderate (weeks delay) | Mitigate |
| High (>60%) | Critical (blocker) | Contingency plan |

---

## Technical Risks

| # | Risk | P | I | Mitigation | Contingency |
|---|---|---|---|---|---|
| R1 | **lopdf can't handle all PDF variants** — PDF spec is massive (ISO 32000), edge cases abound | M | C | Start with PDFBox reference tests, build corpus of 1000+ real-world PDFs | Fall back to compiled PDFBox on Android, drop iOS compatibility for parsing |
| R2 | **Collabora Office SDK has blocking bugs** — LibreOffice on Android may have rendering glitches | M | M | **v1: N/A, Collabora deferred to v1.1** (see 11-roadmap.md). When picked up: test with diverse Office documents early, file bugs upstream. | Use Rust-native office renderer (calamine + printpdf) for 80% of docs, Collabora for complex ones only |
| R3 | **`libqpdf` NDK build/link complexity** — v1 wraps `libqpdf` directly (ADR-003 Path A, decided upfront, not a fallback) rather than writing a custom Rust stream optimizer; risk shifts from "algorithm misses cases" to "cross-compiling a C++ library for NDK is fiddly" | L | M | Prototype the NDK build in Phase 0, before it's on the critical path in Phase 5. Pin a known-good qpdf version. | If NDK build proves unworkable: fall back to lossy-only optimization for v1, defer lossless to v1.1 |
| R4 | **Tesseract NDK crashes on some PDF pages** — native memory issues from graphics-heavy pages | L | M | Pre-process PDF pages to normalize image format before OCR. Test with scanned book corpus. | Use Google ML Kit for OCR fallback (on-device, included in Play Services) |
| R5 | **WebView HTML→PDF has CSS limitations** — complex CSS (flexbox, grid) may not render in print mode | L | L | Test with WeasyPrint's own test suite. Document known CSS limitations. | Accept limitations. Most HTML-to-PDF use cases are simple documents. |
| R6 | **Rust JNI overhead causes latency** — frequent JNI crossings add up for per-page operations | L | M | Batch operations: send all pages at once, not one-by-one. Use file paths for large data. | Profile and optimize. JNI overhead is typically <1μs per call — negligible vs PDF processing time. |
| R7 | **APK too large for Play Store** — users may not download a large app on mobile data | L | M | **v1: largely resolved by the Collabora cut** — v1 ships a single ~36-40MB build (02-tech-stack.md §5), no lite/full split needed. Use Android App Bundle + dynamic delivery for OCR languages (v1.2+). | v1.1, when Collabora is added back: reintroduce lite/full variants, or ship Collabora as a Play Feature Delivery on-demand module instead of baking it into a separate APK variant |
| R8 | **Memory leaks in Rust engine** — long-running sessions accumulate unfreed PDF objects | L | C | `Drop` implementations rigorously tested. Integration test: process 1000 PDFs, watch memory. | Periodic engine restart between tool chains (acceptable workaround). |
| R9 | **Concurrency bugs in Rust** — `RwLock` deadlocks or data races | L | C | Single-threaded engine for v1 (no `rayon`). Add parallelism only after benchmarks prove need. | If parallelism needed: use message-passing (channels) not shared state. |
| R10 | **`:engine` process death/restart** — the engine runs in a separate process (01-architecture.md §2.2); OS can kill it under memory pressure mid-operation, AIDL binding can drop | M | M | Bound service with `START_REDELIVER_INTENT`, checkpoint long operations so they can resume or fail cleanly. Integration test: kill `:engine` process mid-op, verify Kotlin recovers. | Collapse to single-process if process-death handling proves unreliable in practice; lose the memory-isolation benefit but simplify. |
| R11 | **Build/QA matrix complexity** — v1: 1 build variant × 3 ABIs × 2 API tiers (26-29 vs 30+) = 6 configurations. v1.1 reintroduces full/lite, doubling to 12 once Collabora returns | L (v1) / M (v1.1) | M | CI matrix covers all combinations (12-dev-environment.md). Prioritize arm64 + API 30+ for manual QA, treat others as CI-only gates. | Drop armv7 support if CI/QA cost is unsustainable — arm64 covers the vast majority of active devices. |

---

## Project Risks

| # | Risk | P | I | Mitigation |
|---|---|---|---|---|
| P1 | **Feature creep** — trying to match all 50+ Stirling tools delays v1 | M | H | Strict P0/P1/P2 prioritization in spec. Deferred list is explicit. |
| P2 | **Review/verification bottleneck** — 11-roadmap.md's ~30-week (~7 month) estimate assumes 1 human orchestrating AI-written code; AI compresses the code-writing phases but not the human-bound ones (PDF fidelity checks, crypto/security review, on-device native debugging, a11y audits, real alpha/beta feedback loops — see §Timeline table in 11-roadmap.md). If those bottlenecks run longer than estimated, or the human reviewer can't keep pace with AI-generated PR volume, timeline slips on exactly those phases, not the AI-compressed ones. | M | H | Track time actually spent in each bottleneck category weekly; if review is falling behind generation, slow down generation rather than let review quality slip. Phase-based milestones with clear "done" criteria. Cut scope (defer more tools to v1.1) before stretching timeline. |
| P3 | **Play Store rejection** — policy violation, content rating issue | L | M | Review Play Store policies before development. No network permission simplifies compliance. |
| P4 | **Stirling-PDF adds features during development** — moving target | M | L | Track Stirling changelog. New server features go into mobile v1.1, not v1 scope creep. |

---

## Market Risks

| # | Risk | P | I | Mitigation |
|---|---|---|---|---|
| M1 | **Adobe Scan / Acrobat dominance** — users default to Adobe | H | L | Differentiate: zero-network privacy, open-source, full tool suite. Target privacy-conscious users. |
| M2 | **Low Play Store discoverability** — can't compete with Adobe's SEO | H | M | Leverage Stirling PDF's existing community (85k GitHub stars, Discord, Reddit). Cross-promote. |
| M3 | **iOS demand overwhelms Android-only launch** — users complain | M | L | Clear messaging: "Android first, iOS in development." Share roadmap publicly. |
| M4 | **Trademark risk using the "Stirling" name** — this is an independent mobile port, not an official Stirling-Tools release; shipping as "Stirling PDF Mobile" (16-playstore-checklist.md) on the Play Store without confirmed trademark/naming permission from the upstream project risks a takedown or rebrand after launch | M | H | Contact Stirling-Tools maintainers before Play Store submission to confirm naming is acceptable, or secure written permission. Check upstream LICENSE/trademark policy now, before the package name (`com.stirlingmobile`) and store listing are locked in. | If permission denied or unclear: rebrand before submission (e.g. "PDF Toolkit for Stirling" as a compatible/companion app, or a fully independent name) — much cheaper pre-launch than post-launch. |

---

## Risk Review Schedule

- **Weekly**: Team reviews top risks during standup
- **Per-phase**: Full risk register review at phase boundaries
- **Pre-release**: Security risk deep-dive before Play Store submission
- **Trigger-based**: Any risk materializes → immediate review + contingency activation
