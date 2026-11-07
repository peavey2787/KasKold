#!/usr/bin/env python3
"""Fail-closed source/architecture contracts for the Android Companion shell."""
from __future__ import annotations
from pathlib import Path
import re
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
APP = ROOT / "apps/kaskold-companion-android"
SOURCE = APP / "app/src/main/java/com/kaskold/companion"
VAULT_SOURCE = ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault"
MAIN = SOURCE / "app/MainActivity.kt"
WEB_HOST = SOURCE / "app/CompanionWebViewHost.kt"
MOBILE_BRIDGE = SOURCE / "app/CompanionMobileBridge.kt"
CONTAINER = SOURCE / "app/AppContainer.kt"
BUILD = APP / "app/build.gradle.kts"
MANIFEST = APP / "app/src/main/AndroidManifest.xml"
MAX_KOTLIN_LINES = 450
MAX_DIRECT_KOTLIN_FILES = 8


def add(errors, message): errors.append(message)


def check_structure(errors):
    for name in ("app", "domain", "features", "infrastructure", "shared"):
        if not (SOURCE / name).is_dir(): add(errors, f"missing Android source responsibility root: {name}")
    for path in (SOURCE / "generated", SOURCE / "infrastructure/engine", SOURCE / "domain/runtime",
                 SOURCE / "domain/serialization", SOURCE / "domain/transaction", SOURCE / "domain/wallet",
                 SOURCE / "features/send", SOURCE / "features/receive", SOURCE / "features/scanner",
                 SOURCE / "features/wallet", SOURCE / "features/utxos", SOURCE / "features/activity"):
        if path.exists(): add(errors, f"retired native-wallet Android surface must not exist: {path.relative_to(ROOT)}")
    forbidden_files = (
        SOURCE / "app/AppSecurityScreen.kt", SOURCE / "features/root/RootScreen.kt",
        SOURCE / "features/settings/SettingsScreen.kt",
        APP / "app/src/main/assets/web/bridge.js", APP / "app/src/main/assets/web/bridge.html",
        SOURCE / "infrastructure/security/PersistentHotWalletStore.kt",
    )
    for path in forbidden_files:
        if path.exists(): add(errors, f"obsolete Android compatibility file must not exist: {path.relative_to(ROOT)}")
    if list(APP.rglob("*.rs")) or (APP / "Cargo.toml").exists():
        add(errors, "Android app must not embed a duplicate Rust runtime/workspace")
    for path in (*SOURCE.rglob("*.kt"), *VAULT_SOURCE.rglob("*.kt")):
        lines = len(path.read_text(encoding="utf-8").splitlines())
        if lines > MAX_KOTLIN_LINES: add(errors, f"Kotlin SRP line limit exceeded ({lines}>{MAX_KOTLIN_LINES}): {path.relative_to(ROOT)}")
    for directory in sorted({p.parent for root in (SOURCE, VAULT_SOURCE) for p in root.rglob("*.kt")}):
        if len(list(directory.glob("*.kt"))) > MAX_DIRECT_KOTLIN_FILES:
            add(errors, f"crowded Android source folder: {directory.relative_to(ROOT)}")


def require_tokens(errors, text, tokens, prefix):
    for token in tokens:
        if token not in text: add(errors, f"{prefix}: {token}")


def check_live_shell_files(errors):
    required = (
        MAIN, WEB_HOST, MOBILE_BRIDGE, CONTAINER, SOURCE / "features/root/MobileOverlayScreen.kt",
        SOURCE / "features/settings/SecuritySettingsScreen.kt",
        SOURCE / "features/settings/DecoyLaunchSettingsScreen.kt",
        SOURCE / "features/cover/WeatherCoverScreen.kt",
        SOURCE / "infrastructure/security/AppLockService.kt",
        SOURCE / "infrastructure/network/WeatherService.kt",
        SOURCE / "infrastructure/network/WeatherCoverFacade.kt",
        SOURCE / "infrastructure/persistence/WeatherCoverPreferences.kt",
    )
    for path in required:
        if not path.is_file(): add(errors, f"live Android shell responsibility missing: {path.relative_to(ROOT)}")


def check_live_shell(errors):
    check_live_shell_files(errors)
    main = MAIN.read_text(encoding="utf-8")
    host = WEB_HOST.read_text(encoding="utf-8")
    bridge = MOBILE_BRIDGE.read_text(encoding="utf-8")
    require_tokens(errors, main, ("CompanionWebViewHost(", "MobileOverlayScreen("), "Android thin Activity contract missing")
    require_tokens(errors, host, (
        'COMPANION_URL = "https://$ASSET_HOST/assets/companion/index.html"', "WebViewAssetLoader.Builder()",
        "settings.javaScriptEnabled = true", "settings.allowFileAccess = false", "settings.allowContentAccess = false",
        "WebSettings.MIXED_CONTENT_NEVER_ALLOW", "shouldOverrideUrlLoading", 'addJavascriptInterface(',
        '"KasKoldMobile"', "import('./js/mobile/native_adaptations.js')",
    ), "Android CompanionWebViewHost contract missing")
    require_tokens(errors, bridge, ("@JavascriptInterface", "openMobileSettings", "resetWalletSurface"), "Android native JS bridge contract missing")
    container = CONTAINER.read_text(encoding="utf-8")
    require_tokens(errors, container, ("AppPreferences", "WeatherCoverPreferences", "WeatherCoverFacade", "AppLockService"), "Android native shell dependency missing")
    check_retired_wallet_dependencies(errors)


def check_retired_wallet_dependencies(errors):
    combined = "\n".join(p.read_text(errors="ignore") for p in SOURCE.rglob("*.kt"))
    for token in ("KasKoldEngine", "WalletStore", "WalletSyncService", "QrEngineFacade", "PendingSigningStore",
                  "PersistentHotWalletStore", "persistentHotWallet", "kaskold_hot_wallet"):
        if re.search(rf"(?<![A-Za-z0-9_]){re.escape(token)}(?![A-Za-z0-9_])", combined):
            add(errors, f"retired Android native wallet dependency remains: {token}")


def check_platform(errors):
    build = BUILD.read_text(encoding="utf-8")
    require_tokens(errors, build, ("compileSdk = 37", "targetSdk = 37", 'versionName = "2.0.0"', "JavaVersion.VERSION_17", "isMinifyEnabled = true", "isShrinkResources = true"), "Android build contract missing")
    require_tokens(errors, build, ('into("companion")', 'addGeneratedSourceDirectory(', 'syncCompanionWebUi', 'target/kaskold-companion-web/site', 'build_companion_runtime.py'), "Android complete Companion UI asset contract missing")
    if 'commandLine("bash"' in build or '"bash", "-lc"' in build:
        add(errors, "Android Gradle runtime build must not require bash; use the canonical cross-platform runtime builder")
    if 'src/main/assets/web' in build:
        add(errors, "Android generated Companion runtime must not be staged under authored src/main/assets")
    for token in ("androidx.camera:", "com.google.mlkit:barcode-scanning", "com.google.zxing:core"):
        if token in build: add(errors, f"retired native scanner dependency remains: {token}")
    manifest = MANIFEST.read_text(encoding="utf-8")
    require_tokens(errors, manifest, ('android:allowBackup="false"', 'android:usesCleartextTraffic="false"', "android.permission.INTERNET", "android.permission.CAMERA"), "Android manifest contract missing")
    try: ET.parse(MANIFEST)
    except Exception as exc: add(errors, f"AndroidManifest.xml is not well formed: {exc}")
    sync = (ROOT / "scripts/linux/build/android-runtime-sync.sh").read_text(encoding="utf-8")
    if "tools/build/web/build_companion_runtime.py" not in sync:
        add(errors, "Android runtime sync must delegate to the canonical Companion runtime builder")


def check_tests(errors):
    required = (
        APP / "app/src/test/java/com/kaskold/companion/domain/OverlayPolicyTest.kt",
        APP / "app/src/test/java/com/kaskold/companion/domain/WeatherUnlockPolicyTest.kt",
        APP / "app/src/test/java/com/kaskold/companion/infrastructure/PersistenceRobolectricTest.kt",
        APP / "app/src/test/java/com/kaskold/companion/infrastructure/WeatherServiceRobolectricTest.kt",
        APP / "app/src/test/java/com/kaskold/companion/infrastructure/AppLockServiceRobolectricTest.kt",
        APP / "app/src/androidTest/java/com/kaskold/companion/integration/MainActivityRecreationInstrumentedTest.kt",
    )
    for path in required:
        if not path.is_file(): add(errors, f"Android live-shell regression test missing: {path.relative_to(ROOT)}")


def check_generated_runtime(errors):
    legacy = APP / "app/src/main/assets/web"
    if legacy.exists():
        add(errors, "legacy generated Android runtime must not exist under app/src/main/assets/web")
    generated = APP / "app/build/generated/kaskold-companion-web-ui/companion"
    if generated.exists():
        for relative in ("index.html", "js/main.js", "js/mobile/native_adaptations.js", "pkg/companion_web.js", "pkg/companion_web_bg.wasm"):
            if not (generated / relative).is_file():
                add(errors, f"Android generated Companion site is incomplete: missing {relative}")


def main():
    errors=[]
    check_structure(errors); check_live_shell(errors); check_platform(errors); check_tests(errors); check_generated_runtime(errors)
    if errors:
        for msg in errors: print(f"ERROR: {msg}")
        return 1
    print("PASS: Android direct-Companion shell, mobile security/weather, hardening, and dead-native-wallet contracts.")
    return 0

if __name__ == "__main__": raise SystemExit(main())
