# PSKT, parser, address, and covenant hardening

This review covers host protocol parsing, standard PSKT/KSPT relay and merge behavior, address canonicalization, externally sourced UTF-8 presentation paths, and covenant signer/branch binding.

## Security invariants

- Untrusted UTF-8 is never sliced at an arbitrary byte offset. Fixed-format protocol text is validated as ASCII before byte indexing; user/carrier text is split only at `char_indices()` boundaries.
- Kaspa addresses accepted by the protocol are canonical encodings with an exact payload/checksum length. The firmware validator also enforces the exact version-dependent symbol count and zero canonical padding bits.
- A PSKT/KSPT input is not marked complete merely because enough signature entries exist. Standard P2PK and canonical M-of-N multisig signatures must verify as BIP340 signatures over the exact SIGHASH_ALL transaction digest.
- Signature merges are idempotent only for byte-identical signatures. A different signature for the same public key, malformed existing signature map, duplicate KSPT position, signature-capacity overflow, or mismatched transaction body is an error. Offline PSKT merges preflight every input before mutation.
- Covenant signatures are resolved through a shared structural branch/key resolver. The resolver parses pushes and nested IF/ELSE/ENDIF control flow, binds each canonical PUSH32+CHECKSIG/CHECKSIGVERIFY key to its exact branch path, and rejects malformed flow, malformed pushes, excessive nesting, duplicate/ambiguous keys, and out-of-range positions. Generic covenant inputs still are not promoted to complete because KSPT does not carry the final witness selectors/preimages needed to prove the executed branch.

## Covenant branch/binding audit

The registered known-covenant path recomputes the reviewed commitment and checks the complete registered script grammar. `Sha256Preimage` must equal the complete fixed CheckSigFromStack script; `OracleV1` must match its fixed opcode/layout grammar, canonical integer push, commitment, and oracle key.

Two adapter weaknesses were corrected:

1. Covenant witness routing previously treated a malformed/noncanonical owner layout as the IF/owner branch. It now rejects malformed layouts and malformed signer-key encodings.
2. Finalized covenant KSPT encoding previously assigned every non-owner signer to signature position 1 without proving branch membership. Encoding now uses the shared branch-aware resolver to map the signer key to the exact canonical CHECKSIG branch position; missing or ambiguous bindings are rejected.

The opaque `KeyPresent` check was also tightened. It previously accepted a coincidental 32-byte occurrence anywhere in script bytes. It now accepts only an actual 32-byte script push containing the x-only key. This remains intentionally weaker than a registered known-covenant grammar and must not be treated as proof that a particular branch authorizes that key.

`script_binds_fixed_commitment` now requires the complete canonical fixed script rather than accepting the pattern embedded inside an otherwise unrelated script.

## Fuzz coverage

`qa/fuzz/public_host_parsers.rs` feeds arbitrary binary to public byte parsers (QR framing/decoder, pairing, anti-klepto, covenant and private-swap messages) and feeds every valid arbitrary UTF-8 input to the public address, account-key, account, PSKT, completion, finalization, signing-request, and signing-response entry points. Existing dedicated fuzz targets continue to cover compact KSPT, standard PSKT, firmware external inputs, QR framing, account keys, and steganographic picture parsing.

## Validation note

This archive was hardened in an environment without a Rust toolchain (`cargo`, `rustc`, `rustfmt`, and Clippy were unavailable). TOML syntax and static source invariants were checked here, but compilation, unit tests, Clippy, and libFuzzer execution must be run in the normal KasKold development environment before release. Until those gates pass, this work should not be represented as release-validated. The branch/key binding gap identified in the prior pass is now implemented; generic covenant completion intentionally remains fail-closed when execution-branch witness evidence is absent.
