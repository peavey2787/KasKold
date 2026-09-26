[KasKold](../../README.md) › [Documentation](../README.md) › [Companion](COMPANION.md) › Web

# Companion Web

Companion Web hosts the Rust/WASM `online-watcher` runtime and is strictly watch-only. It starts with kpub management, Broadcast TX, and Multisig. It contains no wallet-create/restore controls and no local signing backend.

For authorization, prepare the signing request in Companion and transfer it to KasKold Hardware or Vault. Import the signed response back into Companion for finalization/broadcast.

Browser storage is limited to non-secret watch-only state. No mnemonic, seed, xprv, or private key belongs in Companion Web storage.
