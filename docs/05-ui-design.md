# Stirling Mobile — UI Design

> **Status:** Draft v1.0
> **Framework:** Jetpack Compose + Material 3
> **Target:** Android phones (portrait primary, landscape supported)

---

## 1. Navigation Map

```
App Launch
  │
  ├── Home Screen
  │     ├── Recent Files (horizontal list)
  │     ├── Quick Tools (top 6, configurable)
  │     ├── All Tools (grid → categorized)
  │     │     ├── Page Operations (Merge, Split, Rotate, Reorder, ...)
  │     │     ├── Convert (PDF→Image, Image→PDF, Office↔PDF, ...)
  │     │     ├── Security (Password, Sign, Redact, Sanitize)
  │     │     ├── Edit & Annotate (Watermark, Page Numbers, ...)
  │     │     ├── OCR
  │     │     ├── Forms (Fill, Flatten)
  │     │     ├── Compress & Optimize
  │     │     ├── Metadata
  │     │     └── Advanced (Compare, Overlay, ...)
  │     ├── Settings
  │     └── About / Licenses
  │
  ├── Viewer Screen (per-file)
  │     ├── Page render (pdfium)
  │     ├── Thumbnail strip (bottom)
  │     ├── Page jump
  │     ├── Text search
  │     ├── Share / Export
  │     ├── Add to pipeline
  │     └── [Tool] button → opens tool with this file pre-loaded
  │
  └── Tool Screen (per-tool)
        ├── File input (pick from device / recent / viewer)
        ├── Tool-specific parameters
        ├── Process button
        ├── Progress indicator
        ├── Result preview (viewer embedded)
        └── Export / Share / Chain to next tool
```

## 2. Screen Layouts

### 2.1 Home Screen

```
┌──────────────────────────────┐
│  Stirling Mobile        ⚙️   │  ← TopAppBar
├──────────────────────────────┤
│  Recent Files                │
│  ┌────┐ ┌────┐ ┌────┐      │  ← LazyRow, thumbnails
│  │ 📄 │ │ 📄 │ │ 📄 │  ... │
│  └────┘ └────┘ └────┘      │
│                              │
│  Quick Tools                 │
│  ┌─────┐ ┌──────┐ ┌───────┐ │
│  │Merge│ │Compress│ │Sign  │ │
│  └─────┘ └──────┘ └───────┘ │
│  ┌─────┐ ┌──────┐ ┌───────┐ │
│  │Split│ │PDF→Img│ │OCR   │ │
│  └─────┘ └──────┘ └───────┘ │
│                              │
│  All Tools                   │
│  ┌────────────────────────┐  │
│  │ 📄 Page Operations  →  │  │  ← Expandable sections
│  ├────────────────────────┤  │
│  │ 🔄 Convert          →  │  │
│  ├────────────────────────┤  │
│  │ 🔒 Security         →  │  │
│  ├────────────────────────┤  │
│  │ ✏️ Edit & Annotate  →  │  │
│  ├────────────────────────┤  │
│  │ 👁️ OCR              →  │  │
│  ├────────────────────────┤  │
│  │ 📝 Forms            →  │  │
│  ├────────────────────────┤  │
│  │ 📦 Compress         →  │  │
│  ├────────────────────────┤  │
│  │ ℹ️ Metadata         →  │  │
│  ├────────────────────────┤  │
│  │ 🔬 Advanced         →  │  │
│  └────────────────────────┘  │
│                              │
│  ┌────────────────────────┐  │
│  │  📁 Open File          │  │  ← FAB / bottom CTA
│  └────────────────────────┘  │
└──────────────────────────────┘
```

### 2.2 Tool Screen (generic pattern)

```
┌──────────────────────────────┐
│  ← Merge PDFs           ℹ️   │  ← TopAppBar with info/tooltip
├──────────────────────────────┤
│                              │
│  Input Files                 │
│  ┌────────────────────────┐  │
│  │ 📄 document_a.pdf   ✕  │  │  ← File chips, removable
│  │ 📄 document_b.pdf   ✕  │  │
│  │ 📄 document_c.pdf   ✕  │  │
│  │ ＋ Add files            │  │
│  └────────────────────────┘  │
│                              │
│  Reorder output pages?       │
│  [Toggle switch]             │  ← Tool-specific parameters
│                              │
│  ┌────────────────────────┐  │
│  │     Process (3 files)  │  │  ← Primary CTA
│  └────────────────────────┘  │
│                              │
│  ↻ Processing... (67%)       │  ← Progress state
│  ═══════════════░░░░░░░      │
│                              │
│  Result                      │
│  ┌────────────────────────┐  │
│  │  [page 1 preview]      │  │
│  │  merged.pdf · 2.4 MB   │  │
│  │  [Share] [Open] [Chain]│  │
│  └────────────────────────┘  │
└──────────────────────────────┘
```

### 2.3 Viewer Screen

```
┌──────────────────────────────┐
│  document.pdf (3/42)    🔍 ⋮ │  ← TopAppBar with page counter
├──────────────────────────────┤
│                              │
│  ┌────────────────────────┐  │
│  │                        │  │
│  │    PDF Page Render     │  │  ← pdfium-render, pinch-to-zoom
│  │    (scroll vertically  │  │
│  │     for next pages)    │  │
│  │                        │  │
│  └────────────────────────┘  │
│                              │
│ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐  │  ← Thumbnail strip (horizontal)
│ │1 │ │2 │ │3*│ │4 │ │5 │  │     current page highlighted
│ └──┘ └──┘ └──┘ └──┘ └──┘  │
│                              │
│  [Share] [Tools ▾] [Export]  │  ← Bottom bar
└──────────────────────────────┘
```

---

## 3. Component Tree

```
App
├── AppTheme (Material3 dynamic color)
├── NavHost
│   ├── HomeScreen
│   │   ├── TopAppBar
│   │   ├── RecentFilesRow
│   │   │   └── FileThumbnailCard (per file)
│   │   ├── QuickToolsGrid
│   │   │   └── ToolChip (per tool)
│   │   ├── AllToolsList
│   │   │   └── ToolCategory (per category)
│   │   │       └── ToolListItem (per tool)
│   │   └── OpenFileButton
│   │
│   ├── ViewerScreen
│   │   ├── ViewerTopBar
│   │   ├── PdfPageViewer
│   │   │   └── PdfPage (lazy, rendered by pdfium)
│   │   ├── ThumbnailStrip
│   │   │   └── ThumbnailItem
│   │   └── ViewerBottomBar
│   │
│   ├── ToolScreen (generic, composed per tool)
│   │   ├── ToolTopBar
│   │   ├── FileInputSection
│   │   │   └── FileChip
│   │   ├── ToolParameterSection
│   │   │   └── (tool-specific composables)
│   │   ├── ProcessButton
│   │   ├── ProgressIndicator
│   │   └── ResultSection
│   │       ├── PdfPreviewEmbed
│   │       └── ResultActions
│   │
│   └── SettingsScreen
│       ├── ThemeSelector (light/dark/system)
│       ├── LanguageSelector
│       ├── OCR Language Downloads
│       ├── Default Export Format
│       ├── Clear Cache
│       └── AboutSection (version, licenses, credits)
│
├── BottomNavigationBar (Home, Viewer if active, Files)
└── SnackbarHost
```

---

## 4. Design Tokens

### 4.1 Colors (Material 3 dynamic, with fallback)

| Token | Light | Dark |
|---|---|---|
| Primary | `#1A73E8` (Google Blue) | `#8AB4F8` |
| Surface | `#FFFFFF` | `#1E1E1E` |
| SurfaceVariant | `#F1F3F4` | `#2D2D2D` |
| Error | `#D93025` | `#F28B82` |
| OnPrimary | `#FFFFFF` | `#003D74` |

Prefers `dynamicColor` on Android 12+ (Material You).

### 4.2 Typography

| Role | Size | Weight |
|---|---|---|
| Display (tool name) | 24sp | Medium |
| Headline | 20sp | Regular |
| Title | 16sp | Medium |
| Body | 14sp | Regular |
| Label (buttons) | 14sp | Medium |
| Caption (metadata) | 12sp | Regular |

### 4.3 Spacing

| Token | Value |
|---|---|
| xs | 4dp |
| sm | 8dp |
| md | 16dp |
| lg | 24dp |
| xl | 32dp |

### 4.4 Elevation

| Level | Usage |
|---|---|
| 0dp | Surface, cards on surface |
| 1dp | File chips |
| 2dp | TopAppBar |
| 3dp | FAB, bottom sheets |
| 6dp | Dialogs |

---

## 5. Interaction Patterns

### 5.1 File Selection
- **Single file**: SAF file picker (`ACTION_OPEN_DOCUMENT`, MIME `application/pdf`)
- **Multiple files**: SAF with `EXTRA_ALLOW_MULTIPLE`
- **Images**: MIME `image/*` (for image-to-PDF)
- **From viewer**: "Use this file" button passes current file to tool
- **Recent files**: Tap a recent file card → opens viewer first, then user picks tool

### 5.2 Processing Flow
1. User configures tool parameters
2. Taps "Process" button
3. Button enters loading state (circular progress)
4. Progress updates (percentage + estimated time)
5. On completion: preview shown, actions available
6. On error: snackbar with error message, retry available

### 5.3 Pipeline Flow
1. After any tool completes, "Chain Next Tool" button appears
2. Tapping shows tool picker filtered to compatible tools
3. Output of previous tool becomes input of next
4. Pipeline is tracked in FileContext, user can jump back
5. "Export Final" → shares/saves the last output

### 5.4 Gestures
- Pinch-to-zoom in viewer (0.5x - 5x)
- Double-tap to zoom to fit width
- Swipe left/right to change pages (viewer)
- Long-press file chip → reorder (in merge/reorder tools)
- Pull-to-refresh on recent files list

---

## 6. State Management

```kotlin
// Per-tool ViewModel pattern
class MergeViewModel(
    private val fileRepo: FileRepository,
    private val engine: PdfEngine
) : ViewModel() {

    // UI state sealed hierarchy
    data class UiState(
        val step: Step = Step.Input,
        val selectedFiles: List<FileInfo> = emptyList(),
        val processing: Boolean = false,
        val progress: Float = 0f,
        val result: MergeResult? = null,
        val error: String? = null
    )

    enum class Step { Input, Processing, Result, Error }

    data class MergeResult(
        val outputPath: String,
        val sizeBytes: Long,
        val pageCount: Int,
        val previewBytes: ByteArray?
    )
}
```

Every tool screen follows this exact pattern. Tool-specific ViewModels extend a base `ToolViewModel` that handles lifecycle, file I/O, and error mapping.

---

## 7. Internationalization Layout Concerns

- All strings use `stringResource()` with locale-specific `strings.xml` or Compose `Strings`
- RTL layout via `LayoutDirection` — Compose handles automatically
- Tool names, descriptions, error messages all extracted to resource files
- Number formatting uses device locale (page numbers, file sizes)
- Font selection: system default (Roboto for Latin, Noto for CJK/Arabic)

---

## 8. Accessibility

- All interactive elements have `contentDescription`
- Minimum touch target 48dp (enforced by lint rule)
- `clickable` with `role = Button` semantics
- Progress announcements via `AccessibilityManager`
- Error states announced via live region
- Color is never the sole differentiator (icons always accompany color)
