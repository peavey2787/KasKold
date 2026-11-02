import { walletSession, decode_address, parse_kpub } from '../boundary.js';
// KasKold Companion Web — features/covenants/generation/panel_keys
import { showCovenantPanel } from './panels/router.js';
import { byId } from '../../../core/dom.js';


// Covenant type -> the dedicated field panel it reveals. Types not listed use the simple panel.
const PANEL_BY_TYPE = new Map([
    ['additive', 'cov-fields-piggy'],
    ['escrow', 'cov-fields-escrow'],
    ['ship-escrow', 'cov-fields-ship-escrow'],
    ['timelocked-savings', 'cov-fields-savings'],
    ['dms', 'cov-fields-dms'],
    ['global-spending-limit', 'cov-fields-splimit'],
    ['global-allowance', 'cov-fields-allowance'],
    ['payjoin', 'cov-fields-payjoin'],
    ['commit-reveal', 'cov-fields-commit-reveal'],
    ['merkle-whitelist', 'cov-fields-merkle-whitelist'],
    ['oracle-v1', 'cov-fields-oracle-v1'],
    ['crowdfund', 'cov-fields-crowdfund'],
]);

export function covShowPanel(panel) {
    showCovenantPanel(panel);
}

// Panels that were guarded by a null check before the refactor; all others throw when missing.
const OPTIONAL_PANEL_IDS = new Set([
    'cov-fields-ship-escrow', 'cov-fields-savings', 'cov-fields-dms',
    'cov-fields-splimit', 'cov-fields-oracle-v1', 'cov-fields-crowdfund',
]);

function setPanelVisible(id, visible) {
    const panel = byId(id);
    if (!panel && OPTIONAL_PANEL_IDS.has(id)) return;
    panel.classList.toggle('hidden', !visible);
}

export function covTypeChanged() {
    const selected = byId('cov-type').value;
    setPanelVisible('cov-fields-simple', !PANEL_BY_TYPE.has(selected));
    for (const [type, panelId] of PANEL_BY_TYPE) {
        setPanelVisible(panelId, type === selected);
    }
}
// key plus EVERY receive and change address payload — a counterparty may
// have shared any index from their device, not just /0/0. Matching only
// /0/0 made escrow role detection fail whenever the shared address was at
// a browsed index (arbiter got seller tabs).
export function walletMatchesPk(target) {
    if (!target || !walletSession.hasWallet()) return false;
    const acct = getAccountPubkeyHex();
    if (acct && acct === target) return true;
    try {
        const w = walletSession.current();
        const all = [...(w.receive_addresses || []), ...(w.change_addresses || [])];
        for (const a of all) {
            try {
                const d = JSON.parse(decode_address(a));
                if (d.payload && d.payload === target) return true;
            } catch { /* An address that will not decode cannot match. */ }
        }
    } catch { /* With no wallet open nothing can match. */ }
    return false;
}
export function getOwnerPubkeyHex() {
    if (!walletSession.hasWallet()) return null;
    const w = walletSession.current();
    const addr0 = w.receive_addresses[0];
    const decoded = JSON.parse(decode_address(addr0));
    return decoded.payload || null;
}
export function getAccountPubkeyHex() {
    if (!walletSession.hasWallet()) return null;
    const w = walletSession.current();
    if (!w.kpub) return null;
    const kpubInfo = JSON.parse(parse_kpub(w.kpub));
    return kpubInfo.account_pubkey || null;
}
