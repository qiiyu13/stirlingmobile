# ADR-001: Rust PDF Engine

**Status:** Accepted
**Date:** 2026-07-02
**Deciders:** Stirling Mobile team

## Context

The PDF engine is the core of the app. It must run on-device, handle 50+ operations, support large files (>500MB), and compile for both Android (today) and iOS (future). We evaluated four options.

## Decision

Use **Rust** compiled via NDK as the PDF engine language.

## Options Considered

### Option A: Java/Kotlin (port PDFBox to Android)
- **Pros**: Same language as UI, no JNI, familiar to Android devs
- **Cons**: PDFBox is 4MB+, Java GC hurts large PDF performance, iOS incompatible

### Option B: C/C++ (port poppler + MuPDF)
- **Pros**: Battle-tested, fast, small footprint
- **Cons**: Memory unsafety, complex build system, MuPDF is AGPL, integrating multiple C libs creates ABI headache

### Option C: Rust (lopdf + custom ops)
- **Pros**: Memory safety, zero-cost abstractions, compiles to Android NDK and iOS FFI from same codebase, rich crate ecosystem (lopdf, printpdf, pdfium, image, ring, etc.)
- **Cons**: Team must learn Rust, JNI bridge overhead, fewer PDF-specific libraries than Java

### Option D: Flutter/Dart (custom engine)
- **Pros**: Cross-platform UI + logic in one language
- **Cons**: No PDF ecosystem in Dart, would need to FFI out anyway, adds a runtime

## Consequences

### Positive
- Same Rust code compiles for Android (arm64, armv7, x86_64) and iOS (arm64) with zero source changes
- Memory safety eliminates use-after-free and buffer overflow risks in PDF parsing
- `lopdf` crate provides solid PDF read/write foundation
- `cargo-ndk` automates cross-compilation
- `uniffi` auto-generates Kotlin bindings from Rust
- iOS-ready engine from day 1 — only UI needs rewriting for iOS

### Negative
- Team must learn Rust (steep but one-time cost)
- JNI calls add ~1μs overhead per call (negligible vs PDF processing time)
- Crate ecosystem less PDF-mature than Java's PDFBox — some gaps need custom code
- Build pipeline is more complex (Rust → NDK → .so → APK vs. just Java → APK)

### Mitigations
- Start with simpler PDF operations (merge, split) to build Rust confidence
- Use `uniffi` for JNI generation (prevents JNI bugs)
- File path passing for large files (avoid JNI byte array overhead)
- Thorough fuzz testing of PDF parser (cargo-fuzz) to compensate for less battle-tested parser
