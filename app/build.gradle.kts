plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.stirlingmobile"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.stirlingmobile"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
        create("benchmark") {
            initWith(getByName("release"))
            matchingFallbacks += listOf("release")
            isDebuggable = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
        // pdfium-render dlopen()'s libpdfium.so by filesystem path at
        // runtime (it's not loaded via System.loadLibrary), so it needs to
        // actually be extracted to nativeLibraryDir on install rather than
        // left mmap'd uncompressed inside the APK (AGP's default since
        // native libs are page-aligned).
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.09.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.9.2")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.6")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.6")
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("androidx.documentfile:documentfile:1.0.1")
    // On-device OCR: PaddleOCR PP-OCRv5 mobile via the official ppocr-sdk
    // (pure-Kotlin AAR built from PaddleOCR/deploy/ppocr-android, vendored in
    // libs/). It runs det+rec on a Bitmap and returns text + boxes; Rust only
    // overlays those as an invisible text layer. onnxruntime + OpenCV are the
    // SDK's transitive engine deps.
    implementation(files("libs/ppocr-sdk-release.aar"))
    implementation("com.microsoft.onnxruntime:onnxruntime-android:1.21.1")
    // Official OpenCV (Maven Central since 4.9). The stale quickbirdstudios
    // 4.5.3 AAR the SDK defaults to imports a dead libc++ symbol
    // (__sfp_handle_exceptions) and crashes on modern devices; same org.opencv.*
    // API, so this is a drop-in.
    implementation("org.opencv:opencv:4.11.0")
    implementation("androidx.core:core-ktx:1.15.0")

    debugImplementation("androidx.compose.ui:ui-tooling")
}
