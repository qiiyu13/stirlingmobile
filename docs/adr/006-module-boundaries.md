# ADR-006: Module Boundaries — Rust vs Kotlin

**Status:** Accepted
**Date:** 2026-07-02
**Deciders:** Stirling Mobile team

## Context

With a hybrid architecture (Rust engine + Kotlin UI), we must decide what logic lives in each layer. Wrong placement causes: duplicated code, unnecessary JNI overhead, or UI thread blocking.

## Decision

Follow the **"Rust owns the PDF, Kotlin owns the user"** principle.

## Boundary Rules

### Rust owns:

| Concern | Reason |
|---|---|
| PDF parsing and writing | lopdf is Rust-native. No PDF byte-level logic in Kotlin, **except** the two exemptions below. |
| All native-path PDF transformations | The core value prop — must be cross-platform (iOS). |
| Image processing | `image` crate, not Android BitmapFactory. iOS future. |
| Cryptographic operations | `ring` crate for security-critical code. |
| OCR orchestration | Tesseract is C, called from Rust via FFI. |
| PDF/A validation | PDF spec logic — complex, cross-platform. |

### Kotlin owns:

| Concern | Reason |
|---|---|
| UI (all screens) | Compose is Android-only by design. iOS gets SwiftUI rewrite. |
| File management (SAF, storage) | Android platform APIs. iOS will use different file APIs. |
| User preferences | DataStore is Android-only. |
| Navigation and lifecycle | Compose Navigation, Android lifecycle. |
| Sharing and intents | Android platform API. |
| Permission handling | Android platform API. |
| HTML→PDF via WebView **(exemption)** | WebView is Android-only, and print-to-PDF happens inside the WebView engine, not lopdf. Per ADR-004. |
| Office↔PDF via Collabora SDK **(exemption)** | Collabora/LibreOffice produces PDF bytes directly; there's no Rust step to own. Per ADR-002. |

### Bridge — shared concern:

| Concern | Home | Reason |
|---|---|---|
| Error type definitions | Rust (source of truth), Kotlin (mirror) | `uniffi` generates Kotlin types from Rust |
| File path convention | Defined in Rust, respected by both | Single source for temp/output path generation |
| Progress reporting | Rust sends progress via callback, Kotlin displays it | JNI callback from Rust to Kotlin |
| Cancelation | `AtomicBool` flag in Rust, set by Kotlin | Cancelable operations must respect this flag between pages |

## Decision Rationale

The "Rust owns the PDF" rule is the key insight. If Rust owns all PDF data:
- Kotlin never inspects PDF internals — cannot introduce PDF-specific bugs
- iOS port only needs to rebuild the UI layer, not the PDF logic
- Tests can be written at the Rust API boundary — language-agnostic
- No PDF parsing code duplication

## Consequences

### Positive
- Clean separation: Rust is the PDF engine, Kotlin is the app shell
- iOS transition requires zero Rust changes
- PDF logic is testable without Android emulator
- No Kotlin code can corrupt PDF state

### Negative
- Every new tool requires both Rust code + Kotlin ViewModel — more files
- File path conventions must be coordinated across language boundary
- Simple operations (e.g., "just rotate this page") require JNI round-trip even though they're conceptually simple
- Error mapping from Rust error codes to user-facing messages requires a translation layer

### Mitigations
- `uniffi` auto-generates Kotlin bindings — no manual JNI code
- Comprehensive error code enum shared between Rust and Kotlin
- Tool scaffolding template reduces boilerplate for new tools
- Integration tests verify the full Rust→Kotlin→UI chain
