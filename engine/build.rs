use std::env;
use std::path::PathBuf;

/// Links the vendored libqpdf (+ its zlib/libjpeg deps) for Android targets.
/// Static libs live in `engine/native/<abi>/lib`, built via NDK CMake toolchain
/// (see docs/12-dev-environment.md and ADR-003). Host builds (desktop tests)
/// skip this entirely — optimize.rs's qpdf FFI is Android-only.
fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let (abi, ndk_triple) = match target.as_str() {
        "aarch64-linux-android" => ("arm64-v8a", "aarch64-linux-android"),
        "armv7-linux-androideabi" => ("armeabi-v7a", "arm-linux-androideabi"),
        "x86_64-linux-android" => ("x86_64", "x86_64-linux-android"),
        _ => return,
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let lib_dir = manifest_dir.join("native").join(abi).join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=qpdf");
    println!("cargo:rustc-link-lib=static=jpeg");
    println!("cargo:rustc-link-lib=static=z");

    let ndk_home = env::var("ANDROID_NDK_HOME")
        .or_else(|_| env::var("ANDROID_NDK_ROOT"))
        .or_else(|_| env::var("NDK_HOME"))
        .expect("ANDROID_NDK_HOME (or ANDROID_NDK_ROOT/NDK_HOME) must be set to link libqpdf for Android");
    let ndk_lib_dir = PathBuf::from(&ndk_home)
        .join("toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib")
        .join(ndk_triple);

    // The API-versioned dir (matching app/build.gradle.kts minSdk=26) has the
    // real libc.so stub. The base triple dir only has libc.a, whose ifunc
    // dispatch objects (dynamic_function_dispatch.o) aren't PIC-safe in a
    // shared object — pulling libc statically here breaks x86_64 with
    // "recompile with -fPIC". Search the API dir first so bare `-lc`
    // (needed by libqpdf's C++ memset/memcpy calls) resolves dynamically.
    println!(
        "cargo:rustc-link-search=native={}",
        ndk_lib_dir.join("26").display()
    );
    println!("cargo:rustc-link-lib=dylib=c");

    // libc++_static pulls in duplicate __aeabi_* builtins that clash with
    // Rust's compiler_builtins on armeabi-v7a (NDK linker "undefined version
    // LIBC_N" bug) — use the shared libc++ instead. Its .so must ship in
    // jniLibs alongside libstirling_engine.so (see docs/12-dev-environment.md).
    println!("cargo:rustc-link-search=native={}", ndk_lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=c++_shared");

    // Rust's compiler_builtins exports versioned __aeabi_memcpy-family symbols
    // (@@LIBC_N) on armv7-android; rustc's auto version-script only lists our
    // uniffi FFI exports and pairs it with --no-undefined-version, which
    // rejects those versioned symbols once libqpdf (C++, ARM EABI) pulls them
    // in. `--undefined-version` doesn't override the no- flag in lld, so
    // supply a second version-script declaring the missing node instead.
    if abi == "armeabi-v7a" {
        let script = manifest_dir.join("native").join("libc_n.version");
        std::fs::write(&script, "LIBC_N {\n};\n").expect("write libc_n.version script");
        println!(
            "cargo:rustc-link-arg=-Wl,--version-script={}",
            script.display()
        );
    }
}
