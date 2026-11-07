plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// Optional: `MIXAR_EXTRA_JNI_DIR` points at a jniLibs-shaped directory
// (`arm64-v8a/*.so`) holding a custom ONNX Runtime build (ACL/QNN/TVM EPs)
// plus its runtime sidecars. When set, it replaces the Microsoft AAR below.
// Read once: the android source set and the dependency block must agree.
val extraJniDir: String? = System.getenv("MIXAR_EXTRA_JNI_DIR")
val useExtraJni = extraJniDir != null

android {
    namespace = "top.mixar.gui_flutter"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    extraJniDir?.let { sourceSets.getByName("main").jniLibs.srcDir(it) }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "top.mixar.app"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        // midir links `libamidi.so` (API 29+); cpal's AAudio backend needs 26+.
        // `maxOf` keeps a future Flutter floor above 29 (3.47's is 24).
        minSdk = maxOf(flutter.minSdkVersion, 29)
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName

        ndk {
            // 64-bit only: 32-bit Android Rust targets are dropped, and stems
            // only needs the arm64-v8a/x86_64 ONNX Runtime AAR libraries.
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    packaging {
        jniLibs {
            // 64-bit only: the app ships arm64-v8a/x86_64 only, but AARs and
            // Flutter artifacts still carry 32-bit directories.
            excludes += listOf("lib/armeabi-v7a/**", "lib/x86/**")
            // Stem separation only needs the runtime library; the Java/JNI shim
            // that ships in the same AAR is unused.
            excludes += "**/libonnxruntime4j_jni.so"
        }
    }
}

dependencies {
    // ONNX Runtime for stems: NNAPI + XNNPACK execution providers, loaded at
    // runtime by `ort`'s `load-dynamic` from `lib/<abi>/libonnxruntime.so`.
    // A custom build (MIXAR_EXTRA_JNI_DIR) carries its own copy instead.
    if (!useExtraJni) {
        implementation("com.microsoft.onnxruntime:onnxruntime-android:1.31.0")
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
