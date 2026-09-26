plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

val repositoryRoot = rootProject.projectDir.resolve("../..").canonicalFile
val toolchainPins = repositoryRoot.resolve("qa/config/toolchains.env").readLines()
    .mapNotNull { line ->
        val trimmed = line.trim()
        if (trimmed.isEmpty() || trimmed.startsWith("#") || !trimmed.contains("=")) null
        else trimmed.substringBefore("=") to trimmed.substringAfter("=")
    }.toMap()
val stableRust = requireNotNull(toolchainPins["KASKOLD_STABLE_RUST"]) { "KASKOLD_STABLE_RUST pin is required" }
val androidNdk = requireNotNull(toolchainPins["KASKOLD_ANDROID_NDK"]) { "KASKOLD_ANDROID_NDK pin is required" }

val buildVaultRuntime by tasks.registering(Exec::class) {
    group = "build setup"
    description = "Builds the network-free Rust vault-runtime static library for Android."
    workingDir(repositoryRoot)
    inputs.file(repositoryRoot.resolve("Cargo.toml"))
    inputs.file(repositoryRoot.resolve("Cargo.lock"))
    for (crate in listOf("vault-runtime", "hot-wallet", "offline-signer", "kaskold-protocol", "shared-signer")) {
        inputs.file(repositoryRoot.resolve("crates/$crate/Cargo.toml"))
        inputs.files(fileTree(repositoryRoot.resolve("crates/$crate/src")) { include("**/*.rs") })
    }
    outputs.files(
        repositoryRoot.resolve("target/aarch64-linux-android/release/libvault_runtime.a"),
        repositoryRoot.resolve("target/x86_64-linux-android/release/libvault_runtime.a"),
    )
    commandLine(
        "cargo", "+$stableRust", "ndk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "--platform", "26",
        "build", "-p", "vault-runtime", "--release", "--locked",
    )
}

android {
    namespace = "com.kaskold.vault"
    compileSdk = 37
    ndkVersion = androidNdk

    defaultConfig {
        applicationId = "com.kaskold.vault"
        minSdk = 26
        targetSdk = 37
        versionCode = 20000
        versionName = "2.0.0"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
        externalNativeBuild {
            cmake { arguments += "-DKASKOLD_REPOSITORY_ROOT=${repositoryRoot.absolutePath}" }
        }
    }

    buildFeatures { compose = true }
    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
}

tasks.named("preBuild") { dependsOn(buildVaultRuntime) }
tasks.configureEach {
    if (name.startsWith("externalNativeBuild")) dependsOn(buildVaultRuntime)
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.06.01")
    implementation(composeBom)
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")
    implementation("androidx.camera:camera-core:1.5.0")
    implementation("androidx.camera:camera-camera2:1.5.0")
    implementation("androidx.camera:camera-lifecycle:1.5.0")
    implementation("androidx.camera:camera-view:1.5.0")
    implementation("com.google.mlkit:barcode-scanning:17.3.0")
    implementation("com.google.zxing:core:3.5.3")
    debugImplementation("androidx.compose.ui:ui-tooling")
}
