// @ts-check
import { el, setContent, span } from '../../../../../core/ui/elements.js';
import {
    networkState,
    stealthState,
    walletSession,
    encode_p2pk_address,
    fetch_utxos_for_address_js,
} from '../../../boundary.js';
import { showScreen } from '../../../../../app/navigation.js';
import { bytesToHex } from '../../../../../core/bytes.js';
import { byId, elementById } from '../../../../../core/dom.js';
import { detectWalletNetwork } from '../../../../../core/network.js';
import { resolveNodeUrl } from '../../../../../core/node/resolver.js';
import { toast } from '../../../../../core/ui/toast.js';
import { stopScanner } from '../../camera.js';
import { stealthFeePrepare, stealthFeeSetLevel, stealthShowPanel } from '../../send.js';
import { handleStealthSpend } from '../../spend.js';
import { clearStealthQrTimer } from './lifecycle.js';
import { sompiToKasFixed } from '../../../../../core/amounts.js';

function collectDeviceResults(raw) {
    const count = raw[4];
    stealthState._stealthResults ||= [];
    for (let index = 0; index < count; index++) {
        const offset = 5 + index * 64;
        const pubkey = bytesToHex(raw.slice(offset, offset + 32));
        const tweak = bytesToHex(raw.slice(offset + 32, offset + 64));
        if (/^0+$/.test(pubkey)) continue;
        if (stealthState._stealthResults.some(result => result.pubkey === pubkey)) continue;
        stealthState._stealthResults.push({ pubkey, tweak });
    }
    return count;
}

async function findFundedResults() {
    const network = detectWalletNetwork(walletSession.json(), networkState.network);
    const websocketUrl = await resolveNodeUrl();
    const funded = [];
    let unchecked = 0;
    for (const result of stealthState._stealthResults) {
        let address = '';
        try { address = encode_p2pk_address(result.pubkey, network); } catch { /* A key that cannot encode is listed with no address. */ }
        if (!address) continue;
        try {
            const utxos = JSON.parse(await fetch_utxos_for_address_js(address, websocketUrl));
            const total = utxos.reduce((sum, utxo) => sum + BigInt(utxo.amount), 0n);
            if (total > 0n) funded.push({ ...result, address, total });
        } catch {
            unchecked++;
        }
    }
    return { funded, unchecked };
}

const STEALTH_FEE_LEVELS = [
    { level: 'low', label: 'Low', amount: '2,500' },
    { level: 'normal', label: 'Normal', amount: '5,000', active: true },
    { level: 'priority', label: 'Priority', amount: '10,000' },
];

/** @param {{ level: string, label: string, amount: string, active?: boolean }} option */
function feeCard({ level, label, amount, active }) {
    return el('button', { className: `fee-card${active ? ' fee-card-active' : ''}`, id: `btn-spf-${level}` },
        el('div', { className: 'fee-card-label' }, label),
        el('div', { className: 'fee-card-amount', id: `spf-${level}-amount` }, amount),
        el('div', { className: 'fee-card-time', id: `spf-${level}-time` }));
}

function paymentCard(result, index) {
    const spend = el('button', { className: 'btn btn-primary stealth-spend-btn stealth-spend-btn-wide' }, 'Spend This Payment');
    spend.addEventListener('click', () => handleStealthSpend(result.pubkey, result.tweak));
    return el('div', { className: 'stealth-payment-card' },
        el('div', { className: 'stealth-payment-title' },
            span('u-text-accent-teal', `Payment ${index + 1}`), ' · ',
            span('u-text-accent-teal', `${sompiToKasFixed(result.total, 2)} KAS`)),
        el('div', { className: 'stealth-payment-address' }, result.address),
        spend);
}

function uncheckedNotice(unchecked) {
    return `${unchecked} payment address${unchecked === 1 ? '' : 'es'} could not be checked. Try again when the node is reachable.`;
}

function renderFundedResults({ funded, unchecked }) {
    const list = elementById('stealth-found-list');
    if (funded.length === 0) {
        list.textContent = unchecked > 0 ? uncheckedNotice(unchecked) : 'No funded payments for this wallet.';
        return;
    }
    setContent(list,
        unchecked > 0 ? el('div', { className: 'input-label' }, uncheckedNotice(unchecked)) : '',
        el('label', { className: 'input-label' }, 'Fee'),
        el('div', { className: 'fee-cards' }, STEALTH_FEE_LEVELS.map(feeCard)),
        el('input', { className: 'input-text', id: 'input-spf-fee', attrs: { type: 'number', value: '5000', step: '1', min: '1' } }),
        funded.map(paymentCard));
    ['low', 'normal', 'priority'].forEach(level => {
        const button = byId(`btn-spf-${level}`);
        if (button) button.onclick = () => stealthFeeSetLevel('spf', 'spend', level);
    });
    stealthFeePrepare('spf', 'spend');
}

export async function processStealthResult(raw) {
    const count = collectDeviceResults(raw);
    stopScanner();
    clearStealthQrTimer();
    showScreen('stealth');
    stealthShowPanel('scan');
    elementById('stealth-found-list').textContent = 'Checking balances...';
    elementById('stealth-scan-results').classList.remove('hidden');
    try {
        renderFundedResults(await findFundedResults());
    } catch {
        elementById('stealth-found-list').textContent = 'Node unavailable, cannot check balances.';
    }

    stealthState._stealthBatchStart = (stealthState._stealthBatchStart || 0) + count;
    const remaining = stealthState.stealthAnnouncementsR.length - stealthState._stealthBatchStart;
    if (remaining > 0) {
        setContent(byId('stealth-scan-status'), el('strong', {}, `${remaining} more R to check.`), ' Tap "Show Scan QR" for the next batch.');
    }
    toast(`Scanned ${count} result(s)${remaining > 0 ? `, ${remaining} R left` : ''}`, 'ok', 2000);
}
