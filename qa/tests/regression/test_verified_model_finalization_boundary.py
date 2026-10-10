
import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source, module_text, portal_root, portal_text  # noqa: E402
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[3]


def text(path: str) -> str:
    return module_text(str(path))


def schema_text() -> str:
    """The canonical PSKT schema, which kaskold-protocol re-exports from Kaspa Portal."""
    return "\n".join(
        portal_text(f"transaction/interchange/pskt/schema/{name}")
        for name in ("mod.rs", "tables.rs", "json.rs", "numbers.rs")
    )


def fn_body(source: str, name: str) -> str:
    match = re.search(rf"\bfn\s+{re.escape(name)}\b[^{{]*\{{", source)
    if not match:
        raise AssertionError(f"function {name} not found")
    start = match.end() - 1
    depth = 0
    for i in range(start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[start : i + 1]
    raise AssertionError(f"function {name} is unterminated")


class VerifiedModelFinalizationBoundaryTests(unittest.TestCase):
    def test_recursive_duplicate_keys_have_one_authoritative_grammar(self):
        schema = schema_text()
        host = text("crates/kaskold-protocol/src/pskt/wire.rs")
        companion = text("crates/online-watcher/src/protocol/pskt/wire/json.rs")
        vault = text("crates/offline-signer/src/transaction/std_pskt/parser/mod.rs")
        tests = text("crates/kaskold-protocol/src/unit_tests/pskt_schema.rs")

        self.assertIn("pub fn validate_canonical_json", schema)
        self.assertIn("key_seen_before", schema)
        self.assertIn("JsonSyntaxError::DuplicateKey", schema)
        parse = fn_body(host, "parse_strict_json")
        self.assertLess(parse.index("validate_canonical_json"), parse.index("serde_json::from_slice"))
        self.assertNotIn("StrictVisitor", host)
        self.assertIn("compat::decode_pskt_json_body", companion)
        self.assertIn("validate_canonical_json(json)", vault)
        self.assertIn("canonical_json_rejects_duplicate_keys_recursively_in_every_container_shape", tests)
        for fixture in [
            r'{"x":1,"x":2}',
            r'{"outer":{"x":1,"x":2}}',
            r'{"outer":{"deeper":{"x":1,"x":2}}}',
            r'{"items":[{"x":1,"x":2}]}',
            r'[{"outer":{"x":1,"x":2}}]',
            r'{"unknownExtension":{"array":[{"future":1,"future":2}]}}',
        ]:
            self.assertIn(fixture, tests)

    def test_verified_transaction_is_an_opaque_authorization_token(self):
        verified = text("crates/kaskold-protocol/src/pskt/verified.rs")
        for field in [
            "version", "locktime", "subnetwork_id", "gas", "payload", "network", "inputs", "outputs"
        ]:
            self.assertNotRegex(verified, rf"pub\s+{field}\s*:")
        for field in [
            "previous_tx_id", "previous_index", "amount", "sequence", "sig_op_count",
            "script_version", "script_public_key", "has_covenant_id", "witness",
        ]:
            self.assertNotRegex(verified, rf"pub\s+{field}\s*:")
        self.assertIn("pub fn inputs(&self)", verified)
        self.assertIn("pub const fn witness(&self)", verified)

    def test_standard_pskt_finalizer_never_reparses_after_authorization(self):
        source = text("crates/online-watcher/src/facade.rs")
        body = fn_body(source, "finalize_and_broadcast")
        self.assertIn("verify_complete_pskt", body)
        self.assertIn(".to_consensus()", body)
        for forbidden in ["decode_root", "serde_json", "from_str", "from_slice", "decode_wire", "build_consensus_input"]:
            self.assertNotIn(forbidden, body)

        protocol = text("crates/kaskold-protocol/src/pskt/mod.rs")
        # The unverified structural finalizer is retired, not kept for tests.
        self.assertNotIn("mod finalize;", protocol)
        self.assertFalse((ROOT / "crates/kaskold-protocol/src/pskt/finalize.rs").exists())
        finalize = fn_body(protocol, "finalize_json")
        self.assertIn("verify_for_broadcast(pskt_hex, limits)", finalize)
        self.assertIn("verified_finalize::finalize_json", finalize)
        self.assertIn("verify_complete_pskt(", fn_body(protocol, "verify_for_broadcast"))
        verify = fn_body(protocol, "verify_complete_pskt")
        self.assertIn("verify_complete_transaction", verify)
        self.assertIn("verified::from_compact", verify)
        self.assertNotIn("wire::decode", finalize)

    def test_signed_kspt_finalizer_never_reparses_after_authorization(self):
        source = text("crates/online-watcher/src/facade.rs")
        body = fn_body(source, "broadcast")
        self.assertIn("verify_complete_kspt", body)
        self.assertIn(".to_consensus()", body)
        self.assertNotIn("kspt::decode", body)
        self.assertNotIn("DecodeSink", body)

        protocol = text("crates/kaskold-protocol/src/pskt/mod.rs")
        verify = fn_body(protocol, "verify_complete_kspt")
        self.assertEqual(verify.count("compact::parse"), 1)
        self.assertIn("compact::verified_complete", verify)
        self.assertIn("verified::from_compact", verify)
        self.assertIn("Raw covenant KSPT broadcast is disabled", verify)
        self.assertIn("parse_multisig_redeem", verify)

    def test_covenant_execution_cannot_be_silently_dropped_by_a_kspt_sink(self):
        decode = text("crates/kaskold-protocol/src/wire/kspt/decode.rs")
        trait = re.search(r"pub trait DecodeSink.*?\n}\n", decode, flags=re.S)
        self.assertIsNotNone(trait)
        covenant = re.search(r"fn covenant_execution\([^;{}]*\)[^;{}]*(;|\{)", trait.group(0), flags=re.S)
        self.assertIsNotNone(covenant)
        self.assertEqual(covenant.group(1), ";", "security-critical callback must not have a no-op default")

        parser = text("crates/kaskold-protocol/src/pskt/compact/parser.rs")
        self.assertIn("covenant_execution", parser)
        verified = text("crates/kaskold-protocol/src/pskt/verified.rs")
        self.assertIn("compact::covenant_path(index, input)", verified)
        self.assertIn("supplied_true_mask", verified)
        self.assertIn("materialize_signature_script", verified)

    def test_specialized_covenant_routing_is_typed_template_bound_and_fail_closed(self):
        relay = text("crates/kaskold-protocol/src/pskt/relay.rs")
        verify = fn_body(relay, "verify_complete_transaction")
        build = fn_body(relay, "build_verified_transaction")
        self.assertIn("bind_specialized_witnesses", build)
        self.assertLess(verify.index("build_verified_transaction"), verify.index("verified_complete"))

        specialized = text("crates/kaskold-protocol/src/pskt/specialized.rs")
        for binder in [
            "bind_private_swap", "bind_oracle_v1",
            "bind_commit_reveal", "bind_merkle",
        ]:
            self.assertIn(binder, specialized)
        self.assertIn("covenant_execution", specialized)
        self.assertIn("require_only_route_fields", specialized)
        self.assertIn("no typed verified witness plan", specialized)

        verified = text("crates/kaskold-protocol/src/pskt/verified.rs")
        self.assertIn("SpecializedCovenant", verified)
        self.assertNotIn("minimumSignatures", verified)

        schema = schema_text()
        self.assertIn("SUPPORTED_SPECIALIZED_COVENANT_ROUTING_FIELDS", schema)
        for marker in [
            "escrowBranch", "privateSwapClaim", "oracleV1Claim", "risc0OracleMb",
            "zkProof", "merkleProof", "rollupStateAdvance", "depositHoldingRefund",
        ]:
            self.assertIn(f'"{marker}"', schema)

        e2e = text("crates/kaskold-protocol/src/unit_tests/finalization.rs")
        self.assertIn("supported_specialized_covenants_are_template_bound_and_mutation_hardened_end_to_end", e2e)
        self.assertIn("unsupported_and_zero_signature_covenant_routes_are_disabled_at_public_finalization", e2e)

    def test_covenant_id_presence_is_shared_schema_not_host_only_extension(self):
        schema = schema_text()
        vault = text("crates/offline-signer/src/transaction/std_pskt/parser/inputs/details.rs")
        host = text("crates/kaskold-protocol/src/pskt/relay_fields.rs")
        self.assertRegex(
            schema,
            r'field\(\s*"covenantId",\s*false,\s*NullRule::AllowedAsDefault,\s*'
            r'ValueKind::HexString,\s*DefaultRule::None,?\s*\)',
        )
        self.assertIn('b"covenantId" => self.parse_covenant_id', vault)
        self.assertIn('utxo.get("covenantId")', host)

    def test_vault_signer_and_generic_broadcast_have_explicit_distinct_topology_contracts(self):
        signer = text("crates/offline-signer/src/transaction/kspt/signing/covenant.rs")
        self.assertIn("supplied_mask != resolved.selector_mask()", signer)
        self.assertIn("candidates.len != 1", signer)
        self.assertNotIn("resolved.selector_mask() != 0b1", signer)
        tests = text("crates/offline-signer/src/transaction/kspt/signing/unit_tests/mod.rs")
        self.assertIn("richer_topology.inputs[0].covenant_execution_mask = 3", tests)
        self.assertIn("exactly one active signer may be signed", tests)

        compact = text("crates/kaskold-protocol/src/pskt/compact.rs")
        complete = fn_body(compact, "verified_complete")
        self.assertIn("required_signature_count(index, input)", complete)
        generic = fn_body(compact, "generic_covenant_required")
        self.assertIn("input.covenant_execution else", generic)
        self.assertIn("trace_witness(&input.redeem, mask, truth)", generic)
        self.assertIn("path_positions(&path).len()", generic)
        verified = text("crates/kaskold-protocol/src/pskt/verified.rs")
        self.assertIn("materialize_covenant(witness, redeem_script)", verified)

    def test_completion_uses_executable_shared_schema_and_zero_signature_routes_are_disabled(self):
        relay = text("crates/kaskold-protocol/src/pskt/relay.rs")
        build = fn_body(relay, "build_verified_transaction")
        self.assertIn("schema_validate::validate_document", build)
        self.assertLess(
            build.index("schema_validate::validate_document"),
            build.index("build_base_transaction"),
        )
        for name in ["verified_signature_counts", "is_complete", "verify_complete_transaction"]:
            body = fn_body(relay, name)
            self.assertIn("build_verified_transaction(pskt_hex, network, limits)", body)
        validator = text("crates/kaskold-protocol/src/pskt/schema_validate.rs")
        self.assertIn("pskt_schema::fields", validator)
        self.assertIn("pskt_schema::default_rule", validator)
        self.assertIn("count == 0", validator)
        self.assertIn("SIGHASH_ALL", validator)

    def test_parser_differential_is_symmetric_and_checks_verified_materialization(self):
        fuzz = text("qa/fuzz/pskt_parser_differential.rs")
        self.assertIn("assert_eq!(", fuzz)
        self.assertIn("host_result.is_ok()", fuzz)
        self.assertIn("vault_ok", fuzz)
        self.assertIn("assert_same_semantic_model", fuzz)
        self.assertIn("assert_verified_materialization_matches", fuzz)
        self.assertIn("materialize_signature_script", fuzz)

    def test_zero_signature_public_consumer_routes_are_explicitly_disabled(self):
        routes = {
            "crates/online-watcher/src/wasm_api/contracts/oracle/publish.rs":
                "zero-signature branch has a typed verified witness plan",
            "crates/online-watcher/src/wasm_api/contracts/covenant/families/escrow/shipping/deposit.rs":
                "zero-signature covenant branch has a typed verified witness plan",
            "crates/online-watcher/src/wasm_api/contracts/covenant/families/escrow/shipping/withdraw.rs":
                "zero-signature covenant branch has a typed verified witness plan",
            "crates/online-watcher/src/wasm_api/contracts/covenant/families/escrow/timelocked.rs":
                "zero-signature covenant branch has a typed verified witness plan",
        }
        for path, reason in routes.items():
            source = text(path)
            self.assertIn(reason, source, path)
            self.assertIn("Err(wasm_error!", source, path)

    def test_new_security_boundary_is_in_mutation_scope(self):
        profile = text(".cargo/mutants.toml")
        # The verify-once pipeline and KSPT grammar live in Kaspa Portal, whose
        # mutation gate requires every viable mutant to be caught.
        portal_profile = (portal_root().parent / ".cargo/mutants.toml").read_text()
        self.assertIn('"src/transaction/interchange/pskt/**/*.rs"', portal_profile)
        self.assertIn('"src/transaction/interchange/kspt/**/*.rs"', portal_profile)
        self.assertNotIn('"crates/kaskold-protocol/src/pskt/**/*.rs"', profile)
        self.assertNotIn('"crates/kaskold-protocol/src/wire/pskt_schema.rs"', profile)
        self.assertNotIn('"crates/kaskold-protocol/src/wire/kspt/**/*.rs"', profile)
        self.assertIn('"crates/online-watcher/src/protocol/**/*.rs"', profile)


if __name__ == "__main__":
    unittest.main()
