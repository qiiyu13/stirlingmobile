# ADR-004: Android WebView for HTML→PDF

**Status:** Accepted
**Date:** 2026-07-02
**Deciders:** Stirling Mobile team

## Context

Stirling server uses WeasyPrint (Python) to convert HTML/CSS to PDF. WeasyPrint uses its own CSS layout engine built on Pango/Cairo. On Android, Python runtime is impractical. We need an on-device, 100% offline HTML→PDF converter.

## Decision

Use **Android System WebView's `PrintDocumentAdapter` / `print()` API** for HTML→PDF conversion.

## Options Considered

### Option A: Android WebView (Chrome engine)
- **Pros**: Already on every Android device. CSS support far exceeds WeasyPrint (flexbox, grid, modern CSS). Zero APK size impact (part of Android system). Single Kotlin API call. Offline (no network needed for static HTML). Hardware-accelerated rendering.
- **Cons**: Requires a Looper thread (not main thread). Output may differ from WeasyPrint (different rendering engines). Print-specific CSS (@page, page-break-*) support varies by WebView version.

### Option B: Compile WeasyPrint for Android
- **Pros**: Output parity with Stirling server.
- **Cons**: Requires Python runtime on Android (~20MB). Requires Pango, Cairo, GDK-Pixbuf compiled for Android. Massive engineering effort for minimal benefit. Python on Android is unsupported and fragile.

### Option C: Headless Chromium via custom WebView
- **Pros**: Full Chrome rendering, same as desktop. More control over print output.
- **Cons**: 50MB+ binary. Overkill.

### Option D: Build Rust HTML→PDF renderer
- **Pros**: Full control, smaller binary, portable.
- **Cons**: Building a CSS layout engine is a multi-year project. Even WeasyPrint (10+ years of dev) doesn't support all of CSS.

## Decision Rationale

WebView is the pragmatic choice:
1. Every Android device has it — zero APK cost
2. Chrome's Blink engine is the most CSS-capable layout engine in existence
3. The HTML→PDF use case is typically simple documents (invoices, reports, letters), not complex web apps
4. Print output quality from WebView is excellent (Chrome's own PDF export uses the same path)

## Consequences

### Positive
- Zero APK size impact
- Chrome-grade CSS support (better than WeasyPrint)
- Hardware-accelerated (faster than WeasyPrint)
- No external dependencies
- Simple API: `WebViewCompat.printToPdf()`

### Negative
- Output not byte-identical to Stirling server (different rendering engine)
- Print-specific CSS features (@page margins, page-break-before) have inconsistent support
- Android WebView version varies by device — older devices may have older Chrome engine
- Requires careful threading (WebView must run on a thread with Looper)

### Mitigations
- Run WebView on dedicated thread with its own Looper, destroy after conversion
- Test HTML→PDF output against WeasyPrint's test suite and document differences
- For critical formatting, recommend users use PDF-native tools (watermark, page numbers) instead of depending on CSS print features
- Bundle a fallback: for very old WebView versions (API < 29), use a simplified Rust-based markdown→PDF path with printpdf
