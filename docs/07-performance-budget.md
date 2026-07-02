# Stirling Mobile — Performance Budget

> **Status:** Draft v1.0
> **Test device baseline:** Snapdragon 730G, 4GB RAM, Android 11
> **Test files:** Standard PDF-A (text), PDF-B (images), PDF-C (mixed, 500 pages)

---

## 1. Operation Performance Targets

| Operation | Input | Target | Bucket |
|---|---|---|---|
| First page render | 100-page PDF | < 500ms | Cold start |
| Subsequent page render | Any | < 100ms | Warm (cached) |
| Merge 2 PDFs | 2x 10MB, 50 pages each | < 3s | Fast |
| Merge 2 PDFs (NF-002) | 2x 50MB, ~250 pages each | < 5s | Medium |
| Merge 5 PDFs | 5x 10MB, 50 pages each | < 8s | Medium |
| Split pages | 50MB, 200 pages | < 2s | Fast |
| Rotate all pages | 50MB, 200 pages | < 3s | Fast |
| Compress (lossy) | 50MB image-heavy PDF | < 10s | Medium |
| Lossless optimize | 50MB | < 5s | Medium |
| OCR (English) | 10 pages, 300dpi | < 15s per page | Slow |
| PDF to images | 10 pages → PNG | < 2s per page | Fast |
| Images to PDF | 20x 5MP JPEGs | < 5s | Fast |
| Office→PDF (DOCX) | 5MB, 20 pages | < 15s | Medium |
| PDF→Office (DOCX) | 5MB, 20 pages | < 20s | Medium |
| HTML→PDF | 50KB HTML | < 3s | Fast |
| MD→PDF | 100KB Markdown | < 1s | Fast |
| Sign PDF | 10MB | < 2s | Fast |
| Redact PDF | 10MB, 50 redactions | < 3s | Medium |
| Add watermark | 10MB | < 2s | Fast |
| Fill form | 5MB, 50 fields | < 1s | Fast |

## 2. Memory Budget

| Scenario | Budget | Notes |
|---|---|---|
| Idle (home screen) | < 80MB | Kotlin VM + Compose UI |
| Viewer open (50MB PDF) | < 150MB | pdfium-render holds decoded page cache |
| Viewer open (500MB PDF) | < 300MB | Streamed, only 5 pages in memory |
| Tool processing (50MB PDF) | < 200MB | Rust engine + Kotlin VM |
| Tool processing (500MB PDF) | < 400MB | Memory-mapped file, chunks of 50MB |
| Engine process ceiling | 512MB | Enforced by `android:largeHeap="false"` |
| System memory pressure | GC + trim memory | `onTrimMemory()` frees caches |

## 3. Storage Budget

| Category | Budget | Notes |
|---|---|---|
| APK (v1, no Collabora) | < 40MB | Compressed download ~36MB |
| APK (v1.1, with Collabora) | < 80MB | Compressed download ~77MB |
| APK install size (v1) | < 80MB | Uncompressed .so files + resources |
| App data (working) | < 1GB | Working files, auto-pruned |
| App data (persistent) | < 200MB | History DB, preferences, cached thumbnails |
| OCR language pack (each) | < 20MB | Downloaded on demand, v1.2+ (v1 bundles English only) |
| Collabora SDK (on disk) | ~80MB | v1.1+ only — extracted from APK at install |

## 4. Battery Budget

| Scenario | Drain (% per minute) | 10-min impact |
|---|---|---|
| Viewing PDF (idle scroll) | < 0.3%/min | < 3% |
| Light operation (rotate, split) | < 0.5%/min | < 5% |
| Medium operation (merge, compress) | < 1.0%/min | < 10% |
| Heavy operation (OCR, Office→PDF) | < 2.0%/min | < 20% |

Target: 10-minute heavy operation session drains < 20% battery on a 4000mAh device.

## 5. APK Size Breakdown

| Component | Size (compressed) | Notes | Ships in |
|---|---|---|---|
| Kotlin DEX | ~3MB | Compose + AndroidX | v1 |
| Resources (icons, strings, etc.) | ~5MB | Material Icons, 3-5 languages in v1 (40+ by v1.2) | v1 |
| libstirling_engine.so (arm64) | ~8MB | Rust engine (stripped, LTO) | v1 |
| libstirling_engine.so (armv7) | ~6MB | Secondary ABI | v1 |
| libstirling_engine.so (x86_64) | ~9MB | Emulator ABI | v1 |
| libtesseract.so | ~3MB | Single ABI bundled | v1 |
| eng.traineddata | ~15MB | English only in v1, more languages downloadable v1.2+ | v1 |
| collabora-office-sdk.aar | ~28MB | Compressed in APK | v1.1 |
| **Total (v1, no Collabora)** | **~36MB** | matches the `release` variant in 02-tech-stack.md | v1 |
| **Total (v1.1, with Collabora)** | **~77MB** | matches the future `fullRelease` variant | v1.1 |

## 6. Large File Strategy

Files > 100MB use a streaming/memory-mapped approach:

```
File > 100MB detected
  → JNI passes file path, not byte buffer
  → Rust opens file via mmap (memmap2 crate)
  → Processes in chunks (pages are independent units)
  → Writes output incrementally via BufWriter
  → Progress reported per page/chunk processed
  → Cancelable between chunks (AtomicBool flag)
  → Kotlin shows progress bar with page count
```

Never attempt to load entire >100MB PDF into a `ByteArray`. This is the #1 source of OOM crashes in PDF apps.

## 7. Benchmarking Suite

```bash
# Automated performance tests
./gradlew :app:benchmark:connectedCheck

# Manual benchmarking
adb shell am start -n com.stirlingmobile/.benchmark.BenchmarkActivity \
  --es operation "merge" --es file_sizes "10,50,100,200" --ei iterations 10
```

Each operation benchmarked with:
- `androidx.benchmark` library
- 10 warm-up iterations + 50 measured iterations
- Monitored: wall time, CPU time, memory high-water mark, battery drain
- Regression threshold: >10% degradation = CI failure

## 8. Performance Regression Prevention

| Guard | Method |
|---|---|
| CI benchmark check | GitHub Actions runs `benchmark` variant, fails on regression |
| Memory leak detection | LeakCanary in debug builds, heap dumps on OOM |
| ANR monitoring | StrictMode in debug, 5s disk read / 2s disk write penalties |
| Frame timing | JankStats on viewer screen, report frames > 16ms |
| Startup time | `androidx.startup` report, target < 1s cold start |
