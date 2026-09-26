[KasKold](../../README.md) › [Documentation](../../docs/README.md) › [Companion](../../docs/companion/COMPANION.md) › iOS

# KasKold Companion iOS

KasKold Companion iOS is a native Swift security/platform shell around the same Companion
wallet used by the browser application. It does not maintain a second native
wallet, transaction builder, QR/signing engine, resolver, or watch-only database.

## Qualification

KasKold Companion iOS has been tested on **macOS Sonoma with Xcode 16.2**, including XCTest and the native iOS mutation gate. Distribution releases additionally require an Apple signing identity/provisioning profile and physical-device smoke evidence.

## Architecture

- `KasKold/Features/Root/` hosts the synchronized Companion UI in a loopback-only `WKWebView`.
- `KasKold/Features/Settings/` and `KasKold/Features/Cover/` provide native app-lock/security and weather privacy/decoy behavior.
- `KasKold/Infrastructure/` owns only native shell persistence/security responsibilities.
- `target/companion-runtime/ios/CompanionUI/` is generated staging for the complete Companion application. Xcode references that build-owned resource tree directly; generated Companion runtime files are not written under `KasKold/Resources/`.

## macOS / Xcode setup

Install full Xcode, select it with `xcode-select`, complete Xcode first-launch setup, and ensure GNU Make and Python 3 are available. From the repository root, use the public Make interface:

```text
make ios
make ios-test
make ios-qa
make ios-release
```

`make ios` builds the shared KasKold Companion Web/WASM runtime and then performs the Xcode Simulator build. On success it prints the generated application path under `target/ios/DerivedData/Build/Products/Debug-iphonesimulator/KasKold.app`. `make ios-test` writes and prints `target/ios/KasKoldTests.xcresult`. `make ios-qa` performs the Xcode tests followed by the iOS architecture, CRAP, and full native-shell mutation gates.

## Runtime synchronization

The Xcode target's **Sync shared Companion runtime** phase runs the canonical
`tools/build/web/build_companion_runtime.py`, then the platform-neutral
`tools/build/ios/sync_runtime.py`. The generated runtime stays under `target/`;
Xcode does not write generated Companion runtime files into authored iOS sources.


## Distribution signing

The repository does not pin a developer-specific Apple Team ID. Simulator builds do not require signing. For a device/App Store archive, supply the publishing team's Apple Developer Team ID without committing it:

```bash
KASKOLD_IOS_DEVELOPMENT_TEAM=ABCDE12345 make ios-release
```

`make ios-release` creates `target/ios/KasKold.xcarchive`. Distribution/export from that archive uses the publisher's local Xcode/App Store Connect credentials and provisioning. Signing identities, provisioning profiles, and team IDs are deployment credentials rather than source configuration.
