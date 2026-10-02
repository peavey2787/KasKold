# Upstream attribution

KasKold 2.0.0 is an independently maintained open-source Kaspa wallet and
signing platform derived from the hardened Rust architecture of **KasSigner**.
KasSigner was originally developed by the KasSigner Project / InKasWeRust.
KasKold is independently maintained and is not affiliated with or endorsed by
the original KasSigner maintainers.

The original 2025 KasKold implementation was an earlier air-gapped Kaspa wallet
proof of concept. It is preserved in Git history on the `legacy/kaskold-poc`
branch and `kaskold-poc-2025` tag, but its JavaScript wallet/security
implementation is not used as the security foundation of current KasKold.

## Upstream project

- Project: KasSigner — offline signer, seed manager and stego backup for Kaspa
- Source: <https://github.com/InKasWeRust/KasSigner>
- Copyright: KasSigner Project (kassigner@proton.me), maintained by InKasWeRust
- License: GNU General Public License v3.0 (GPL-3.0-only)

KasKold is a modified version of KasSigner. As GPL-3.0 section 5 requires, the
modifications are released under the same license, every source file keeps
its original copyright notice, and the changes are recorded in this
repository's Git history and `CHANGELOG.md`. The signing core that KasKold now
imports from Kaspa Portal was contributed from this codebase and is likewise
GPL-3.0.

## Licensing

All first-party KasKold code is **GPL-3.0-only** (see the root `LICENSE`).
This includes `shared-signer`, `kaskold-protocol`, and `kaskold-sdk`, which an
earlier revision labelled MIT/Apache-2.0; because they contain KasSigner-derived
GPL code, GPL-3.0-only is the license that applies. Third-party code under
`external/` keeps its own notices and licenses.

## Compatibility identifiers

Certain protocol, serialized-format, fixture, or cryptographic domain
identifiers may intentionally retain their historical `KasSigner` spelling
where changing them would alter compatibility or cryptographic semantics. Such
identifiers are compatibility artifacts, not current product branding.

Branding changes must never be used as a reason to change wire bytes or
cryptographic domain-separation strings without a separately reviewed protocol
migration.
