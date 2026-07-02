# Stirling Mobile — Testing Strategy

> **Status:** Draft v1.0

---

## 1. Testing Pyramid

```
         ╱  E2E  ╲
        ╱ (Compose)╲          5% — User workflows
       ╱────────────╲
      ╱ Integration  ╲       15% — JNI bridge, file I/O
     ╱────────────────╲
    ╱    Unit Tests     ╲    80% — Rust engine, ViewModels
   ╱────────────────────╲
```

---

## 2. Rust Engine Tests (Unit)

**Framework:** `cargo test` (built-in)
**Location:** `engine/src/` — `#[cfg(test)]` modules next to source
**Coverage target:** >80% line coverage

### Test categories

| Category | What's tested | Example |
|---|---|---|
| PDF parsing | lopdf reads valid/corrupt PDFs | `test_parse_valid_pdf()`, `test_reject_corrupt_pdf()` |
| Page operations | Every operation input→output | `test_merge_two_docs()`, `test_split_by_range()` |
| Content operations | Watermark, page numbers, redact | `test_redact_removes_content()`, `test_watermark_opacity()` |
| Round-trip | Parse→modify→write→re-parse | `test_split_roundtrip()` — split then re-merge = original |
| Fidelity | Output matches reference PDFBox output | `test_merge_output_matches_pdfbox()` |
| Edge cases | Empty PDF, 1 page, 1000 pages, Unicode | `test_merge_single_page()`, `test_unicode_metadata()` |
| Error handling | Wrong password, corrupt input, OOM | `test_wrong_password_returns_error()` |
| Security | Encrypt→decrypt, sign→verify | `test_aes256_roundtrip()`, `test_sign_verify()` |

### Test fixtures

```
engine/test_fixtures/
├── blank_1page.pdf
├── blank_100pages.pdf
├── text_only.pdf
├── image_heavy.pdf
├── password_protected.pdf      (password: "test")
├── corrupt_header.pdf
├── unicode_metadata.pdf
├── form_fillable.pdf
├── scanned_no_ocr.pdf
├── pdfa_2b_valid.pdf
└── office/
    ├── simple.docx
    ├── complex_tables.xlsx
    └── presentation.pptx
```

---

## 3. Rust Engine Tests (Property-Based)

**Framework:** `proptest`
**Coverage:** Merge, split, rotate, reorder

```rust
proptest! {
    #[test]
    fn merge_is_associative(
        docs in prop::collection::vec(valid_pdf_strategy(), 2..10)
    ) {
        let merged = PdfEngine::merge(&docs).unwrap();
        assert!(merged.page_count == docs.iter().map(|d| d.page_count).sum());
    }

    #[test]
    fn split_then_merge_is_identity(
        doc in valid_pdf_strategy(),
        split_at in 1u32..doc.page_count
    ) {
        let parts = PdfEngine::split(&doc, &format!("1-{}", split_at)).unwrap();
        let merged = PdfEngine::merge(&parts).unwrap();
        // Content should be identical (page count matches)
        assert_eq!(merged.page_count, doc.page_count);
    }
}
```

---

## 4. Kotlin Unit Tests

**Framework:** JUnit 5 + Turbine (Flow testing) + MockK
**Location:** `app/src/test/`

### ViewModel Tests

```kotlin
@Test
fun `mergeViewModel emits Success state on engine merge success`() = runTest {
    val engine = mockk<PdfEngine>()
    val fileRepo = mockk<FileRepository>()
    val viewModel = MergeViewModel(engine, fileRepo)

    coEvery { engine.merge(any(), any()) } returns EngineResult.OkPath("/tmp/merged.pdf")
    every { fileRepo.readAll(any()) } returns listOf("pdf1".toByteArray())

    viewModel.merge(listOf("uri1", "uri2"))

    viewModel.uiState.test {
        assertEquals(Step.Processing, awaitItem().step)
        assertEquals(Step.Result, awaitItem().step)
        cancelAndIgnoreRemainingEvents()
    }
}

@Test
fun `mergeViewModel emits Error state on engine failure`() = runTest {
    val engine = mockk<PdfEngine>()
    coEvery { engine.merge(any(), any()) } returns
        EngineResult.Err(ErrorCode.CorruptedPdf, "Bad PDF")

    val viewModel = MergeViewModel(engine, mockk())
    viewModel.merge(listOf("bad_pdf"))

    viewModel.uiState.test {
        assertNotNull(awaitItem().error)
        cancelAndIgnoreRemainingEvents()
    }
}
```

### Repository Tests

```kotlin
@Test
fun `fileRepository prunes imports older than 7 days`() = runTest {
    val repo = FileRepository(testContext)
    val oldFile = FileInfo(/* 8 days ago */)
    val newFile = FileInfo(/* 1 day ago */)

    repo.addImport(oldFile)
    repo.addImport(newFile)
    repo.pruneOldImports()

    assertNull(repo.getImport(oldFile.id))
    assertNotNull(repo.getImport(newFile.id))
}
```

---

## 5. Integration Tests

**Framework:** JUnit + AndroidX Test + `cargo test` via shell
**Location:** `app/src/androidTest/`

### JNI Bridge Tests

```kotlin
@RunWith(AndroidJUnit4::class)
class PdfEngineIntegrationTest {
    @Test
    fun engineInit_loadsNativeLibrary() {
        val result = PdfEngine.init(context.filesDir.absolutePath + "/temp")
        assertTrue(result is EngineResult.Ok)
    }

    @Test
    fun merge_producesValidPdf() {
        val testPdf1 = loadTestFixture("blank_1page.pdf")
        val testPdf2 = loadTestFixture("text_only.pdf")

        val result = PdfEngine.merge(arrayOf(testPdf1, testPdf2))

        assertTrue(result is EngineResult.OkPath)
        val output = File((result as EngineResult.OkPath).path)
        assertTrue(output.exists())
        assertTrue(output.length() > 0)

        // Verify it's a valid PDF
        val info = PdfEngine.getInfo(output.path, "")
        assertTrue(info is EngineResult.OkInfo)
    }

    @Test
    fun merge_withLargeFiles_usesStreaming() {
        val largePdfs = (1..5).map { generateLargePdf(20 * 1024 * 1024) }
        val result = PdfEngine.merge(largePdfs.toTypedArray())
        assertTrue(result is EngineResult.OkPath)
    }

    @Test
    fun allTools_handleCorruptedInput() {
        val corrupt = "not a pdf".toByteArray()
        // Test each tool rejects corrupt input gracefully
        for (tool in listOf("merge", "split", "rotate", "compress")) {
            val result = when (tool) {
                "merge" -> PdfEngine.merge(arrayOf(corrupt))
                // ... etc
            }
            assertTrue(result is EngineResult.Err)
        }
    }
}
```

### PDF Fidelity Tests

Compare output against reference files generated by Stirling PDF server:

```bash
# Reference generation (one-time, on server)
for tool in merge split rotate compress; do
    curl -X POST http://localhost:8080/api/v1/$tool \
        -F "fileInput=@test_fixtures/input.pdf" \
        -o "reference/$tool/output.pdf"
done

# Mobile comparison (CI)
for tool in merge split rotate compress; do
    # Run mobile engine on same input
    adb shell am instrument -w -e tool "$tool" \
        com.stirlingmobile.test/com.stirlingmobile.test.FidelityTest

    # Pull mobile output, compare byte-for-byte or visually
    adb pull /data/data/com.stirlingmobile/files/output.pdf mobile_output.pdf
    compare_pdfs reference/$tool/output.pdf mobile_output.pdf
done
```

---

## 6. E2E / Compose UI Tests

**Framework:** Compose Testing + `androidx.test`
**Location:** `app/src/androidTest/`

```kotlin
@Test
fun userCanMergeTwoPdfs() {
    composeTestRule.setContent { StirlingMobileApp() }

    // Navigate to Merge tool
    composeTestRule.onNodeWithText("Merge PDFs").performClick()

    // Add files
    composeTestRule.onNodeWithText("Add files").performClick()
    // (File picker is a system dialog — mocked in test)

    // Tap Process
    composeTestRule.onNodeWithText("Process").performClick()

    // Wait for result
    composeTestRule.waitUntil(5000) {
        composeTestRule.onAllNodesWithText("merged.pdf").fetchSemanticsNodes().isNotEmpty()
    }

    // Verify share button appears
    composeTestRule.onNodeWithText("Share").assertIsDisplayed()
}
```

### Key E2E Flows

| Flow | Priority |
|---|---|
| Merge 2 PDFs → Share | P0 |
| View PDF → Rotate page → Save | P0 |
| Import PDF → Compress → Compare sizes | P0 |
| Split PDF → Extract specific pages | P0 |
| Import image → Convert to PDF | P1 |
| Full pipeline: Split → Rotate → Compress → Export | P1 |
| OCR scanned PDF | P2 |
| Office DOCX → PDF → View | P2 |
| Sign PDF with certificate | P2 |

---

## 7. CI Matrix

| Platform | Rust Tests | Kotlin Unit | Android Tests | E2E | Fidelity |
|---|---|---|---|---|---|
| ubuntu-latest (GitHub Actions) | ✅ | ✅ | — | — | — |
| macos-latest (GitHub Actions) | ✅ | — | — | — | — |
| Android Emulator API 30 | — | — | ✅ | ✅ | ✅ |
| Android Emulator API 34 | — | — | ✅ | — | — |
| Physical device (manual) | — | — | ✅ | ✅ | ✅ |

### CI Pipeline

```
PR opened
  ├── Rust: cargo clippy, cargo fmt --check, cargo test
  ├── Kotlin: ./gradlew lintDebug, ./gradlew testDebugUnitTest
  ├── Android: ./gradlew connectedCheck (emulator)
  ├── E2E: ./gradlew e2eTest (emulator)
  ├── Fidelity: compare-outputs.sh (emulator)
  ├── Benchmark: ./gradlew benchmark:connectedCheck
  └── Size: check APK < 40MB (v1, no Collabora; raises to < 80MB in v1.1 — see 07-performance-budget.md)

All must pass → PR can merge
```

---

## 8. Manual QA Checklist

Per release:
- [ ] Test on 3 physical devices (low-end, mid-range, flagship)
- [ ] Test with 10 different PDFs (text, image, form, password, corrupt)
- [ ] Test all P0 tools with >100MB PDF
- [ ] Test airplane mode (no network calls)
- [ ] Test accessibility (TalkBack on all major screens)
- [ ] Test RTL language (Arabic)
- [ ] Test dark mode
- [ ] Test device rotation during processing
- [ ] Test process death recovery (developer options → don't keep activities)

---

## 9. Bug Bounty / Community Testing

- Internal test track on Play Store (closed alpha, 100 testers)
- Open testing track (open beta, unlimited) before production release
- Crash reporting via Play Console (no SDK needed — Play collects ANRs & crashes natively)
- GitHub Issues for bug reports from beta testers
