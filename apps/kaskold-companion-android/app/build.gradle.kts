import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.FileSystemOperations
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import javax.inject.Inject

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    jacoco
}

val repositoryRoot = rootProject.projectDir.resolve("../..").canonicalFile
val companionWeb = repositoryRoot.resolve("apps/kaskold-companion-web")
val canonicalCompanionSite = repositoryRoot.resolve("target/kaskold-companion-web/site")
val companionWebUiAssets = layout.buildDirectory.dir("generated/kaskold-companion-web-ui")
val runtimeBuilder = repositoryRoot.resolve("tools/build/web/build_companion_runtime.py")
val pythonCommand = providers.environmentVariable("PYTHON").orElse(
    if (System.getProperty("os.name").lowercase().contains("windows")) "python" else "python3",
)

val syncKasKoldRuntime by tasks.registering(Exec::class) {
    group = "build setup"
    description = "Builds the canonical KasKold Companion Web/WASM runtime under repository target/."

    inputs.file(runtimeBuilder)
    inputs.file(repositoryRoot.resolve("qa/config/toolchains.env"))
    inputs.file(companionWeb.resolve("Cargo.toml"))
    inputs.file(companionWeb.resolve("Cargo.lock"))
    inputs.files(fileTree(companionWeb.resolve("src")) { include("**/*.rs") })
    inputs.files(fileTree(companionWeb.resolve("web")) { exclude("pkg/**") })
    for (crate in listOf("online-watcher", "shared-signer", "offline-signer")) {
        inputs.file(repositoryRoot.resolve("crates/$crate/Cargo.toml"))
        inputs.files(fileTree(repositoryRoot.resolve("crates/$crate/src")) { include("**/*.rs") })
    }
    outputs.dir(canonicalCompanionSite)
    commandLine(pythonCommand.get(), runtimeBuilder.absolutePath, "--mode", "release")
}

val verifyKasKoldRuntime by tasks.registering {
    group = "verification"
    description = "Fails if the canonical KasKold Companion Web runtime is incomplete."
    dependsOn(syncKasKoldRuntime)
    doLast {
        val required = listOf(
            canonicalCompanionSite.resolve("index.html"),
            canonicalCompanionSite.resolve("css/app.css"),
            canonicalCompanionSite.resolve("js/main.js"),
            canonicalCompanionSite.resolve("js/mobile/native_adaptations.js"),
            canonicalCompanionSite.resolve("pkg/companion_web.js"),
            canonicalCompanionSite.resolve("pkg/companion_web_bg.wasm"),
        )
        val missing = required.filterNot { it.isFile && it.length() > 0L }
        check(missing.isEmpty()) {
            "Canonical Companion runtime is incomplete: ${missing.joinToString { it.relativeTo(repositoryRoot).path }}"
        }
    }
}

abstract class SyncCompanionWebUiTask : DefaultTask() {
    @get:InputDirectory
    abstract val webSite: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @get:Inject
    abstract val fileSystemOperations: FileSystemOperations

    @TaskAction
    fun sync() {
        fileSystemOperations.sync {
            into(outputDirectory.get().asFile)
            from(webSite.get().asFile) { into("companion") }
        }
    }
}

val syncCompanionWebUi = tasks.register<SyncCompanionWebUiTask>("syncCompanionWebUi") {
    dependsOn(verifyKasKoldRuntime)
    group = "build setup"
    description = "Copies the canonical Companion site into Gradle-owned generated Android assets."
    webSite.fileValue(canonicalCompanionSite)
    outputDirectory.set(companionWebUiAssets)
}

android {
    namespace = "com.kaskold.companion"
    compileSdk = 37

    defaultConfig {
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        applicationId = "com.kaskold.companion"
        minSdk = 26
        targetSdk = 37
        versionCode = 20000
        versionName = "2.0.0"
        vectorDrawables.useSupportLibrary = true
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    testOptions.unitTests.isIncludeAndroidResources = true
    buildTypes {
        debug { enableUnitTestCoverage = true }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
}

androidComponents {
    onVariants(selector().all()) { variant ->
        variant.sources.assets?.addGeneratedSourceDirectory(
            syncCompanionWebUi,
            SyncCompanionWebUiTask::outputDirectory,
        )
    }
}

tasks.named("preBuild") {
    dependsOn(syncCompanionWebUi)
}

// Gradle 9.x may execute asset merging independently of preBuild ordering.
// Make the generated Companion asset producer an explicit prerequisite so the
// mapped output directory is never queried before syncCompanionWebUi completes.
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("Assets") }.configureEach {
    dependsOn(syncCompanionWebUi)
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.06.01")
    implementation(composeBom)

    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.fragment:fragment-ktx:1.9.0")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")
    implementation("androidx.webkit:webkit:1.17.0")
    implementation("androidx.biometric:biometric:1.1.0")

    testImplementation("junit:junit:4.13.2")
    testImplementation("androidx.test:core-ktx:1.7.0")
    testImplementation("org.robolectric:robolectric:4.16.1")
    androidTestImplementation("androidx.test:core-ktx:1.7.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("androidx.test:rules:1.7.0")
    androidTestImplementation("androidx.test.ext:junit-ktx:1.3.0")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.7.0")
    debugImplementation("androidx.compose.ui:ui-tooling")
}
