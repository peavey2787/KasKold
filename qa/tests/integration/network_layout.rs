use crate::common::workspace_root;

#[test]
fn rpc_subsystem_is_grouped_by_responsibility() {
    let root = workspace_root();
    let online = root.join("crates/online-watcher/src");
    let network = online.join("network");

    assert!(!network.join("rpc.rs").exists());
    // The wRPC codec, transport and submission encoder are Kaspa Portal's; the
    // Companion keeps only its URL-addressed query and submission helpers.
    for required in ["mod.rs", "queries/utxos.rs", "submission.rs"] {
        assert!(
            network.join(required).exists(),
            "missing network/{required}"
        );
    }
    for retired in ["codec", "wrpc", "model", "error.rs", "submission"] {
        assert!(
            !network.join(retired).exists(),
            "network/{retired} duplicates Kaspa Portal"
        );
    }
    assert!(
        !online.join("infrastructure/browser_websocket.rs").exists(),
        "the browser WebSocket transport comes from Kaspa Portal"
    );
    let network_mod = std::fs::read_to_string(network.join("mod.rs")).expect("network/mod.rs");
    assert!(network_mod.contains("kaspa_portal::network"));

    for required in [
        "wasm_api/contracts/vault/genesis.rs",
        "wasm_api/contracts/vault/spend.rs",
        "wasm_api/contracts/vault/split.rs",
        "wasm_api/contracts/vault/tagged.rs",
        "contracts/seq_commit/proof.rs",
    ] {
        assert!(online.join(required).exists(), "missing {required}");
    }
    // Signed-KSPT authorization and sighash are Kaspa Portal's.
    for retired in [
        "protocol/transaction/signed_kspt.rs",
        "protocol/transaction/sighash.rs",
    ] {
        assert!(
            !online.join(retired).exists(),
            "{retired} duplicates Kaspa Portal"
        );
    }
    assert!(
        !online.join("contracts/vault/transactions.rs").exists(),
        "retired browser-signing vault transaction layer must not return",
    );
}
