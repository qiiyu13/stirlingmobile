# Stirling Mobile — Security

> **Status:** Draft v1.0
> **Principle:** All PDF data stays on-device. No network. No telemetry. No exceptions.

---

## 1. Threat Model

### 1.1 Assets

| Asset | Sensitivity | Storage |
|---|---|---|
| User PDF files (imported) | High — may contain PII, financial data, legal docs | App-private storage |
| PDF content in memory | High — decrypted, readable | RAM only |
| Passwords for encrypted PDFs | High — temporary, per-session | RAM only, zeroed after use |
| Signing certificates (PFX) | Critical — identity | App-private storage, user-managed |
| Signing certificate passwords | Critical | RAM only, zeroed after use |
| Tool operation history | Low — file names, operation types | SQLite (Room) |
| App preferences | Low | DataStore |

### 1.2 Threat Actors

| Actor | Capability | Concern |
|---|---|---|
| Other apps on device | Can read public storage, cannot read app-private storage | Low risk (Android sandbox) |
| Malicious PDF | Crafted PDF exploiting parser vulnerability | Medium risk |
| Device theft | Physical access to unlocked device | Low risk (files in app-private, encrypted at rest if device encrypted) |
| User (accidental) | Shares wrong file | UX mitigation (preview before share) |
| Google Play scanning | Automated malware scanning | Low — we're FOSS, no obfuscation needed |

### 1.3 Attack Surfaces

| Surface | Risk | Mitigation |
|---|---|---|
| PDF parsing (lopdf) | Crafted PDF → parser bug → crash/exploit | `catch_unwind` boundary, fuzz testing |
| JNI boundary | Buffer overflow, use-after-free | Rust memory safety, no `unsafe` in JNI layer |
| File I/O (path traversal) | User-supplied file path escapes sandbox | Canonicalize all paths, reject `..` |
| Collabora SDK | LibreOffice vulnerabilities | Update SDK regularly, validate output |
| WebView (HTML→PDF) | XSS in user HTML | Sandbox WebView, no network access |
| Memory (sensitive data) | Passwords linger in RAM | Zero memory after use, `secrecy` crate |
| Intent handling | Malicious app sends crafted intent | Only accept PDF/image MIME, copy to app-private first |

---

## 2. Sandbox Rules

### 2.1 Android Permissions

```xml
<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE"
    android:maxSdkVersion="29" />
<uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE"
    android:maxSdkVersion="29" />
<!-- minSdk is 26, SAF/scoped storage only fully replaces these on API 30+.
     On API 26-29 the app requests them at runtime for the legacy file picker
     path; on API 30+ they're capped out and SAF handles all file access. -->

<!-- Explicitly NOT requested in the base module -->
<!-- <uses-permission android:name="android.permission.INTERNET" /> -->
```

**No INTERNET permission in the base module.** This is the base app's core security property, verified at build time: if INTERNET permission leaks into the base APK (from a library), the build fails. The OCR language-pack feature (F-071, see 10-i18n.md §6) ships as a separate dynamic feature module that declares INTERNET on its own — installed and requested only when the user opts into downloading a language pack. The base app and all other tools stay network-free; see NF-020 in 00-spec.md.

**API 26-29 file access.** Below API 30, `READ/WRITE_EXTERNAL_STORAGE` are requested at runtime the first time the user picks a file outside app-private storage. Denial degrades gracefully to app-private storage only (import via share-sheet, no external browse) — same UX contract as NF-012.

### 2.2 File Access Policy

```
User file from SAF
  → Copy to app-private storage FIRST
  → Process on copy only
  → Original untouched

Temporary files
  → /data/data/com.stirlingmobile/files/temp/
  → Wiped on engine shutdown

Export
  → User explicitly chooses destination via SAF
  → App never writes outside without user consent
```

### 2.3 WebView Sandbox (HTML→PDF only)

```kotlin
webView.settings.apply {
    javaScriptEnabled = true       // Needed for HTML rendering
    allowFileAccess = false        // No local file access
    allowContentAccess = false      // No content providers
    databaseEnabled = false
    domStorageEnabled = false
    allowFileAccessFromFileURLs = false
    allowUniversalAccessFromFileURLs = false
    // No network — only render supplied HTML string
}

// WebView is in a separate process (:webview)
// android:process=":webview"
// Destroyed after rendering completes
```

---

## 3. Cryptographic Operations

### 3.1 PDF Encryption

Used for the "Add Password" and "Remove Password" tools:

```
Algorithm: AES-128 or AES-256 (per PDF 2.0 spec)
Mode: CBC with 16-byte IV (128-bit) or 32-byte IV (256-bit)
Key derivation: Adobe standard (revision 5/6 per ISO 32000-2)
Implementation: ring + aes crates in Rust
```

User-supplied passwords are never stored. They are held in `secrecy::SecretString` in Rust and zeroed after the operation completes.

### 3.2 Digital Signatures

```
Certificate format: PKCS#12 (.pfx/.p12)
Hashing: SHA-256 (minimum), SHA-384/SHA-512 (preferred)
Signature: RSA-PKCS#1 or RSA-PSS
Implementation: ring + rcgen + x509-parser (Rust)
```

The PFX file is read from user-selected location (SAF), never copied or cached. Its password is held in `secrecy::SecretString`, used, then zeroed.

### 3.3 Random Number Generation

```
Source: Android's SecureRandom (fed from Linux kernel CSPRNG)
Rust: ring::rand::SystemRandom (reads /dev/urandom, available on Android)
Never use: kotlin.random.Random, rand::thread_rng (non-cryptographic)
```

---

## 4. Secure Coding Practices

### 4.1 Rust

- `#![forbid(unsafe_code)]` in engine crate (except JNI FFI module)
- JNI FFI module: `unsafe` blocks are minimal, reviewed, and documented
- All `#[no_mangle]` JNI functions wrapped in `catch_unwind`
- Fuzz testing with `cargo-fuzz` on PDF parser, merge, and split
- `cargo audit` in CI to catch vulnerable dependencies

### 4.2 Kotlin

- `minSdk = 26` — no legacy security holes (TLS 1.2+, hardened libcore)
- No reflection (ProGuard/R8 strips unused code, reflection breaks this)
- No dynamic code loading (`DexClassLoader`, `Runtime.exec()`)
- Input validation at trust boundaries (SAF URIs, user-provided strings for metadata)
- All file paths canonicalized before use

---

## 5. Data Safety (Play Store Compliance)

For the Google Play Data Safety section:

| Data type | Collected? | Shared? | Purpose |
|---|---|---|---|
| PDF files | No | No | — |
| Personal info | No | No | — |
| Device ID | No | No | — |
| Crash logs | Optional* | No | Debugging |
| App interactions | No | No | — |
| OCR language pack (traineddata) | No† | No | Downloaded, not uploaded |

\* Crash logs only available via Play Console (Android Vitals), not an SDK. User must opt in to share diagnostics with developers.
† Downloading a language pack sends no user or file data — it's an anonymous GET against a public GitHub URL, from the isolated OCR feature module only.

**Data Safety declaration: "No data collected"** — the base app requests zero network permission; the optional OCR module downloads public static assets only and never transmits user data, so the declaration holds app-wide. Play's reviewer-facing rationale is spelled out in 16-playstore-checklist.md §3.

---

## 6. Incident Response

| Event | Response |
|---|---|
| PDF parser vulnerability (CVE in lopdf) | Update `lopdf` version, release patch within 48hrs |
| Collabora SDK vulnerability | Update SDK, release patch |
| Build compromise (malicious dependency) | `cargo audit` + `gradle dependencyCheck` block CI |
| User reports data leak | Impossible by design (no network). Investigate SAF behavior, file manager caching |

---

## 7. Compliance

| Regulation | Applicability | Notes |
|---|---|---|
| GDPR | Yes (EU users) | No personal data collected. Compliant by default. |
| CCPA | Yes (California users) | No personal data collected. Compliant by default. |
| COPPA | Yes (if used by children) | No data collection. Safe. |
| HIPAA | Not certified | Not a medical app. Users should not store PHI without device-level encryption. |

---

## 8. Build Integrity

- APK signed with developer key (Play App Signing optional but recommended)
- `build.gradle` enforces `minSdk 26`, `targetSdk 35`
- ProGuard/R8 enabled for release builds (shrinks + obfuscates DEX, does not affect .so files)
- `debuggable = false` for release builds
- No embedded API keys, secrets, or tokens (nothing to leak)
