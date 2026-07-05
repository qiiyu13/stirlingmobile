# Stirling Mobile — Play Store Checklist

> **Status:** Draft v1.0
> **Target:** Production release on Google Play Store

---

## 1. Store Listing Assets

**Before locking in the app name:** this is an independent port of Stirling-PDF, not an official release. Confirm naming is acceptable with the Stirling-Tools maintainers (or check their trademark/license policy) before submission — see M4 in 14-risk-register.md. Rebranding after launch is far more expensive than before.

### 1.1 Text Content

| Item | Format | Status |
|---|---|---|
| App name | "Stirling PDF Mobile" (max 30 chars) | ⏳ |
| Short description | "50 PDF tools. 100% offline. No ads. Open source." (max 80 chars) — not "50+", v1 ships exactly 50 (04-feature-catalog.md); update to "57+" once v1.1's Collabora tools ship | ⏳ |
| Full description | Feature list, privacy emphasis, open-source note (max 4000 chars) | ⏳ |
| Developer name | (Your developer account name) | ⏳ |
| Website | Link to Stirling PDF or GitHub | ⏳ |
| Email | Support contact email | ⏳ |
| Privacy policy URL | Link to hosted privacy policy page | ⏳ |

### 1.2 Graphics

| Asset | Spec | Status |
|---|---|---|
| App icon | 512x512 PNG, adaptive icon (foreground + background) | ⏳ |
| Feature graphic | 1024x500 PNG, shown at top of store listing | ⏳ |
| Screenshots (phone) | Minimum 2, max 8. 1080p resolution recommended. | ⏳ |
| Screenshots (7" tablet) | Optional, same specs | ⏳ |
| Screenshots (10" tablet) | Optional | ⏳ |
| Video (YouTube) | Optional, linked from listing | ⏳ |

**Required screenshot scenarios:**
1. Home screen with tool grid
2. PDF viewer screen showing a rendered page
3. Merge tool with selected files
4. Compress tool showing before/after sizes
5. OCR tool with progress
6. Sign tool interface
7. Dark mode variant of home screen
8. Settings screen

### 1.3 Feature Graphic Guidelines

Capture the value proposition:
- "50 PDF Tools — 100% Offline" (not "50+" until v1.1 adds Collabora's 7)
- "Open Source. No Ads. No Tracking."
- Stirling logo + Play Store badge

---

## 2. App Content

### 2.1 Content Rating

Complete the Google Play Console content rating questionnaire.

Expected rating: **Everyone** or **Everyone 10+**

Reasoning: The app processes documents, no user-generated content, no communication features, no age-gated functionality.

### 2.2 Target Audience

- Age: 13+
- No special child-directed features

---

## 3. Data Safety Section

### 3.1 Declaration: "No data collected"

| Question | Answer |
|---|---|
| Does your app collect any user data? | **No** |
| Does your app share any user data? | **No** |

This is valid because:
- No network permission in the base module manifest (see 09-security.md §2.1); INTERNET is isolated to the optional OCR dynamic feature module
- No analytics SDK
- No crash reporting SDK
- Files processed in app-private storage only
- No user accounts, no sign-in
- OCR language downloads are the **only** network call, and: (a) user-initiated, not automatic; (b) downloads public tessdata files, no user data sent

If reviewer flags OCR download as "data collection":
- Amend to: "App activity — App interactions (Optional) — for downloading OCR language packs on user request only"
- This still qualifies as minimal data collection

### 3.2 Privacy Policy

Must be a publicly accessible URL. Content:

```
# Stirling PDF Mobile — Privacy Policy

## Data Collection
Stirling PDF Mobile does not collect, store, or transmit any personal information.

All PDF files are processed entirely on your device. No files, metadata, or
usage data is sent to any server.

## Internet Access
This app does not require internet access. The only optional network request
is downloading OCR language packs, which is initiated by you and downloads
from public GitHub repositories (Tesseract OCR trained data).

## Third-Party Services
This app does not integrate any third-party analytics, advertising, or
tracking services.

## Changes
If this policy changes, the update will be posted within the app and on this page.

## Contact
[Your support email]

Last updated: [Date]
```

---

## 4. Build & Signing

### 4.1 Release Build Requirements

```bash
# Build the release APK
make build-release

# Output:
# app/build/outputs/apk/full/release/app-full-release.apk
# app/build/outputs/apk/lite/release/app-lite-release.apk
```

### 4.2 Signing

- **Option A (Recommended):** Google Play App Signing
  - Google manages the signing key
  - You upload with an upload key, Google re-signs with the app signing key
  - Safer — key cannot be lost, Google can verify authenticity

- **Option B:** Self-managed signing key
  - You hold the keystore
  - If lost, cannot update the app ever again
  - Store keystore in password manager, backup offline

```bash
# Generate upload key (for Play App Signing)
keytool -genkey -v -keystore upload-keystore.jks \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias stirlingmobile

# In app/build.gradle.kts:
android {
    signingConfigs {
        create("release") {
            storeFile = file("upload-keystore.jks")
            storePassword = System.getenv("KEYSTORE_PASSWORD")
            keyAlias = "stirlingmobile"
            keyPassword = System.getenv("KEY_PASSWORD")
        }
    }
}
```

### 4.3 App Bundle vs APK

- **Use Android App Bundle (AAB)** for Play Store
- APK is for direct distribution only
- AAB enables:
  - Dynamic delivery (download OCR languages on demand)
  - APK splitting by ABI (smaller downloads per device)
  - Play Feature Delivery (Collabora SDK as install-time module)

---

## 5. Pre-Submission Testing

**Hard gate before anything else in this section:** all 57 v1 tools (04-feature-catalog.md) pass every acceptance criterion in 00-spec.md §6. Verify this first — the checks below don't matter if a tool is still broken.

| Test | Status |
|---|---|
| All 57/57 v1 tools pass 00-spec.md §6 acceptance criteria | ⏳ |
| App runs on API 26 device | ⏳ |
| App runs on API 34 device | ⏳ |
| App handles process death gracefully | ⏳ |
| No crash on first launch (cold start) | ⏳ |
| No ANR during any operation | ⏳ |
| All screens render in light + dark mode | ⏳ |
| All strings translated (or fall back to English) | ⏳ |
| RTL layout functional (Arabic test) | ⏳ |
| No network calls from base module (verified via Network Profiler); OCR feature module's tessdata fetch is the sole exception, isolated per 09-security.md §2.1 | ⏳ |
| Accessibility: TalkBack usable on home, viewer, one tool screen | ⏳ |
| APK size < 40MB (v1, single build, no Collabora) | ⏳ |
| Privacy policy URL accessible and correct | ⏳ |
| Content rating questionnaire completed | ⏳ |
| Data safety form submitted | ⏳ |

---

## 6. Release Tracks

| Track | Testers | Purpose |
|---|---|---|
| Internal testing | Up to 100 | Quick distribution to team. No review required. |
| Closed alpha | Up to 100 (via email/group) | Early feedback from trusted users. |
| Open beta | Unlimited | Public testing before production. |
| Production | Unlimited | Full release to all users. |

### Recommended flow:

```
Internal test (1 week)
  → Closed alpha (2 weeks)
    → Open beta (2 weeks)
      → Production
```

---

## 7. Post-Launch

### 7.1 Monitoring

- **Android Vitals** (Play Console) — ANR rate, crash rate, startup time
- **Ratings & Reviews** — Respond to every review in the first month
- **Pre-launch report** — Play Console runs automated tests on your APK across devices

### 7.2 Crash Thresholds

| Metric | Good | Bad (fix immediately) |
|---|---|---|
| ANR rate | < 0.47% | > 0.47% |
| Crash rate | < 1.09% | > 1.09% |
| User-perceived crash rate | < 0.47% | > 0.47% |

These are Google's official "bad behavior" thresholds that can affect your store listing visibility.

### 7.3 First Update (v1.0.1)

Within 1 week of launch: fix any crashes from Android Vitals, address critical reviews.

---

## 8. Checklist Summary

- [ ] Developer account registered ($25 one-time fee)
- [ ] App icon designed (adaptive)
- [ ] Feature graphic designed
- [ ] 6-8 screenshots captured
- [ ] Store listing text written (short + full description)
- [ ] Privacy policy page published (URL owned by you)
- [ ] Content rating questionnaire completed
- [ ] Data safety form ("No data collected") submitted
- [ ] Signing key generated (or Play App Signing opted-in)
- [ ] Release build signed
- [ ] AAB uploaded to internal testing track
- [ ] QA completed on 3 physical devices
- [ ] Beta testing feedback addressed
- [ ] Production release submitted for review
