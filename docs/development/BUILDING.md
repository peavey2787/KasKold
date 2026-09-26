[KasKold](../../README.md) › [Documentation](../README.md) › Development › Building

<!-- KasKold — Air-gapped offline signing device for Kaspa -->
<!-- License: GPL-3.0-only -->

# Building and testing

GNU Make is the small, stable developer interface. Targets describe user intentions; detailed implementation/debug steps remain under `scripts/`, `tools/`, and `qa/`.

## Normal development

```bash
make help
make test
```

`make test` runs the fast host/browser contributor suite. It deliberately runs **no Android, iOS/Xcode, physical-device, or HIL tests**, and it does not run coverage/CRAP generation, mutation, fuzz campaigns, historical regression-policy suites, branch ratchets, architecture policing, firmware build/lint matrices, QEMU, or benchmarks.

Run the specialist assurance umbrella explicitly when needed:

```bash
make qa
```

`make qa` is intentionally expensive and authoritative for the shared **repository/Web/firmware** QA graph. Android and iOS testing is deliberately excluded from this command and runs only through `make android-qa` and `make ios-qa`. Shared QA runs strict coverage/CRAP first, then immediately executes the pinned stable Core CI gate: formatting, workspace/all-target Clippy with warnings denied, strict `make test`, and `git diff --check`. The complete Core CI transcript is retained at `target/qa/core-ci/core-ci.log`. After that gate passes, shared QA continues with strict architecture/security/regression work, browser/QEMU/software integration, real-node and funded/interactive testnet E2E before long unattended campaigns, benchmarks, fresh mutation certification, and fuzzing last. Physical-device/HIL work remains explicit through `make test-hardware`, `make workflow-e2e`, and `make workflow-hil`. To continue a failed shared run without replaying earlier green stages, use `make qa RESUME_FROM=<stable-step-id>`; the named step is rerun and the remaining canonical shared-QA catalog follows.

## Web apps

```bash
make companion
make vault-web
```

`make companion` builds the strictly watch-only **KasKold Companion Web** application. The canonical runnable site is staged under `target/kaskold-companion-web/site/`. For direct local development, the same build also mirrors the generated WASM bindings into `apps/kaskold-companion-web/web/pkg/`, so serving `apps/kaskold-companion-web/web/` works immediately after `make companion`. Companion never creates/restores wallets, owns private keys, or signs locally.

`make vault-web` builds the separate **KasKold Vault Web** signer. Its canonical runnable site is staged under `target/kaskold-vault-web/site/`, with a generated local `pkg/` mirror under `apps/kaskold-vault-web/web/pkg/`. Vault Web owns wallet creation/restoration/review/signing through `vault-runtime` and intentionally contains no blockchain watcher, public-node resolver, or broadcast capability.

Both local `pkg/` mirrors are generated/ignored and are not source-archive content.

## iOS

```bash
make ios
make ios-vault
make ios-release
make ios-test
make ios-qa
```

On macOS these commands operate on the **iOS product family**: the watch-only Companion and the separate Vault signer. `make ios` builds both Debug applications and prints their resulting simulator artifacts; `make ios-vault` builds only the Vault Debug simulator app. `make ios-release` archives both `KasKold.xcarchive` and `KasKoldVault.xcarchive`; `make ios-test` runs the Companion XCTest/XCUITest suite and also builds Vault for the selected simulator to validate its Rust C ABI/Swift bridge. These commands fail clearly outside macOS/Xcode. `make ios-qa` runs the product-family build/test validation plus the strict iOS architecture/CRAP/mutation gates. The repository does not pin an Apple Team ID; device/App Store archives require the publishing team's ID, for example `KASKOLD_IOS_DEVELOPMENT_TEAM=ABCDE12345 make ios-release`.

On macOS, use the same public Make interface: `make ios`, `make ios-vault`, `make ios-test`, `make ios-qa`, and `make ios-release`. Full Xcode must already be installed and selected; the Make targets validate the required repository-pinned Rust/WASM toolchain and fail clearly when an Apple prerequisite is unavailable.

## Android

```bash
make android
make android-vault
make android-release
make android-test
make android-qa
```

These are real Gradle/API-37 operations over the **Android product family**: the watch-only Companion and the separate Vault signer. `make android` builds both Debug apps, `make android-vault` builds only the Vault Debug app, `android-release` builds both optimized Release variants, `android-test` runs both Debug unit-test suites, and `android-qa` adds the strict Android architecture/CRAP/instrumentation/mutation gates plus an optional standalone Kotlin CLI smoke test. If `kotlinc` is not on `PATH`, that duplicate smoke test is skipped; the equivalent policy remains exercised by Gradle/JUnit. Successful Debug/Release builds print the APK paths beneath each application's `app/build/outputs/apk/` tree. The build wrapper reads the pinned API/build-tools versions from `qa/config/toolchains.env`; when they are missing it prefers the current Android CLI (`android sdk install`) and enables the preview channel required by Android 17/API 37, with a `sdkmanager --channel=3` compatibility fallback for older command-line-tools layouts. For targeted debugging, `KASKOLD_ANDROID_PRODUCTS=companion` or `KASKOLD_ANDROID_PRODUCTS=vault` can still restrict the native wrapper directly; the public `make android-vault` target is the normal Vault-only entry point and the default product selection is `all`. Distribution signing is intentionally publisher-controlled: the repository contains no release keystore/signing configuration, and `*.jks`/`*.keystore` are ignored.

## Firmware and devices

`BOARD` defaults to `m5stack`; supported public board values are `m5stack` and `m5stack-lite`. CoreS3 Lite is development/build-supported only until physical qualification is complete.

```bash
make firmware
make firmware BOARD=m5stack-lite

make flash
make flash BOARD=m5stack-lite
make flash PORT=/dev/ttyACM0
make flash BOARD=m5stack-lite PORT=/dev/ttyACM0

On Windows, when `make flash` is launched from a captured/non-terminal runner such as Project Repo Manager, the firmware is flashed first and the UART monitor is then opened in a separate console window on the same selected COM port. The detached monitor attaches with `no-reset-no-sync`, so it does not reboot or reflash the freshly started firmware. Press `CTRL+C` in that monitor console to stop it. A normal interactive terminal keeps the single-process `espflash flash --monitor` behavior.

# Flash an existing signed merged normal-release image only (no rebuild/provisioning)
make flash-release
make flash-release BOARD=m5stack PORT=/dev/ttyACM0 RELEASE_DIR=release

make test-hardware BOARD=m5stack PORT=/dev/ttyACM0
make workflow-e2e BOARD=m5stack PORT=/dev/ttyACM0
make workflow-hil BOARD=m5stack PORT=/dev/ttyACM0
```

Firmware feature flags remain documented in the firmware manifest/docs; there is no public `firmware-features` Make command. Production release flashing is restricted to the hardware-qualified CoreS3 target.

QEMU remains explicit:

```bash
make firmware-qemu-setup
make firmware-qemu
make firmware-qemu-test
```


### Owner-authorized CoreS3 application

After generating an RSA-3072 owner key outside the repository, build the enrollment record and owner-signed application with:

```bash
make owner-firmware OWNER_KEY=/secure/offline/path/owner.pem
```

The generated `OWNERKEY.KAS` and `OWNERFW.BIN` are placed under `target/owner-firmware/`. Back up `owner.pem` before enrollment; it cannot be reconstructed from the enrollment record, device/eFuse state, or a signed image. Owner-key enrollment is an explicit CoreS3 provisioning action before the Settings-only Pop It! flow; `secure-provisioning` uses vendor + optional owner authority, while `secure-owner-only` requires the owner key as the sole authority; normal release firmware omits that UI/path entirely, while development firmware only simulates the irreversible path. See [Pop It! and owner-authorized firmware](../security/POP_IT_SECURE_BOOT.md).

## Release

```bash
make release
make release SIGNING_KEY=/path/to/key.bin RELEASE_DIR=release
```

`make release` builds the normal non-destructive production profile (`production` without `secure-provisioning`): Pop It!, owner-authority UI, and irreversible boot-control staging are not compiled into that firmware. `make flash-release` consumes only the existing signed merged `*-full.bin` plus its `SHA256SUMS`; it does not build, provision eFuses, invoke the special secure profile, or fall back to unsigned artifacts. The dedicated `make secure-provisioning` and `make secure-owner-only` targets build `m5stack,secure-provisioning` and `m5stack,secure-owner-only` respectively; neither target flashes hardware. Both bootloaders defer irreversible flash-encryption/Secure-Boot/anti-rollback changes until explicit Pop It consent. The owner-only variant requires only `OWNER_KEY` and does not require the vendor Schnorr release key.

A production release is always reproducible. The intended release workflow is `make test` → `make qa` → `make test-hardware` → `make workflow-e2e` → `make workflow-hil` → `make release` → `make release-readiness`. The reproducible-build implementation lives under `scripts/` and `tools/`; `make release` builds and manifest-verifies the release artifacts without replaying the preceding test stages. `make release-readiness` is the separate fail-closed gate for operator-supplied source/artifact-bound signed evidence.

## Generated output ownership

Temporary/generated output, including run-specific QA and hardening evidence, belongs under the repository-root `target/qa/` tree (or Gradle `build/generated` for Android); distributables belong under `release/`, and authored source/contracts stay in the repository.

For detailed device behavior, timeouts, UART/HIL evidence, and resume tranches, see [Build, Sign & Flash](BUILD_FLASH_GUIDE.md) and the hardware/HIL documentation rather than `make help`.
