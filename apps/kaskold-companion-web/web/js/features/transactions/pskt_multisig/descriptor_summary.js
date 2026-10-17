import { multisig_descriptor_summary } from '../../../wasm/api.js';

// Kind, threshold and participant count of a descriptor KasKold can
// co-sign, parsed by Kaspa Portal; null when it is not one.
export function descriptorSummary(text) {
    try {
        return JSON.parse(multisig_descriptor_summary(String(text || '').trim()));
    } catch (_) {
        return null;
    }
}
