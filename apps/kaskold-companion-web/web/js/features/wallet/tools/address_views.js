import { navigationState, networkState, transactionState, walletSession, walletState } from '../../../app/state/index.js';
import { hideLoading, showLoading, showScreen } from '../../../app/navigation.js';
import { toast } from '../../../core/ui/toast.js';
import { updateConsolidateButtons } from './consolidation.js';
import { explorerAddressUrl } from '../../../core/explorer.js';
import { generate_qr_frames } from '../../../wasm/api.js';
import { fetchCoinControlUtxos } from '../core/coin_control_utxos.js';

import { utf8ToHex } from '../../../core/bytes.js';
import { byId } from '../../../core/dom.js';
import { qrImage } from '../../../core/qr/svg_image.js';
import { el, setContent, span } from '../../../core/ui/elements.js';
import { sompiToKasFixed } from '../../../core/amounts.js';
import { normalizeUtxos, sortUtxosLargestFirst } from '../../../core/utxo.js';
import { signerMaxInputs } from '../../transactions/shared/signer_limits.js';

navigationState.addressesReturnScreen = 'dashboard';

function copyAddressHandler(address, icon) {
    return event => {
        event.stopPropagation();
        navigator.clipboard.writeText(address);
        icon.textContent = '✓';
        setTimeout(() => { icon.textContent = '⧉'; }, 800);
        toast('Address copied', 'ok', 1000);
    };
}

function addressRow(address, index, isChange, observed) {
    const funded = observed.funded.has(index);
    const used = !funded && Boolean(observed.used?.has(index));
    const explore = el('a', {
        className: 'addr-explore',
        attrs: { href: explorerAddressUrl(networkState.network, address), target: '_blank', rel: 'noopener', title: 'View in explorer' },
    }, '↗');
    explore.onclick = event => event.stopPropagation();
    const copyIcon = span('copy-icon', '⧉');
    copyIcon.onclick = copyAddressHandler(address, copyIcon);
    const row = el('div', { className: `addr-item${funded || used ? ' addr-used' : ''}`, dataset: { addr: `${index}-${isChange ? 'c' : 'r'}` } },
        span('addr-idx', String(index)),
        span('addr-val', address),
        funded ? span('addr-badge', 'funded') : null,
        used ? span('addr-badge used', 'used') : null,
        explore,
        copyIcon);
    row.onclick = () => showVerify(address, index, isChange);
    return row;
}

export function showAddresses() {
    if (!walletSession.hasWallet()) return;
    navigationState.addressesReturnScreen = (navigationState.currentScreenName && navigationState.currentScreenName !== 'addresses') ? navigationState.currentScreenName : 'dashboard';
    const wallet = walletSession.current();
    const receive = { funded: new Set(walletState.fundedReceiveIndices), used: walletState.usedReceiveIndices };
    const change = { funded: new Set(walletState.fundedChangeIndices), used: walletState.usedChangeIndices };
    setContent(byId('address-list'),
        el('div', { className: 'addr-section-title' }, "Receive (m/44'/111111'/0'/0)"),
        wallet.receive_addresses.map((address, index) => addressRow(address, index, false, receive)),
        el('div', { className: 'addr-section-title' }, "Change (m/44'/111111'/0'/1)"),
        wallet.change_addresses.map((address, index) => addressRow(address, index, true, change)));
    showScreen('addresses');
}

function showVerify(addr, index, isChange) {
    const path = isChange
        ? `m/44'/111111'/0'/1/${index}`
        : `m/44'/111111'/0'/0/${index}`;
    byId('verify-path').textContent = path;
    byId('verify-address').textContent = addr;

    const qr = byId('verify-qr');
    try {
        const frames = JSON.parse(generate_qr_frames(utf8ToHex(addr)));
        const svg = String(frames[0]?.svg || '');
        if (!svg) throw new Error('Address QR frame is empty');
        qr.replaceChildren(qrImage(svg, 'Address QR code'));
    } catch (_) {
        qr.replaceChildren();
    }

    // Explorer link
    const link = byId('btn-verify-explore');
    if (link) {
        link.href = explorerAddressUrl(networkState.network, addr);
    }
    showScreen('verify');
}

function toggleUtxoSelection(index, row, check, utxoCount) {
    const selection = transactionState.consolidateSelection;
    if (selection.has(index)) {
        selection.delete(index);
    } else if (selection.size < signerMaxInputs()) {
        selection.add(index);
    } else {
        toast(`KasKold supports at most ${signerMaxInputs()} selected inputs`, 'info', 1800);
        return;
    }
    check.textContent = selection.has(index) ? '☑' : '☐';
    row.style.borderColor = selection.has(index) ? 'var(--teal)' : '';
    updateConsolidateButtons(utxoCount);
}

function selectableUtxoRow(utxo, index, utxoCount) {
    const check = el('div', { className: 'utxo-check' }, transactionState.consolidateSelection.has(index) ? '☑' : '☐');
    const row = el('div', { className: 'utxo-item utxo-selectable', dataset: { utxoIdx: index } },
        check,
        el('div', { className: 'utxo-info' },
            el('div', { className: 'utxo-amount' }, `${sompiToKasFixed(utxo.amount)} KAS`),
            el('div', { className: 'utxo-detail' }, `${utxo.tx_id.slice(0, 16)}…:${utxo.index}`)));
    row.onclick = () => toggleUtxoSelection(index, row, check, utxoCount);
    return row;
}

export async function showUtxos() {
    if (!walletSession.hasWallet()) return;
    showLoading('Fetching UTXOs...');
    transactionState.consolidateSelection = new Set();

    try {
        const coinControl = await fetchCoinControlUtxos();
        const utxos = normalizeUtxos(coinControl.utxos);
        hideLoading();
        networkState.cachedUtxos = utxos;

        const totalSompi = utxos.reduce((s, u) => s + u.amount, 0n);
        byId('utxo-summary').textContent = `${utxos.length} current UTXO${utxos.length !== 1 ? 's' : ''} · ${sompiToKasFixed(totalSompi)} KAS · ${coinControl.scannedAddresses} addresses scanned · ${coinControl.source}`;

        if (utxos.length === 0) {
            setContent(byId('utxo-list'), el('div', { className: 'u-align-text-center-text-text-muted-padding-20px' }, 'No UTXOs found'));
            byId('btn-consolidate').style.display = 'none';
            byId('btn-consolidate-selected').style.display = 'none';
            byId('btn-send-selected-utxos').style.display = 'none';
        } else {
            sortUtxosLargestFirst(utxos);
            setContent(byId('utxo-list'), utxos.map((utxo, index) => selectableUtxoRow(utxo, index, utxos.length)));

            updateConsolidateButtons(utxos.length);
        }

        showScreen('utxos');
    } catch (e) {
        hideLoading();
        toast('Failed to fetch UTXOs: ' + e, 'error', 5000);
    }
}
