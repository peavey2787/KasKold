import { oracleState } from '../../../../../app/state/index.js';
import { pskb_append_output } from '../../../../../wasm/api.js';
import { byId } from '../../../../../core/dom.js';
import { kasToSompi, sompiToKasString } from '../../../../../core/amounts.js';

export function spliceOracleServiceFee(wireHex, protocol) {
    return pskb_append_output(wireHex, BigInt(protocol.feeSompi), protocol.feeSpk);
}

export function setOracleFee(totalKas, fromCustom) {
    let sompi;
    try { sompi = kasToSompi(String(totalKas).trim()); } catch (_) { return; }
    if (sompi < 100000000n) return;
    const value = sompiToKasString(sompi);
    oracleState._oracleMbFeeTotalKas = value;
    document.querySelectorAll('.omb-fee-btn').forEach((button) => {
        const selected = !fromCustom && button.getAttribute('data-omb-fee') === value;
        button.style.background = selected ? 'var(--teal)' : 'var(--bg)';
        button.style.color = selected ? '#0a0a0a' : 'var(--text)';
        button.style.borderColor = selected ? 'var(--teal)' : 'var(--border)';
    });
    const button = byId('btn-oracle-mb-ask');
    if (button) button.textContent = `Ask for new price (≈${value} KAS)`;
}
