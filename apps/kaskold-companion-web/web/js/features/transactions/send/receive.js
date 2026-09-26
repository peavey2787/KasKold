import { networkState, walletSession } from '../../../app/state/index.js';
import { showScreen } from '../../../app/navigation.js';
import { getNextReceiveIndex } from '../../wallet/core.js';
import { extend_addresses, generate_qr_frames } from '../../../wasm/api.js';
// KasKold Companion Web — features/transactions/send/receive
import { utf8ToHex } from '../../../core/bytes.js';
import { byId } from '../../../core/dom.js';
import { toast } from '../../../core/ui/toast.js';

const MAX_MANUAL_ADDRESS_INDEX = 999;

export function splitReceiveAddressLines(address) {
    const chars = Array.from(String(address ?? ''));
    const base = Math.floor(chars.length / 3);
    const remainder = chars.length % 3;
    const lines = [];
    let offset = 0;
    for (let index = 0; index < 3; index += 1) {
        const length = base + (index < remainder ? 1 : 0);
        lines.push(chars.slice(offset, offset + length).join(''));
        offset += length;
    }
    return lines;
}

function renderReceiveAddress(address) {
    const display = byId('receive-address');
    display.replaceChildren();
    display.dataset.address = address;
    for (const line of splitReceiveAddressLines(address)) {
        const row = document.createElement('span');
        row.className = 'address-line';
        row.textContent = line;
        display.appendChild(row);
    }
}

function ensureAddress(chain, index) {
    if (!walletSession.hasWallet()) return '';
    let wallet = walletSession.current();
    const key = chain === 'change' ? 'change_addresses' : 'receive_addresses';
    const addresses = wallet[key] || [];
    if (index >= addresses.length) {
        const missing = index + 1 - addresses.length;
        const extraReceive = chain === 'receive' ? missing : 0;
        const extraChange = chain === 'change' ? missing : 0;
        walletSession.replace(extend_addresses(walletSession.json(), extraReceive, extraChange, networkState.network));
        wallet = walletSession.current();
    }
    return wallet[key]?.[index] || '';
}

function renderAddress(chain, index) {
    const address = ensureAddress(chain, index);
    if (!address) throw new Error('Could not derive the requested address');
    try {
        const frames = JSON.parse(generate_qr_frames(utf8ToHex(address)));
        byId('receive-qr').innerHTML = frames[0].svg;
    } catch (_) {
        byId('receive-qr').innerHTML = '';
    }
    renderReceiveAddress(address);
    byId('receive-chain').value = chain;
    byId('receive-index').value = String(index);
}

export function showReceive() {
    if (!walletSession.hasWallet()) return;
    const index = getNextReceiveIndex();
    const panel = byId('receive-advanced-panel');
    panel.classList.add('hidden');
    const toggle = byId('btn-receive-advanced');
    toggle.textContent = 'Advanced ▸';
    toggle.setAttribute('aria-expanded', 'false');
    try {
        renderAddress('receive', index);
    } catch (error) {
        toast(`Could not show receive address: ${error}`, 'error', 3000);
        return;
    }
    showScreen('receive');
}

export function toggleReceiveAdvanced() {
    const panel = byId('receive-advanced-panel');
    const toggle = byId('btn-receive-advanced');
    const opening = panel.classList.contains('hidden');
    panel.classList.toggle('hidden', !opening);
    toggle.textContent = opening ? 'Advanced ▾' : 'Advanced ▸';
    toggle.setAttribute('aria-expanded', opening ? 'true' : 'false');
}

export function applyReceiveAdvanced() {
    const chain = byId('receive-chain').value === 'change' ? 'change' : 'receive';
    const raw = byId('receive-index').value.trim();
    const index = Number.parseInt(raw, 10);
    if (!Number.isSafeInteger(index) || index < 0 || index > MAX_MANUAL_ADDRESS_INDEX || String(index) !== raw.replace(/^0+(?=\d)/, '')) {
        toast(`Address index must be a whole number from 0 to ${MAX_MANUAL_ADDRESS_INDEX}`, 'error', 3000);
        return;
    }
    try {
        renderAddress(chain, index);
    } catch (error) {
        toast(`Could not derive address: ${error}`, 'error', 3000);
    }
}

export function copyAddress() {
    const display = byId('receive-address');
    const addr = display.dataset.address || display.textContent.replace(/\s+/g, '');
    navigator.clipboard.writeText(addr).then(() => {
        byId('btn-copy-address').textContent = 'Copied!';
        setTimeout(() => { byId('btn-copy-address').textContent = 'Copy Address'; }, 1600);
    });
}
