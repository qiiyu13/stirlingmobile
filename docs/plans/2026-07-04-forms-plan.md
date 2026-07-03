# Forms Implementation Plan

> **Goal:** Implement Phase 4 forms tools: fill, flatten, extract (F-080, F-081, F-082)

**Architecture:** Rust `engine/src/forms.rs` with 3 UniFFI functions + 3 Kotlin ViewModels + 3 Compose Screens + 3 Tool enum entries

**Tech Stack:** lopdf (Rust PDF lib), UniFFI, Jetpack Compose, SAF pickers

---

### Task 1: Rust forms module — extract fields

**Files:** Create `engine/src/forms.rs`, modify `engine/src/lib.rs`

Implement `forms_get_fields(path) -> Vec<FormField>`. Walk /AcroForm/Fields tree recursively, handle /Kids hierarchy, resolve page number from widget /P reference. Define `FormField` struct with UniFFI traits.

### Task 2: Rust forms module — fill fields

**File:** Modify `engine/src/forms.rs`

Implement `forms_fill(path, values, output_path)`. Set /V on matched fields, set NeedAppearances flag. Define `FieldFill` struct.

### Task 3: Rust forms module — flatten

**File:** Modify `engine/src/forms.rs`

Implement `forms_flatten(path, output_path)`. For each widget with /AP/N stream, copy operators into page content prefix, remove from /Annots and /Fields.

### Task 4: Extend EngineError + register in lib.rs

**Files:** Modify `engine/src/lib.rs`, `engine/src/merge.rs`

Add form variants (NoAcroForm, FieldNotFound, NoAppearanceStream) to existing EngineError. Add `mod forms;` and `pub use forms::*;` to lib.rs.

### Task 5: Build + verify Kotlin bindings

Run `./gradlew :engine:build` to generate updated stirling_engine.kt.

### Task 6: Kotlin ViewModels

**Files:** Create `FormsFillViewModel.kt`, `FormsFlattenViewModel.kt`, `FormsExtractViewModel.kt`

### Task 7: Kotlin Screens

**Files:** Create `FormsFillScreen.kt`, `FormsFlattenScreen.kt`, `FormsExtractScreen.kt`

### Task 8: Navigation registration

**File:** Modify `MainActivity.kt`

Add Tool.FORMS_FILL, FORMS_FLATTEN, FORMS_EXTRACT to enum, when branches, HomeScreen buttons.
