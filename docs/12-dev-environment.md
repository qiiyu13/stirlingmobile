# Stirling Mobile — Dev Environment

> **Status:** Draft v1.0

---

## 1. Prerequisites

| Tool | Minimum Version | Check Command |
|---|---|---|
| Android Studio | Hedgehog (2023.1+) | — |
| JDK | 17 | `java -version` |
| Kotlin | 2.0+ | — (bundled with Android Studio) |
| Rust | 1.80+ | `rustc --version` |
| Android NDK | 27.0+ | `ls $ANDROID_HOME/ndk/` |
| cargo-ndk | 3.5+ | `cargo ndk --version` |
| uniffi-bindgen | 0.28+ | `uniffi-bindgen --version` |
| Git | 2.40+ | `git --version` |

---

## 2. One-Time Setup

```bash
# 1. Clone
git clone https://github.com/your-org/stirlingmobile.git
cd stirlingmobile

# 2. Install Rust targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
rustup component add clippy rustfmt

# 3. Install Android tools
cargo install cargo-ndk
cargo install uniffi-bindgen

# 4. Set ANDROID_HOME (if not already set)
export ANDROID_HOME=$HOME/Android/Sdk
export NDK_HOME=$ANDROID_HOME/ndk/$(ls $ANDROID_HOME/ndk/ | sort -V | tail -1)

# 5. Verify
cargo ndk --version
uniffi-bindgen --version
./gradlew --version
```

---

## 3. Project Structure

```
stirlingmobile/
├── app/                          # Android application module
│   ├── src/
│   │   ├── main/
│   │   │   ├── java/com/stirlingmobile/
│   │   │   │   ├── MainActivity.kt
│   │   │   │   ├── ui/            # Compose screens
│   │   │   │   ├── viewmodel/     # ViewModels
│   │   │   │   ├── repository/    # FileRepo, HistoryRepo
│   │   │   │   ├── engine/        # JNI wrapper (PdfEngine.kt)
│   │   │   │   └── di/            # Manual DI module
│   │   │   ├── res/
│   │   │   └── jniLibs/           # Generated .so files (gitignored)
│   │   ├── test/                  # Kotlin unit tests
│   │   └── androidTest/           # Integration + E2E tests
│   └── build.gradle.kts
├── engine/                        # Rust PDF engine
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 # JNI entry points
│   │   ├── pdf/
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs
│   │   │   ├── writer.rs
│   │   │   └── render.rs
│   │   ├── ops/
│   │   │   ├── pages.rs           # Merge, split, rotate, etc.
│   │   │   ├── content.rs         # Watermark, redact, annotate
│   │   │   └── compare.rs
│   │   ├── security/
│   │   │   ├── encrypt.rs
│   │   │   └── sign.rs
│   │   ├── convert/
│   │   │   ├── images.rs
│   │   │   ├── markdown.rs
│   │   │   └── pdfa.rs
│   │   ├── optimize/
│   │   │   └── compress.rs        # libqpdf NDK wrapper (ADR-003)
│   │   ├── forms/
│   │   │   └── acroform.rs
│   │   ├── metadata.rs
│   │   ├── error.rs
│   │   └── jni_bridge.rs
│   ├── tests/                     # Integration tests
│   └── test_fixtures/             # PDF test files
├── docs/                          # Documentation
├── build.gradle.kts               # Root build file
├── settings.gradle.kts
├── gradle.properties
├── gradlew
└── Makefile                       # Convenience commands
```

---

## 4. Daily Development Commands

### Rust engine

```bash
# Build engine for all Android targets
make build-engine

# Run Rust tests
make test-engine

# Run Rust linter
make lint-engine

# Watch for changes and rebuild
make watch-engine

# Run fuzz tests (requires cargo-fuzz)
make fuzz-engine
```

### Kotlin app

```bash
# Build debug APK
make build-debug

# Install on connected device
make install-debug

# Run Kotlin unit tests
make test-kotlin

# Run Android instrumentation tests
make test-android

# Run lint
make lint-kotlin

# Build release APK
make build-release
```

### Combined

```bash
# Full build: Rust engine + Kotlin app
make build

# Full test: Rust tests + Kotlin tests + Android tests
make test

# Full check: lint + build + test
make check

# Clean all artifacts
make clean
```

---

## 5. Makefile Reference

```makefile
.PHONY: build-engine test-engine lint-engine build-debug test-kotlin test-android build test check clean

ANDROID_TARGETS := aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

build-engine:
	cd engine && cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o ../app/src/main/jniLibs build --release

test-engine:
	cd engine && cargo test

lint-engine:
	cd engine && cargo clippy -- -D warnings && cargo fmt --check

watch-engine:
	cd engine && cargo watch -x "ndk -t arm64-v8a -o ../app/src/main/jniLibs build"

build-debug: build-engine
	./gradlew assembleFullDebug

install-debug: build-debug
	adb install -r app/build/outputs/apk/full/debug/app-full-debug.apk

test-kotlin:
	./gradlew testFullDebugUnitTest

test-android:
	./gradlew connectedFullDebugAndroidTest

build: build-engine build-debug

test: test-engine test-kotlin test-android

check: lint-engine lint-kotlin build test

clean:
	./gradlew clean
	cd engine && cargo clean
	rm -rf app/src/main/jniLibs/*
```

---

## 6. IDE Setup

### Android Studio

1. Open `stirlingmobile/` as an existing project
2. Install recommended plugins: Rust, Kotlin, Compose
3. Set `ANDROID_HOME` in `local.properties`
4. Run configuration: "app" module, `fullDebug` variant
5. For Rust editing: open `engine/` in a separate IDE window (VS Code + rust-analyzer recommended)

### VS Code (Rust engine)

1. Open `stirlingmobile/engine/` in VS Code
2. Install `rust-analyzer` extension
3. `.vscode/settings.json` should set:
   ```json
   {
     "rust-analyzer.cargo.target": "aarch64-linux-android",
     "rust-analyzer.check.command": "clippy"
   }
   ```

---

## 7. CI/CD (GitHub Actions)

```yaml
name: CI
on: [push, pull_request]

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cd engine && cargo test
      - run: cd engine && cargo clippy -- -D warnings
      - run: cd engine && cargo fmt --check

  kotlin:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-java@v4
        with: { java-version: '17' }
      - run: ./gradlew lintFullDebug testFullDebugUnitTest

  android:
    runs-on: macos-latest
    strategy:
      matrix:
        api-level: [30, 34]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-java@v4
        with: { java-version: '17' }
      - uses: reactivecircus/android-emulator-runner@v2
        with:
          api-level: ${{ matrix.api-level }}
          script: |
            make build-engine
            ./gradlew connectedFullDebugAndroidTest

  benchmark:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: reactivecircus/android-emulator-runner@v2
        with:
          api-level: 30
          script: |
            make build-engine
            ./gradlew :app:benchmark:connectedCheck
```

---

## 8. Troubleshooting

| Problem | Solution |
|---|---|
| `cargo ndk` fails: "NDK not found" | Set `ANDROID_NDK_HOME` to NDK path: `export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.0.12077973` |
| JNI function not found at runtime | Run `nm -D app/src/main/jniLibs/arm64-v8a/libstirling_engine.so \| grep Java_` to verify symbol exists |
| Rust compile errors for Android target | Verify `rustup target list --installed` includes Android targets |
| Gradle sync fails | Check `local.properties` has `sdk.dir`, run `./gradlew --refresh-dependencies` |
| Emulator can't run ARM .so | Use x86_64 emulator image. Our engine builds for x86_64 |
| APK too large | Run `make build-release` (LTO + stripping). Debug APK is always larger |
