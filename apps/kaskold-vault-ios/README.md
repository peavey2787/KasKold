[KasKold](../../README.md) › [Documentation](../../docs/README.md) › KasKold Vault

# KasKold Vault for iOS

KasKold Vault is a separate air-gapped signing application, not an offline mode of Companion.

Security boundaries in this source tree:

- there is no `URLSession`, WebSocket, node resolver, blockchain RPC, telemetry, or broadcast client;
- wallet creation, derivation, validation, and signing belong to the shared Rust `vault-runtime` and hardened signer core;
- Rust seals each wallet as authenticated `KHV3` custody state and wraps the native multi-wallet catalog in an authenticated `KVI1` inventory envelope before persistence;
- a random 256-bit platform wrapping key is protected with `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` and must be zeroized after each Rust call;
- persisted ciphertext uses complete iOS file protection;
- camera access is intended only for QR request/response transport.

The Swift UI is deliberately thin. It must not gain a second BIP39/BIP32/Schnorr/KSPT implementation.

The software shell mirrors the M5 wallet, backup, recovery, multisig, and XPrv-export workflows while delegating custody and cryptography to the shared Rust runtime. Its home screen deliberately reuses the M5 Connect, Scan QR, Wallet, Settings, and Home artwork in the same 2×2 main-menu pattern. Settings is a software projection: M5-only Display, Audio, Advanced provisioning, device-bound storage, and hardware RTC surfaces are omitted rather than emulated. Hardware-only secure-boot/eFuse, device-secret, and authenticated-RTC guarantees remain physical-M5 properties.
