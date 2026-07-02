# ADR-002: Collabora Office SDK for Office↔PDF

**Status:** Accepted, **v1.1-scoped** — see 11-roadmap.md
**Date:** 2026-07-02
**Deciders:** Stirling Mobile team

**Update:** Solo/AI-orchestrated v1 defers this decision's implementation entirely (effectively Option D for v1's launch scope) — not because Option A was wrong, but because its risk (SDK bugs, native debugging) is the one bottleneck AI orchestration doesn't compress, so it's scheduled for v1.1 instead of blocking launch. See P2/R2 in 14-risk-register.md. The decision below (Option A) still stands as the long-term plan.

## Context

Stirling server uses LibreOffice (via CLI) for Office↔PDF conversions (DOCX, XLSX, PPTX to/from PDF). On mobile, we cannot call LibreOffice as a CLI process. We need an on-device, 100% offline alternative with equal quality.

## Decision

Bundle the **Collabora Office SDK** (LibreOffice compiled for Android) for Office↔PDF conversions.

## Options Considered

### Option A: Collabora Office SDK (LibreOffice for Android)
- **Pros**: Same LibreOffice rendering engine Stirling server uses → 100% output parity. MPL 2.0 license (Play Store compatible). Proven on Android (Collabora Office is on Play Store). Supports all formats (DOCX, XLSX, PPTX, ODT, ODS, ODP).
- **Cons**: ~40MB APK size impact. ARM64 only (no 32-bit support). Third-party SDK dependency with potential bugs.

### Option B: Rust-native office renderer
- **Pros**: Smaller binary, full control, no external dependency, cross-ABI
- **Cons**: Massive engineering effort (building a typesetting engine from scratch). Lower fidelity on complex documents (nested tables, custom fonts, charts, SmartArt). Would take 6+ months just for this feature.

### Option C: Apache POI on Android + printpdf rendering
- **Pros**: POI already in Stirling's Java deps, reads office formats well
- **Cons**: No page layout engine — POI reads content but doesn't render. Would need to build layout from scratch. Java GC issues on large documents.

### Option D: No office conversions (out of scope)
- **Pros**: Zero cost, smaller APK
- **Cons**: Major feature gap vs Stirling PDF. Users expect Office↔PDF on mobile.

## Consequences

### Positive
- 100% output parity with Stirling server for Office conversions
- All format support (legacy .doc, .xls, .ppt; modern .docx, .xlsx, .pptx; ODF)
- Battle-tested by millions of Collabora Office users on Android
- Same license family as other MPL 2.0 deps (resvg, uniffi)

### Negative
- 40MB APK size increase in v1.1, when this ships (v1 carries none of this cost — see update above)
- ARM64-only (32-bit and x86 devices will need a lite variant without office conversion, reintroduced in v1.1 — see 02-tech-stack.md §5)
- SDK updates must be tracked for CVEs — manual update process
- Adds C++ dependency to an otherwise Rust/Kotlin stack

### Mitigations
- v1.1 reintroduces `fullRelease`/`liteRelease` variants (v1 ships neither — one Collabora-free build only, see 02-tech-stack.md §5)
- Android App Bundle splits by ABI, so users on ARM64 get the SDK, others get lite automatically
- Collabora Office SDK updates via Renovate/Dependabot automation
- Integration tests specifically for office conversion fidelity
