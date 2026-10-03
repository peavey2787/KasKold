<!-- KasKold — Air-gapped offline signing device for Kaspa -->
<!-- Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me) -->
<!-- License: GPL-3.0-only -->

# Contributing to KasKold

Thanks for helping improve KasKold.

## Security vulnerabilities

**Do not open a public issue for a suspected vulnerability.** Email
`kaskold@proton.me` with subject `[SECURITY]`; see [SECURITY.md](SECURITY.md).

## Development setup

GNU Make is the contributor interface on Linux, Windows, and macOS. Start with:

```bash
make help
make test
```

Use `make firmware`, `make companion`, `make vault-web`, `make android`, `make android-vault`, `make ios`, and `make ios-vault` for focused builds. Use `make qa`, `make android-qa`, and `make ios-qa` for their corresponding validation suites. Platform scripts under `scripts/`, `tools/`, and `qa/` plus direct Python checks remain available for CI/debugging, but they are implementation helpers rather than competing public entry points. iOS work requires macOS with full Xcode plus the repository-pinned Rust/Python/WASM prerequisites exercised by the Make targets.

## Before opening a pull request

1. Make the smallest coherent change; do not weaken QA thresholds to make it pass.
2. Run `make test` while developing. It is intentionally host/browser-only: no Android, iOS/Xcode, physical-device, or HIL tests run there.
3. Run `make qa` before release-oriented changes. It is the authoritative all-non-hardware suite and includes eligible Android/iOS software tests, QEMU, real-node and funded/interactive E2E, coverage/CRAP, benchmarks, fresh mutation certification, and fuzzing. Keep first-party versions pinned at `2.0.0` unless a release explicitly changes them.
4. Firmware changes should pass `make qa` before release-oriented review.
5. Companion changes should build with `make companion`.
6. `make android-qa` and `make ios-qa` remain useful focused mobile commands; the corresponding non-hardware mobile checks are also cataloged under `make qa`, with explicit SKIP results on ineligible hosts.
7. Hardware-sensitive changes should be exercised on the affected board. If you do not have that hardware, say so plainly in the PR; a skipped HIL check is not a pass.
8. Update the smallest relevant user guide or changelog entry when behavior changes.

## Code and security rules

- Firmware remains `no_std` and must not gain a wallet-network path.
- No secret-bearing production logs; zeroize owned transient key material.
- Avoid `unsafe` except where hardware access requires it, and keep its scope documented.
- Monetary/DAA values crossing JavaScript boundaries must use exact integer handling, not lossy `Number` arithmetic.
- Keep wallet-spending keys separate from covenant-only signing domains.
- Do not reintroduce retired password-only secret containers or obsolete transaction/session wire formats as current encoders.
- Keep compiler/lint/CRAP/mutation/coverage/fuzz/architecture gates intact.

## Useful contribution areas

Security review, QR/camera reliability, CoreS3 Lite qualification and other hardware validation,
macOS/iOS validation, transaction/covenant review UX, Companion/mobile testing, and
clear reproducible documentation are especially useful.

## Code of Conduct

### Our Pledge

KasKold is a security-critical project. We are committed to providing a welcoming and respectful environment for everyone, regardless of background.

### Our Standards

- Be respectful and constructive.
- Focus on technical merit.
- Accept constructive criticism gracefully.
- Prioritize security over features.
- Report vulnerabilities responsibly; see [SECURITY.md](SECURITY.md).

### Enforcement

Unacceptable behavior can be reported to `kaskold@proton.me`.

## License

All first-party KasKold code, including `crates/shared-signer`,
`crates/kaskold-protocol`, and `crates/kaskold-sdk`, is licensed
**GPL-3.0-only**, the license of the upstream project KasKold is forked from
([KasSigner, see docs/legal/UPSTREAM_ATTRIBUTION.md](docs/legal/UPSTREAM_ATTRIBUTION.md)).
Third-party code under `external/` keeps its own license.

By contributing, you agree that your contribution is distributed under
GPL-3.0-only.
