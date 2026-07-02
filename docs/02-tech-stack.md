# Stirling Mobile — Tech Stack

> **Status:** Draft v1.0
> **Principle:** No runtime-heavy frameworks. Every dependency justifies its APK weight.

---

## 1. Language Stack

| Layer | Language | Justification |
|---|---|---|
| UI | Kotlin 2.x | Native Android, type-safe, coroutine-native |
| UI framework | Jetpack Compose | Declarative, Material 3, Google-supported |
| Business logic | Kotlin | Shared with UI, coroutine orchestration |
| PDF engine | Rust (stable) | Memory safety, zero-cost abstractions, cross-platform |
| JNI bridge | Kotlin `external fun` + Rust `jni` crate | Auto-generated via `uniffi` |

---

## 2. Android Dependencies

### 2.1 Core

| Dependency | Version | License | APK Impact | Purpose |
|---|---|---|---|---|
| Jetpack Compose BOM | 2025.x | Apache 2.0 | ~5MB | UI framework |
| Compose Navigation | 2.8+ | Apache 2.0 | ~500KB | Screen routing |
| Lifecycle ViewModel | 2.8+ | Apache 2.0 | ~200KB | State management |
| Coroutines | 1.9+ | Apache 2.0 | ~300KB | Async operations |
| Room | 2.7+ | Apache 2.0 | ~1MB | File history persistence |
| DataStore | 1.1+ | Apache 2.0 | ~200KB | User preferences |
| Coil | 3.x | Apache 2.0 | ~800KB | Image loading (thumbnails) |
| Material Icons Extended | - | Apache 2.0 | ~2MB | Tool icons |

### 2.2 PDF Rendering

| Dependency | Version | License | APK Impact | Purpose |
|---|---|---|---|---|
| pdfium-render (Rust) | 0.8+ | Apache 2.0 | In .so | GPU PDF page rendering |

No PDF.js. No WebView for PDF viewing (native Rust pdfium is faster, smaller).

### 2.3 Build System

| Tool | Version | Purpose |
|---|---|---|
| Gradle | 8.x | Android build |
| AGP | 8.x | Android Gradle Plugin |
| Kotlin | 2.x | Kotlin compiler |
| NDK | 27+ | Rust cross-compilation |
| cargo-ndk | latest | Rust → Android target bridge |
| Mozilla uniffi | 0.28+ | Rust ↔ Kotlin binding generation |

---

## 3. Rust Crate Dependencies

### 3.1 PDF Core

| Crate | Version | License | Purpose |
|---|---|---|---|
| lopdf | 0.34+ | MIT | PDF read/write/parse |
| pdfium-render | 0.8+ | Apache 2.0 | PDF page rendering |
| printpdf | 0.7+ | MIT | Create PDF from primitives |

### 3.2 Image & Graphics

| Crate | Version | License | Purpose |
|---|---|---|---|
| image | 0.25+ | MIT | Image decode/encode (PNG, JPEG, WebP, BMP, TIFF, GIF) |
| resvg | 0.43+ | MPL 2.0 | SVG rendering |
| tiny-skia | 0.11+ | BSD-3 | 2D graphics (drawing tools, watermarks) |
| rusttype | 0.9+ | MIT/Apache 2.0 | Font loading and glyph rasterization |

### 3.3 Document Processing

| Crate | Version | License | Purpose |
|---|---|---|---|
| pulldown-cmark | 0.12+ | MIT | Markdown to structured document |
| calamine | 0.25+ | MIT | Excel (.xlsx, .xls) read |
| csv | 1.3+ | Unlicense/MIT | CSV read/write |
| quick-xml | 0.37+ | MIT | XML read/write (XMP, DOCX, PPTX) |

### 3.4 Security

| Crate | Version | License | Purpose |
|---|---|---|---|
| ring | 0.17+ | ISC-style | Cryptographic operations |
| rcgen | 0.13+ | MIT/Apache 2.0 | X.509 certificate generation |
| x509-parser | 0.16+ | MIT/Apache 2.0 | X.509 certificate parsing |
| aes | 0.8+ | MIT/Apache 2.0 | AES-128/256 encryption |
| sha2 | 0.10+ | MIT/Apache 2.0 | SHA hashing |
| hmac | 0.12+ | MIT/Apache 2.0 | HMAC operations |

### 3.5 Compression & Optimization

| Crate | Version | License | Purpose |
|---|---|---|---|
| flate2 | 1.0+ | MIT/Apache 2.0 | DEFLATE/Zlib compression |
| jpeg-decoder | 0.3+ | MIT/Apache 2.0 | JPEG recompression |
| zopfli | 0.8+ | Apache 2.0 | Maximum DEFLATE compression |

### 3.6 Infrastructure

| Crate | Version | License | Purpose |
|---|---|---|---|
| jni | 0.21+ | MIT/Apache 2.0 | JNI bindings |
| uniffi | 0.28+ | MPL 2.0 | Language binding generation |
| serde | 1.0+ | MIT/Apache 2.0 | Serialization |
| serde_json | 1.0+ | MIT/Apache 2.0 | JSON (metadata, form data) |
| thiserror | 2.0+ | MIT/Apache 2.0 | Error types |
| log | 0.4+ | MIT/Apache 2.0 | Logging |
| rayon | 1.10+ | MIT/Apache 2.0 | **Deferred to v1.1+** — v1 engine is single-threaded (see R9, 14-risk-register.md); not linked in v1 builds |

---

## 4. Native SDK Dependencies

### 4.1 Tesseract OCR

| Component | Version | License | APK Impact |
|---|---|---|---|
| libtesseract.so | 5.x | Apache 2.0 | ~3MB |
| leptonica | 1.84+ | BSD-2 | In libtesseract.so |
| eng.traineddata | 4.x | Apache 2.0 | ~15MB (one language) |
| Other language packs | - | Apache 2.0 | Download on demand, ~15MB each |

Pre-compiled for Android NDK (aarch64 + armv7). Single language (English) bundled in APK. Additional languages fetched from tessdata GitHub on first use.

### 4.2 Collabora Office SDK — v1.1, not in v1

| Component | Version | License | APK Impact |
|---|---|---|---|
| collabora-office-sdk.aar | Latest stable | MPL 2.0 | ~40MB (arm64 only) |

Provides LibreOffice core compiled for Android. Used for Office↔PDF conversions. **Cut from v1 entirely** (solo-dev scope cut, see 11-roadmap.md and P2/R2 in 14-risk-register.md) — not just deferred to a separate build variant, it isn't built or linked at all until v1.1. When it lands in v1.1, its arm64-only constraint reintroduces the full/lite variant split below.

---

## 5. Build Variants

v1 ships one variant — there's no Collabora to split against, so "lite vs full" doesn't apply yet:

| Variant | Includes | APK Size | Purpose |
|---|---|---|---|
| `release` | Rust engine + Tesseract (English) | ~36MB | Play Store release, v1 |
| `debug` | Same as release + debug symbols | ~80MB | Development |
| `benchmark` | Release + instrumentation | ~45MB | Performance testing, not shipped |

v1.1, once Collabora is added back, reintroduces a `fullRelease`/`liteRelease` split (full ≈ 77MB with Collabora, lite ≈ 36MB without, matching today's numbers in [07-performance-budget.md](./07-performance-budget.md)) — that doc's §Storage stays the source of truth for whichever set of variants is current; update there first.

---

## 6. Target Architecture

| ABI | Supported | Notes |
|---|---|---|
| arm64-v8a | Yes (primary) | All modern Android devices |
| armeabi-v7a | Yes | Legacy 32-bit devices |
| x86_64 | Yes | Emulator + Chromebook ARC |
| x86 | No | Deprecated, <0.1% of devices |

v1 has no Collabora, so all three supported ABIs get the same single build — no variant split needed until v1.1 reintroduces Collabora's arm64-only constraint.

---

## 7. License Compatibility

All dependencies use permissive licenses (MIT, Apache 2.0, BSD, MPL 2.0, ISC). No GPL, LGPL, or AGPL. This allows:

- Proprietary Android app on Play Store
- No copyleft obligations
- MPL 2.0 (Collabora, resvg, uniffi) requires only modified MPL-licensed source to be disclosed — our code is not derivative

---

## 8. What We Deliberately Avoid

| Not using | Because |
|---|---|
| PDF.js | Large JS engine, WebView dependency, slower than pdfium |
| Apache PDFBox on Android | ~4MB, Java-only, no iOS path |
| Flutter | New language (Dart), no benefit over native Compose |
| React Native | JS bridge overhead, PDF engine runs native anyway |
| Firebase / Crashlytics | No network permission requirement |
| Retrofit / OkHttp | No network permission requirement |
| Hilt / Dagger | Manual DI is simpler for a single-DEX app |
| ProGuard / R8 config for Rust | Rust .so files don't need obfuscation |

---

## 9. Toolchain Setup

```
# One-time setup
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
cargo install uniffi-bindgen

# Build Rust engine for all targets
cd engine && cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o ../app/src/main/jniLibs build --release

# Generate Kotlin bindings
uniffi-bindgen generate engine/src/stirling_engine.udl --language kotlin --out-dir app/src/main/java
```
