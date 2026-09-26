[KasKold](../../README.md) › [Documentation](../README.md) › [Companion](COMPANION.md) › Watch-only model

# Companion Watch-Only Model

All three Companion applications have one custody model: **Watch Only - no private keys**.

- Public input: kpubs, addresses, descriptors, network state, unsigned/signing-request data, signed responses.
- Allowed online actions: balances/history, transaction construction, multisig/multi-spend, covenants, signing-request transport, finalization, broadcast.
- Forbidden custody actions: wallet creation, mnemonic restore, private-key storage, seed derivation, local signing, software signer unlock.

The repository enforces this structurally: Companion Web cannot depend on `hot-wallet`, `vault-runtime`, or `offline-signer`; Android/iOS Companion shells cannot contain native persistent software-wallet stores. Signing code belongs to Vault/Hardware.
