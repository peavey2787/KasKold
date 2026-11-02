// @ts-check
import { decode_address, is_kaspa_address } from '../wasm/api.js';

/**
 * @param {string | null | undefined} value
 */
export function addressToXOnly(value) {
    const normalized = (value || '').trim();
    if (!is_kaspa_address(normalized)) {
        return normalized;
    }
    try {
        const decoded = JSON.parse(decode_address(normalized));
        return decoded.payload && decoded.payload.length === 64 ? decoded.payload : '';
    } catch {
        return '';
    }
}
