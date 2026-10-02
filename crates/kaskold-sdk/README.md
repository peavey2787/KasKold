[KasKold](../../README.md) › [Documentation](../../docs/README.md) › [Wallet Integration](../../docs/integration/WALLET_INTEGRATION.md) › SDK

# KasKold SDK — Rust/WASM

`kaskold-sdk` is the friendly, network-free facade a wallet integrates to support KasKold hardware. It depends on `kaskold-protocol`; it is **not** a wallet implementation.

KasWare, Kaspium, and other wallets retain their existing UTXO discovery, automatic/manual coin selection, fees, outputs, change, privacy/consolidation policy, providers, and broadcast logic.

## Native Rust and stable errors

The default build is a genuinely native Rust SDK: WASM dependencies are target/feature-gated. Public operations return `SdkResult<T>` with a stable `SdkErrorKind` (for example `WrongNetwork`, `TransactionMismatch`, `PairingMismatch`, `PairingReplay`, `Qr`, and `Finalization`) so wallet code never needs to match error strings. Public extensible enums/types are `#[non_exhaustive]`; include a wildcard arm when matching them.

From a source checkout, keep the exact version and workspace path together:

```toml
[dependencies]
kaskold-sdk = { version = "=2.0.0", path = "../kaskold-sdk" }
```

After `2.0.0` is published to crates.io, an external consumer can use:

```toml
[dependencies]
kaskold-sdk = "=2.0.0"
```

```rust
use kaskold_sdk::{complete, finalize, prepare, Network};

// Host wallet constructs this PSKT using its own exact inputs and policy.
let request = prepare(&pskt_hex, Network::Mainnet)?;
for frame in request.qr_frames() {
    wallet_ui.render_qr(&frame.payload)?;
}

let signed = complete(&request, &scanned_response_hex)?;
let consensus_json = finalize(&signed)?;
wallet.broadcast(consensus_json)?;
```

There is intentionally no `prepareSend`, generic `createTransaction`, `sendTx`, or `broadcast` operation in the SDK.

`KasKold` provides an instance-owned QR decoder plus descriptor/privacy pairing state. Successful privacy responses are bound to the pending nonce/ranges and stable account fingerprint, then consume the pending request.

## WebAssembly

WASM is an explicit build feature rather than an unconditional native dependency. The platform wrappers compile with `--no-default-features --features wasm`. From the repository root run `make sdk`; the crate-local `./build.sh` and `build.ps1` remain equivalent lower-level entrypoints. The standalone generated package is written to `target/sdk/kaskold-sdk/pkg` as `@kaskold/sdk`; its JavaScript is generated `wasm-bindgen` glue around the Rust `.wasm`.

```js
import init, { KasKold } from "@kaskold/sdk";

await init();
const signer = new KasKold();
const request = JSON.parse(signer.prepare(psktHex, "mainnet"));
// Render each request.qrFrames[i].payload with your wallet's QR library.
const signed = JSON.parse(signer.complete(JSON.stringify(request), scannedResponseHex));
const consensusTx = signer.finalize(JSON.stringify(signed));
wallet.broadcast(consensusTx);
```

See `docs/integration/WALLET_INTEGRATION.md` for pairing, derivation metadata, publishing order, and conformance vectors.

## License

`kaskold-sdk`, `kaskold-protocol`, and `shared-signer` are licensed **GPL-3.0-only**, like all of KasKold, which is a fork of [KasSigner](https://github.com/InKasWeRust/KasSigner) (GPL-3.0). Wallets that integrate the SDK are distributed under the GPL's terms.

## Signer capabilities

Wallets should call `kaskold_sdk::limits()` (or `KasKold::limits()` in the WASM facade) before constructing requests. The returned `SignerCapabilities` describes the reference signer's current KSPT generation, 32-input transaction ceiling, output/script/redeem/payload bounds, multisig limits, and QR framing capacity. Treat these as hardware compatibility limits rather than wallet coin-selection policy; future signer generations may advertise different values.
