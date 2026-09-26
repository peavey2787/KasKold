# KasKold Vault for iOS

iOS Vault is a standalone offline signer with bundle identity `com.kaskold.vault`.

It links the Rust `vault-runtime` static library through a narrow C ABI. Swift owns only UI, camera/QR presentation, protected ciphertext storage, and platform key protection. The wallet seed remains inside Rust except for the explicit recovery-phrase create/restore interface.

Native persistence uses the authenticated `KVI1` inventory envelope so wallet names, slot order, and the active wallet are integrity-protected across restarts. Each slot remains independently sealed by Rust as typed `KHV3` custody state (mnemonic, account-XPrv, or raw key); `KHV2` mnemonic containers and legacy `KHV1` seed-only containers remain readable as one-wallet migration inputs. The inventory ciphertext is stored with complete file protection. Its random 256-bit wrapping key is held in Keychain with `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`. The app contains no URL session, WebSocket, node resolver, blockchain RPC, broadcast, or embedded web-view networking surface.

The software shell mirrors the M5 wallet, backup, recovery, multisig, and XPrv-export workflows while delegating custody and cryptography to the shared Rust runtime. Its home screen deliberately reuses the M5 Connect, Scan QR, Wallet, Settings, and Home artwork in the same 2×2 main-menu pattern. Settings is a software projection: M5-only Display, Audio, Advanced provisioning, device-bound storage, and hardware RTC surfaces are omitted rather than emulated. Hardware-only secure-boot/eFuse, device-secret, and authenticated-RTC guarantees remain physical-M5 properties.
