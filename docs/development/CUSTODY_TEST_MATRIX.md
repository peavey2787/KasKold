[KasKold](../../README.md) › [Documentation](../README.md) › Development › Custody Test Matrix

# Custody Test Matrix

KasKold deliberately separates watch-only networking from signing custody.

| Online client | Signer | Automated expectation | Release evidence |
|---|---|---|---|
| Companion Web | CoreS3 Hardware | watch-only request/finalize path | CoreS3 hardware-qualified; physical cross-device release smoke required |
| Companion Web | CoreS3 Lite Hardware | watch-only request/finalize path | qualification pending; physical cross-device release smoke required |
| Companion Web | Vault Web | browser signer/runtime contracts | browser release smoke required |
| Companion Web | Vault Android | QR signing interoperability | signed app/device smoke required |
| Companion Web | Vault iOS | QR signing interoperability | signed app/device smoke required |
| Companion Android | CoreS3 / Vault | watch-only request/finalize path | signed app/device smoke required |
| Companion iOS | CoreS3 / Vault | watch-only request/finalize path | signed app/device smoke required |

## Architecture contracts

- Companion Web/Android/iOS never instantiate software custody and cannot sign locally.
- Companion Web does not depend on `hot-wallet`, `vault-runtime`, or `offline-signer`.
- Vault Web/Android/iOS own software custody through the shared Rust `vault-runtime`/`hot-wallet` backend.
- Vault review is a separate authorization step before signing.
- Hardware signer cannot network; native Vault apps remain network-isolated.
- CoreS3 Lite remains build-supported with hardware qualification pending until physical evidence exists.
