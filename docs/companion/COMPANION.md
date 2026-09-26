[KasKold](../../README.md) › [Documentation](../README.md) › Companion

# KasKold Companion

KasKold Companion is the online **watch-only** application family for Web, Android, and iOS. It never creates or restores a wallet, never owns a recovery phrase/private key, and never signs locally.

## Startup

Companion starts with **kpub management**. Import or select a public account exported by KasKold Hardware or any KasKold Vault app. **Broadcast TX** and **Multisig** remain available from the watch-only startup surface even before a kpub is selected.

## Responsibilities

Companion may derive public addresses from kpubs, query nodes, track balances/UTXOs/history/assets, construct ordinary/multisig/covenant/stealth transactions, prepare KSPT/PSKT signing requests, receive signed responses, finalize transactions, and broadcast them.

Companion may not create/restore wallets, receive recovery phrases as a custody flow, persist private key material, instantiate `hot-wallet`/`vault-runtime`, or sign a transaction locally.

## Signing

When a transaction requires authorization, Companion displays/transports the signing request to **KasKold Hardware** or **KasKold Vault Web/Android/iOS**. The signer independently reviews and signs it. Companion then accepts the signed response and continues finalization/broadcast.

This external signing exchange is a watch-only workflow, not a second Companion custody mode.

See [Watch-only model](WATCH_ONLY_MODEL.md), [Companion Web](WEB.md), [Android](ANDROID.md), [iOS](IOS.md), and [Vault](../getting-started/VAULT.md).
