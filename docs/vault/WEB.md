[KasKold](../../README.md) › [Documentation](../README.md) › Vault › Web

# KasKold Vault Web

Vault Web is the browser software signer. It is separate from Companion Web.

It can create 12/24-word wallets, restore a BIP39 recovery phrase, revisit recovery words through the explicit Wallet → Backup flow, export the public kpub and its QR, produce standard/compact/plain-text SeedQR backups plus XPrv/private-key advanced exports, scan KSPT signing requests, show an independent transaction review, approve/reject, and display the signed response as QR frames.

The Web Vault home screen reuses the M5 Connect, Scan QR, Wallet, Settings, and Home artwork in the same 2×2 main-menu pattern. Settings intentionally exposes only software-applicable controls; M5-only Display, Audio, Advanced provisioning, device-bound storage, and hardware RTC entries are omitted rather than imitated.

Vault Web depends on the shared Rust `vault-runtime` and does **not** depend on `online-watcher` or `kaskold-sdk`. Its authored UI contains no node resolver, balance lookup, blockchain REST/RPC/WebSocket client, or transaction broadcast path.

Vault Web does not silently persist private custody across page unload. Treat the recovery phrase as the durable recovery mechanism and lock/close the Vault when finished.

## Local browser use

Build the signer runtime before serving it:

```bash
make vault-web
```

Serve the canonical generated site at `target/kaskold-vault-web/site/` (or the authored `apps/kaskold-vault-web/web/` tree after that build has mirrored `pkg/`). Vault Web uses product-specific `vault_*` asset URLs so it cannot accidentally reuse cached Companion JavaScript/CSS when developers switch between the two apps on localhost. If the generated `pkg/vault_web.js` runtime is missing, the page reports that build error instead of remaining indefinitely on “Loading the Vault runtime…”.
