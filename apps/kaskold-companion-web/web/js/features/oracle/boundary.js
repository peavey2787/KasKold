// @ts-check
// The Rust boundary and application state the oracle domain uses.
export {
    covenant_oracle_mb,
    create_covenant_oracle_v1_claim,
    create_covenant_pskb_with_payload,
    create_oracle_mb_publish,
    fetch_utxos_for_address_js,
    pskb_append_output,
    verify_covenant_anti_klepto,
    verify_oracle_v1_attestation,
} from '../../wasm/api.js';
export { covenantState, oracleState, walletSession } from '../../app/state/index.js';
