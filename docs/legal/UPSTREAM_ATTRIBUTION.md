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

## Licensing

The repository-level GPL-3.0 license and all original source-file copyright
notices remain intact. Components that were already distributed under
MIT/Apache-2.0 retain those licenses. Third-party and `external/` notices and
licenses remain authoritative for those components.

## Compatibility identifiers

Certain protocol, serialized-format, fixture, or cryptographic domain
identifiers may intentionally retain their historical `KasSigner` spelling
where changing them would alter compatibility or cryptographic semantics. Such
identifiers are compatibility artifacts, not current product branding.

Branding changes must never be used as a reason to change wire bytes or
cryptographic domain-separation strings without a separately reviewed protocol
migration.
