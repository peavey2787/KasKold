[KasKold](../../README.md) › [Documentation](../../docs/README.md) › KasKold Vault

# KasKold Vault for Android

KasKold Vault is a separate air-gapped signing application. It is not an offline mode of Companion.

Security properties enforced by the project:

- the manifest deliberately omits `android.permission.INTERNET`;
- cleartext traffic is disabled;
- no node, HTTP, WebSocket, resolver, telemetry, or broadcast dependency is included;
- wallet/signing logic belongs to the Rust `vault-runtime` / hardened signing core;
- Rust seals each wallet as authenticated `KHV3` custody state and wraps the native multi-wallet catalog in an authenticated `KVI1` inventory envelope before it reaches Kotlin;
- a random 256-bit wrapping key is itself protected by a non-exportable Android Keystore AES key and is zeroized after each Rust call;
- recovery phrase display/export is an explicit backup operation, not part of ordinary signing.

The camera permission exists only for QR import. Network access must not be added to this application.

The software shell mirrors the M5 wallet, backup, recovery, multisig, and XPrv-export workflows while delegating custody and cryptography to the shared Rust runtime. Its home screen deliberately reuses the M5 Connect, Scan QR, Wallet, Settings, and Home artwork in the same 2×2 main-menu pattern. Settings is a software projection: M5-only Display, Audio, Advanced provisioning, device-bound storage, and hardware RTC surfaces are omitted rather than emulated. Hardware-only secure-boot/eFuse, device-secret, and authenticated-RTC guarantees remain physical-M5 properties.
