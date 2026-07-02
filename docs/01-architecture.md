# Stirling Mobile — Architecture

> **Status:** Draft v1.0
> **Principle:** Android-first, Rust core, Kotlin shell, iOS-capable engine.

---

## 1. System Overview

```
┌──────────────────────────────────────────────────────────────┐
│                    ANDROID APP PROCESS                        │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Kotlin / Jetpack Compose UI                 │ │
│  │  ┌─────────┐  ┌──────────┐  ┌────────────┐             │ │
│  │  │ Home    │  │ Viewer   │  │ Tool Page  │  ...        │ │
│  │  │ Screen  │  │ Screen   │  │ (per tool) │             │ │
│  │  └────┬────┘  └────┬─────┘  └─────┬──────┘             │ │
│  │       │            │              │                     │ │
│  │  ┌────┴────────────┴──────────────┴──────┐              │ │
│  │  │         ViewModel Layer                │              │ │
│  │  │   (per-screen state + coroutine scope) │              │ │
│  │  └───────────────────┬───────────────────┘              │ │
│  │                      │                                  │ │
│  │  ┌───────────────────┴──────────────────┐               │ │
│  │  │       Domain / Repository Layer      │               │ │
│  │  │  FileRepository  │  ToolRepository   │               │ │
│  │  │  HistoryRepository│  PreferencesRepo  │               │ │
│  │  └───────┬───────────┴──────┬───────────┘               │ │
│  └──────────┼──────────────────┼───────────────────────────┘ │
│             │                  │                              │
│  ┌──────────┴──────────────────┴──────────────────────────┐ │
│  │                   JNI Bridge Layer                      │ │
│  │  PdfEngine.kt  ← JNI →  libstirling_engine.so (Rust)   │ │
│  └────────────────────────┬───────────────────────────────┘ │
│                           │                                  │
│  ┌────────────────────────┴───────────────────────────────┐ │
│  │                 Rust PDF Engine                          │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌────────────────┐  │ │
│  │  │ Core PDF     │ │ Content Ops  │ │ Optimization   │  │ │
│  │  │ (lopdf)      │ │ (watermark,  │ │ (qpdf rewrite) │  │ │
│  │  │              │ │  annotate,   │ │                │  │ │
│  │  │              │ │  redact)     │ │                │  │ │
│  │  └──────────────┘ └──────────────┘ └────────────────┘  │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌────────────────┐  │ │
│  │  │ Page Ops     │ │ Security     │ │ Conversion     │  │ │
│  │  │ (merge,      │ │ (encrypt,    │ │ (image, md,    │  │ │
│  │  │  split,      │ │  sign,       │ │  text, csv)    │  │ │
│  │  │  rotate...)  │ │  certify)    │ │                │  │ │
│  │  └──────────────┘ └──────────────┘ └────────────────┘  │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │               Native Support Libraries                   │ │
│  │  ┌──────────────────┐  ┌─────────────────────────────┐  │ │
│  │  │ Tesseract NDK    │  │ Collabora Office SDK        │  │ │
│  │  │ (OCR engine)     │  │ (Office↔PDF conversions)    │  │ │
│  │  └──────────────────┘  └─────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

## 2. Process Architecture

### 2.1 Threading Model

```
Main Thread (UI)
  └── Compose rendering, touch events, animations

Default Dispatcher (CPU-bound)
  └── PDF parsing, page operations under 10MB

IO Dispatcher (I/O-bound)
  └── File read/write, SAF operations, image decode

Rust Engine Thread (internal)
  └── Heavy PDF operations, stream compression, OCR orchestration
      └── v1: single-threaded, one operation at a time (see R9, 14-risk-register.md)
      └── v1.1+: parallelize with rayon only if benchmarks justify it
```

**Rule**: No PDF operation blocks the main thread longer than 16ms. All engine calls are `suspend` functions backed by `Dispatchers.Default`, queued into the Rust engine one at a time.

### 2.2 Process Isolation

```
┌──────────────────────────────┐
│  Main Process                │
│  - UI                        │
│  - Light operations (<50MB)  │
│  - File management           │
└──────────────┬───────────────┘
               │ AIDL / Bound Service
┌──────────────┴───────────────┐
│  Engine Process (:engine)    │
│  - Heavy PDF operations      │
│  - Collabora SDK             │
│  - Tesseract OCR             │
│  - Memory ceiling: 512MB     │
└──────────────────────────────┘
```

Large operations run in a separate `:engine` process via Android bound service. If OOM killer strikes the engine process, the UI survives and can restart the operation. The engine process is configured with `android:process=":engine"` and `android:isolatedProcess="false"` (needs file I/O access).

---

## 3. Module Boundaries

### 3.1 What lives in Rust (`libstirling_engine.so`)

- PDF parsing and writing (lopdf)
- Page manipulation (merge, split, rotate, reorder, etc.)
- Content operations (watermark, redact, annotate)
- Security (encrypt, sign, certify)
- PDF-to-image conversion (pdfium + image-rs)
- Image-to-PDF, markdown-to-PDF
- Compression/optimization (qpdf rewrite)
- PDF/A validation
- Form fill/flatten
- Metadata read/write
- CSV extraction from tables

### 3.2 What lives in Kotlin

- Jetpack Compose UI (all screens)
- Navigation (Compose Navigation)
- ViewModel / state management
- FileRepository (SAF, internal storage, temp lifecycle)
- HistoryRepository (Room/SQLite)
- PreferencesRepository (DataStore)
- Coroutine orchestration
- Android lifecycle management
- Share sheet integration
- Notification/progress display

### 3.3 What calls Native Android APIs

- **HTML→PDF**: Android WebView (`WebView.printToPdf()`)
- **OCR**: Tesseract NDK (called from Rust via C FFI) + ML Kit fallback
- **Office↔PDF**: Collabora Office SDK (called from Kotlin via JNI)
- **PDF rendering**: pdfium (called from Rust, already C-compatible)
- **File picker/saver**: SAF (Storage Access Framework)

---

## 4. Data Flow

### 4.1 Typical Tool Operation

```
User taps "Merge"
  → MergeScreen composable
    → MergeViewModel.selectFiles() via SAF
    → User selects files, ViewModel holds URIs
    → User taps "Process"
    → MergeViewModel.merge() {
        viewModelScope.launch(Dispatchers.IO) {
          val pdfBytes = fileRepository.readAll(uris)
          val result = PdfEngine.merge(pdfBytes)  // JNI → Rust
          fileRepository.save(result, "merged.pdf")
          updateState { Success(outputUri) }
        }
      }
    → Compose recomposes with result preview
    → User taps "Share" → Android share sheet
```

### 4.2 Large File Handling (>100MB)

```
File > 100MB
  → Never loaded entirely into memory
  → Streamed through memory-mapped file (MappedByteBuffer in Kotlin,
    mmap in Rust)
  → Engine reads/writes in chunks via file path, not byte array
  → JNI passes file path strings, not byte buffers
```

### 4.3 Multi-Tool Pipeline

```
Tool chain: Split → Rotate → Compress

1. User imports PDF → stored in app-private dir
2. Split tool reads input → writes split parts to temp/
3. User selects one part for Rotate
4. Rotate reads from temp/part1.pdf → writes to temp/part1_rotated.pdf
5. Compress reads temp/part1_rotated.pdf → writes to output/
6. User exports final from output/

FileContext tracks the chain:
  input.pdf → [split] → part1.pdf, part2.pdf
  part1.pdf → [rotate] → part1_rotated.pdf
  part1_rotated.pdf → [compress] → final.pdf

User can jump back to any step, undo, or branch.
```

---

## 5. Storage Layout

```
/data/data/com.stirlingmobile/
├── files/
│   ├── imported/          # User-imported originals (read-only)
│   │   └── {uuid}.pdf
│   ├── working/           # Current session working files
│   │   └── {session_id}/
│   │       ├── input.pdf
│   │       ├── step_01_split_part1.pdf
│   │       ├── step_02_rotate.pdf
│   │       └── output.pdf
│   ├── temp/              # Ephemeral, wiped on app close
│   └── ocr/
│       └── tessdata/      # Downloaded OCR language packs
├── databases/
│   ├── history.db         # Room: recent files, tool history
│   └── preferences.db     # DataStore: settings
└── cache/
    └── thumbnails/        # Page thumbnail cache
```

---

## 6. Dependency Graph

```
app (Kotlin/Compose)
 ├── engine-bridge (Kotlin JNI wrapper)
 │    └── libstirling_engine.so (Rust NDK)
 │         ├── lopdf
 │         ├── pdfium-render
 │         ├── printpdf
 │         ├── image-rs
 │         ├── pulldown-cmark
 │         ├── csv
 │         ├── ring + rcgen + x509-parser
 │         ├── resvg
 │         └── calamine
 ├── libtesseract.so (NDK)
 ├── collabora-office-sdk.aar
 └── AndroidX (Compose, Navigation, Lifecycle, Room, DataStore)
```

---

## 7. iOS Transition Path

The Rust engine (`libstirling_engine`) compiles for `aarch64-apple-ios` and `aarch64-apple-ios-sim` as a static library. The same JNI surface maps to Swift via C FFI. Kotlin-specific components (UI, storage, lifecycle) must be rewritten in SwiftUI. The engine itself requires zero changes — same Rust code, same API, different target triple.

Estimated effort for iOS: 70% of Android effort (engine is reused, UI is rewrite).
