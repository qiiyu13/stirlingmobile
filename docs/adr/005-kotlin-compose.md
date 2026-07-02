# ADR-005: Kotlin + Jetpack Compose for UI

**Status:** Accepted
**Date:** 2026-07-02
**Deciders:** Stirling Mobile team

## Context

We need a UI framework for the Android app. The app has complex interactions (PDF viewer with gestures, tool parameter forms, drag-and-drop reorder, thumbnail strips). We evaluated native and cross-platform options.

## Decision

Use **Kotlin + Jetpack Compose** for the UI layer.

## Options Considered

### Option A: Kotlin + Jetpack Compose (Native Android)
- **Pros**: Google's recommended UI toolkit. Declarative (less code). Material 3 + dynamic colors. First-class Android integration (lifecycle, navigation, ViewModel). Excellent tooling (Android Studio previews). Growing ecosystem. Full access to Android APIs (SAF, WebView, sharing).
- **Cons**: Android-only (iOS requires rewrite). Relatively new (some APIs still experimental). Can be verbose for complex layouts.

### Option B: React Native
- **Pros**: Cross-platform (Android + iOS). Large ecosystem. Hot reload. Stirling's web frontend is React — could share some patterns.
- **Cons**: JS bridge overhead for our native-heavy app (PDF engine is Rust, pdfium is native, Collabora SDK is native, Tesseract is native — everything crosses the bridge). Slower rendering for complex UIs like PDF viewer. Native module boilerplate for every engine function. Larger APK (RN runtime).

### Option C: Flutter
- **Pros**: Cross-platform. Fast rendering (Skia). Good tooling.
- **Cons**: New language (Dart). PDF engine must FFI out anyway. Collabora SDK has no Flutter binding. iOS path via Flutter means committing to Dart ecosystem long-term. APK includes Flutter runtime (~5MB).

### Option D: Compose Multiplatform
- **Pros**: Share UI code between Android and iOS. Kotlin on both platforms.
- **Cons**: iOS support is still beta. Many Android APIs not available. Our iOS plan is far enough out that we can evaluate this later.

## Decision Rationale

Jetpack Compose wins because:
1. **Native first** — We're launching Android-only. Cross-platform adds complexity we don't need yet.
2. **Performance** — Zero bridge overhead to our Rust engine (JNI is already the bridge, we don't need a second one through JS/Dart).
3. **Ecosystem** — Every Android library works natively (SAF, WebView, sharing, notifications, file management).
4. **iOS path** — When we build iOS, the Rust engine is already ready. We rewrite only the UI in SwiftUI. Compose Multiplatform might be mature by then.

## Consequences

### Positive
- Best Android performance (no JS/Dart overhead)
- Full access to all Android platform APIs
- Material 3 design with dynamic colors on Android 12+
- Compose tooling in Android Studio
- Declarative UI reduces state bugs

### Negative
- iOS requires full UI rewrite (SwiftUI)
- Smaller hiring pool vs React Native
- Compose still evolving (some APIs may change)
- Learning curve for devs coming from XML layouts

### Mitigations
- iOS UI rewrite is already factored into roadmap (v2.0, Month 6)
- Re-evaluate Compose Multiplatform for iOS when v2.0 development starts
- Use established Compose patterns (ViewModel + StateFlow + Navigation)
- Avoid experimental Compose APIs in production code
