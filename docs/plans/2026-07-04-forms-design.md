# Form Tools Design (Phase 4, Weeks 17-18)

## Overview

Three form tools porting Stirling PDF's AcroForm handling to mobile:
- **F-080 Fill** — fill AcroForm text/checkbox/choice fields
- **F-081 Flatten** — merge widget annotations into page content
- **F-082 Extract** — export field data as JSON/CSV

## Rust API

```rust
// Shared data types
pub struct FormField {
    pub name: String,
    pub field_type: String,  // "Tx", "Btn", "Ch", "Sig"
    pub value: Option<String>,
    pub page: u32,
}

pub struct FieldFill {
    pub name: String,
    pub value: String,
}

// API functions
pub fn forms_get_fields(path: String) -> Result<Vec<FormField>, EngineError>;
pub fn forms_fill(path: String, values: Vec<FieldFill>, output_path: String) -> Result<(), EngineError>;
pub fn forms_flatten(path: String, output_path: String) -> Result<(), EngineError>;
```

## Fill strategy

Set `/V` on field dictionaries, set `NeedAppearances` flag in AcroForm so viewers regenerate appearance. Handles field hierarchy (parent fields with `/Kids`). Simple text fields only in v1; checkboxes and choice fields deferred.

## Flatten strategy

Walk page `/Annots` arrays. For each widget with existing `/AP/N` appearance stream, copy stream operators into page content stream (prefix), remove widget from `/Annots`, remove field from `/Fields`. Widgets without appearance streams skipped silently — most PDF generators include them, so this covers the common case.

## Extract strategy

`forms_get_fields` recurses through `/AcroForm/Fields` tree, flattening `/Kids` hierarchy, collecting `/T`, `/V`, `/FT`. Page number resolved via widget `/P` reference lookup. Kotlin side formats output as JSON or CSV.

## Kotlin UI

| Tool | Screen | Flow |
|---|---|---|
| Fill | `FormsFillScreen` | Pick PDF → `formsGetFields()` → show field list (name, type, value) → user edits → `formsFill()` → save |
| Flatten | `FormsFlattenScreen` | Pick PDF → `formsFlatten()` → save |
| Extract | `FormsExtractScreen` | Pick PDF → `formsGetFields()` → display table → export JSON/CSV |

## Error handling

Extend `EngineError` with: `NoAcroForm`, `FieldNotFound(String)`, `NoAppearanceStream`.

## Out of scope

- XFA forms (proprietary Adobe format, no open-source parser)
- Appearance stream generation for filled fields (rely on viewer `NeedAppearances`)
- Choice field validation (arbitrary strings accepted)
- F-083 Create new form fields (P3, deferred)
