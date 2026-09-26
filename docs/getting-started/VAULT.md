[KasKold](../../README.md) › [Documentation](../README.md) › Getting Started › Vault

# Getting Started - Vault

KasKold Vault is the software signer family. Vault apps are the software surfaces that may create wallets, restore recovery phrases, hold private custody state, review signing requests, and sign.

Available applications:

- **Vault Web** - browser signer with no blockchain watcher/node/broadcast capability. Its custody state is session-scoped; back up the recovery phrase and restore when needed.
- **Vault Android** - dedicated network-isolated native signer app.
- **Vault iOS** - dedicated network-isolated native signer app.

Typical workflow: create/restore in Vault, export the kpub to Companion, let Companion construct the transaction online, scan/import the signing request into Vault, review/approve there, then return the signed response to Companion.

See [Vault Web](../vault/WEB.md), [Android Vault](../vault/ANDROID.md), [iOS Vault](../vault/IOS.md), and [air-gap model](../vault/AIR_GAP_MODEL.md).
