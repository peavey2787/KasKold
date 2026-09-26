# KasKold Vault for Android

Android Vault is a standalone offline signer with package identity `com.kaskold.vault`.

Security properties enforced by source/QA include:

- no `android.permission.INTERNET`;
- no HTTP/WebSocket/node/RPC dependency;
- local camera access only for QR signing traffic;
- wallet creation/restoration/signing in the Rust `vault-runtime`/`hot-wallet` boundary;
- persistence writes only the authenticated Rust `KVI1` multi-wallet inventory envelope; each slot is independently sealed as typed `KHV3` custody state, while `KHV2` and legacy `KHV1` ciphertext remain readable as one-wallet migration inputs;
- a random 256-bit platform wrapping key is protected by a non-exportable Android Keystore AES-GCM key;
- explicit scan → transaction review → approve/reject → signed-response state transitions.

The Android build compiles `vault-runtime` as a Rust static library and links it through a small NDK JNI shim. The JNI surface has no seed/private-key accessor.

The software shell mirrors the M5 wallet, backup, recovery, multisig, and XPrv-export workflows while delegating custody and cryptography to the shared Rust runtime. Its home screen deliberately reuses the M5 Connect, Scan QR, Wallet, Settings, and Home artwork in the same 2×2 main-menu pattern. Settings is a software projection: M5-only Display, Audio, Advanced provisioning, device-bound storage, and hardware RTC surfaces are omitted rather than emulated. Hardware-only secure-boot/eFuse, device-secret, and authenticated-RTC guarantees remain physical-M5 properties.
