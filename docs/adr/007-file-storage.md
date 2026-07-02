# ADR-007: App-Private File Storage vs SAF-Write

**Status:** Accepted
**Date:** 2026-07-02
**Deciders:** Stirling Mobile team

## Context

PDF files can be large (100MB+) and contain sensitive data. We must decide where files live during processing, where temporary outputs go, and how users export results.

## Decision

Process all files in **app-private storage** (`/data/data/com.stirlingmobile/files/`). Never write to shared/public storage without explicit user action via SAF.

## Options Considered

### Option A: App-private storage only (selected)
- **Pros**: Android sandbox protects files from other apps. No storage permissions needed on API 30+. Files automatically deleted on app uninstall. Simpler security model (no external file access vectors).
- **Cons**: Files invisible to file manager apps (users can't browse outputs outside the app). Must use SAF for export — adds one step to "save file" flow.

### Option B: Shared storage (Downloads/, Documents/)
- **Pros**: Users can browse output files directly in file manager. Simpler mental model ("it's just a file on my phone").
- **Cons**: Requires WRITE_EXTERNAL_STORAGE on older APIs. Other apps can read processed PDFs (security risk). Files persist after app uninstall (privacy leak). Android 11+ scoped storage restrictions make this complex.

### Option C: Hybrid (process in private, save to shared)
- **Pros**: Security of private processing, convenience of shared output.
- **Cons**: Extra I/O (copy from private to shared). User must still use SAF to pick save location on API 30+ (scoped storage blocks direct writes to Downloads).

## Decision Rationale

App-private storage is the only correct choice for a privacy-first PDF app:
1. **Security**: PDFs may contain PII, financial data, legal documents. They must not be readable by other apps.
2. **Privacy**: Files deleted on uninstall = no data remnants. User trusts us with sensitive documents.
3. **Simplicity**: No storage permission complexity. SAF handles all file I/O on API 26+.
4. **Consistency**: Our core value proposition is "everything stays on-device, in your control." App-private storage enforces this by design.

The "inconvenience" of SAF export is actually a feature: it forces the user to consciously choose where their processed file goes, rather than silently dumping it into Downloads.

## Temp File Lifecycle

```
Import: SAF URI → copy to /files/imported/{uuid}.pdf
Process: read from /files/imported/ → write to /files/working/{session}/{step}.pdf
Export: read from /files/working/ → copy to user-chosen SAF destination
Cleanup:
  - /files/working/ → deleted on session end
  - /files/imported/ → pruned after 7 days unused
  - /files/temp/ → deleted on engine shutdown
```

## Consequences

### Positive
- Strongest possible security model for user files
- No storage permissions required on modern Android
- No file leakage between apps
- Clean deletion on uninstall
- Simplified threat model (see `09-security.md`)

### Negative
- Extra tap for user to export (SAF file picker)
- Files not visible in system file manager during processing
- Cannot use file manager apps to find processed PDFs (by design)
- Migration complexity if we later want cloud sync (would need import from cloud → process → export to cloud)

### Mitigations
- "Share" button for quick sending without saving (Android share sheet with temporary content URI)
- Recent files list in app provides quick access to previously imported files
- Clear UX: "Your files are processed privately on this device" messaging
- "Open with" integration: file managers can "Open with → Stirling Mobile" to import
