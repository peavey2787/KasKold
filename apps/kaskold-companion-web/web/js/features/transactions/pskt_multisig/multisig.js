import { descriptorSummary } from './descriptor_summary.js';
import { renderScanProgress } from '../../../core/ui/scan_progress.js';
import { bytesToHex, hexToBytes } from '../../../core/bytes.js';
import {
    networkState,
    transactionState,
    create_multisig_pskb,
    create_multisig_pskb_selected,
    create_multisig_pskb_multi_js,
    scan_multisig_branch_js,
    decode_qr_frame,
    decoder_progress,
    fetch_utxos_for_address_js,
} from '../boundary.js';
import { KNS_LOOKUP } from '../../../core/config/services.js';
import { hideLoading, showLoading, showScreen } from '../../../app/navigation.js';
import { resolveNodeUrl } from '../../../core/node/resolver.js';
import { toast } from '../../../core/ui/toast.js';
import { getCovFee } from '../../covenants/payload/state.js';
import { stopScanner } from '../../stealth/index/camera.js';
import { openPsktReview } from './review.js';
// KasKold Companion Web — features/transactions/pskt_multisig/multisig
import { byId } from '../../../core/dom.js';
import { addressPrefix } from '../../../core/network.js';

import { kasToSompi, sompiToKasFixed, sompiToKasString } from '../../../core/amounts.js';
import { exactUnsigned } from '../../../core/exact.js';
import { normalizeUtxos, sortUtxosLargestFirst } from '../../../core/utxo.js';
import { renderUtxoSelector } from '../shared/utxo_selector.js';
import { selectedUtxoIndices, selectedUtxos } from '../shared/utxo_selection.js';

// ─── Multisig Spend ───

export function handleDescriptorScan(data) {
    // Descriptor comes as multi-frame binary (same protocol as KSPT)
    const hexStr = bytesToHex(new Uint8Array(data));
    try {
        const result = decode_qr_frame(hexStr);
        if (result && result.length > 0) {
            stopScanner();
            // Convert hex back to ASCII text
            const text = new TextDecoder().decode(hexToBytes(result)).trim();
            if (descriptorSummary(text)) {
                byId('input-ms-descriptor').value = text;
                syncCosignerBranch(text);
                showScreen('multisig-spend');
                toast('Descriptor scanned', 'ok', 1500);
            } else {
                toast('Not a valid descriptor', 'error');
            }
        } else {
            renderScanProgress(byId('scanner-status'), JSON.parse(decoder_progress()));
        }
    } catch (e) {
        console.error('Descriptor decode error:', e);
    }
}
function isHd45Descriptor(descriptor) {
    return descriptorSummary(descriptor)?.kind === 'hd45';
}

function hd45ParticipantCount(descriptor) {
    const summary = descriptorSummary(descriptor);
    return summary?.kind === 'hd45' ? summary.participants : 0;
}

function syncCosignerBranch(descriptor) {
    const input = byId('input-ms-cosigner');
    const count = hd45ParticipantCount(descriptor);
    if (count > 0) input.max = String(count - 1);
    const parsed = Number.parseInt(input.value, 10);
    if (!Number.isInteger(parsed) || parsed < 0 || (count > 0 && parsed >= count)) input.value = '0';
    return Number.parseInt(input.value, 10) || 0;
}

function selectedMultisigPrefix() {
    const prefix = addressPrefix(networkState.network).replace(/:$/, '');
    if (!prefix) throw new Error(`Unsupported network for multisig discovery: ${networkState.network}`);
    return prefix;
}

function selectedBranchSources() {
    const scan = networkState.msBranchScan;
    const selected = transactionState.msBranchSelectedUtxos || [];
    if (!scan || selected.length === 0) return [];
    const wanted = new Set(selected);
    return (scan.utxos || []).filter((utxo) => wanted.has(`${utxo.tx_id}:${utxo.outpoint_index}`));
}

function renderBranchUtxos(utxos) {
    const root = byId('ms-branch-utxos');
    root.textContent = '';
    transactionState.msBranchSelectedUtxos = [];
    if (!utxos.length) { root.classList.add('hidden'); return; }
    root.classList.remove('hidden');
    for (const utxo of utxos) {
        const key = `${utxo.tx_id}:${utxo.outpoint_index}`;
        const row = document.createElement('label');
        row.className = 'utxo-row';
        const check = document.createElement('input');
        check.type = 'checkbox';
        check.addEventListener('change', () => {
            const current = new Set(transactionState.msBranchSelectedUtxos || []);
            if (check.checked) {
                if (!current.has(key) && current.size >= 3) {
                    check.checked = false;
                    toast('Select at most 3 multisig UTXOs', 'error', 2500);
                    return;
                }
                current.add(key);
            } else {
                current.delete(key);
            }
            transactionState.msBranchSelectedUtxos = [...current];
        });
        const text = document.createElement('span');
        const amount = exactUnsigned(utxo.amount, 'multisig branch amount');
        text.textContent = `C${utxo.chain} #${utxo.index} · ${sompiToKasString(amount)} KAS · ${utxo.address}`;
        row.append(check, text);
        root.appendChild(row);
    }
}

function setDiscoveryStatus(message, state = '') {
    const info = byId('ms-discovery-info');
    info.textContent = message;
    info.dataset.state = state;
}

function renderDiscoveryResult(result, cosigner) {
    const balance = exactUnsigned(result.balance_sompi, 'multisig branch balance');
    const receive = typeof result.next_receive_address === 'string' ? result.next_receive_address.trim() : '';
    const change = typeof result.next_change_address === 'string' ? result.next_change_address.trim() : '';
    if (!receive || !change) {
        throw new Error('Discovery response is missing receive/change addresses; rebuild and reload Companion');
    }
    const fundingHint = balance === 0n
        ? '\nUnfunded multisig branch — your regular wallet balance is separate. Fund the Receive address before creating a multisig transaction.'
        : '';
    setDiscoveryStatus(
        `Branch S${cosigner}: ${sompiToKasString(balance)} KAS · ${result.utxo_count} UTXOs\n`
        + `Receive #${result.next_receive_index}: ${receive}\n`
        + `Change #${result.next_change_index}: ${change}${fundingHint}`,
        'ready',
    );
}

export async function discoverMultisigBranch() {
    const descriptor = byId('input-ms-descriptor').value.trim();
    if (!isHd45Descriptor(descriptor)) { toast("Discovery requires a multi_hd45 descriptor", 'error'); return; }
    const cosigner = syncCosignerBranch(descriptor);
    const discoverButton = byId('btn-ms-discover');
    discoverButton.disabled = true;
    setDiscoveryStatus(`Scanning 45' cosigner branch S${cosigner}…`, 'loading');
    showLoading("Scanning 45' receive/change branch...");
    try {
        const wsUrl = await resolveNodeUrl();
        const result = JSON.parse(await scan_multisig_branch_js(JSON.stringify({
            descriptor, cosigner_index: cosigner, depth: 40, ws_url: wsUrl,
            address_prefix: selectedMultisigPrefix(),
        })));
        networkState.msBranchScan = result;
        renderDiscoveryResult(result, cosigner);
        renderBranchUtxos(result.utxos || []);
        if (!byId('input-ms-source').value.trim()) {
            byId('input-ms-source').value = result.utxos?.[0]?.address || result.next_receive_address || '';
        }
        toast('Multisig receive/change addresses discovered', 'ok', 1800);
    } catch (error) {
        setDiscoveryStatus('Discovery failed: ' + error, 'error');
        toast('Multisig discovery failed: ' + error, 'error', 5000);
    } finally {
        hideLoading();
        discoverButton.disabled = false;
    }
}

export async function toggleMsUtxos() {
    const list = byId('ms-utxo-list');
    if (!list.classList.contains('hidden')) {
        list.classList.add('hidden');
        byId('btn-toggle-ms-utxos').textContent = 'Select UTXOs manually ▸';
        transactionState.msSelectedUtxoIds = null;
        return;
    }
    const sourceAddr = byId('input-ms-source').value.trim();
    if (!sourceAddr) { toast('Enter source address first', 'error'); return; }

    try {
        const wsUrl = await resolveNodeUrl();
        const utxosJson = await fetch_utxos_for_address_js(sourceAddr, wsUrl);
        networkState.msCachedUtxos = normalizeUtxos(JSON.parse(utxosJson));
        sortUtxosLargestFirst(networkState.msCachedUtxos);
    } catch (e) {
        toast('UTXO fetch failed: ' + e, 'error');
        return;
    }

    if (!networkState.msCachedUtxos || networkState.msCachedUtxos.length === 0) {
        toast('No UTXOs for this address', 'error');
        return;
    }

    byId('btn-toggle-ms-utxos').textContent = 'Select UTXOs manually ▾';
    transactionState.msSelectedUtxoIds = [];
    renderUtxoSelector(list, networkState.msCachedUtxos, transactionState.msSelectedUtxoIds, { limit: 8, sort: 'desc' }, ids => { transactionState.msSelectedUtxoIds = ids; });
}
function readMultisigCreateInput() {
    const input = {
        descriptor: byId('input-ms-descriptor').value.trim(),
        sourceAddr: byId('input-ms-source').value.trim(),
        destAddr: byId('input-ms-dest').value.trim(),
    };
    const amountStr = byId('input-ms-amount').value.trim();

    if (!input.descriptor) { toast('Paste the multisig descriptor', 'error'); return null; }
    if (!input.sourceAddr) { toast('Enter the P2SH source address', 'error'); return null; }
    if (!input.destAddr) { toast('Enter the destination address', 'error'); return null; }
    try { input.amountSompi = kasToSompi(amountStr); } catch { toast('Enter a valid amount with at most 8 decimal places', 'error'); return null; }
    if (input.amountSompi <= 0n) { toast('Enter amount', 'error'); return null; }
    return input;
}

function resolveMultisigDestination(destAddr) {
    if (!destAddr.endsWith('.kas')) return destAddr;
    const kns = KNS_LOOKUP[destAddr.toLowerCase()];
    if (!kns) { toast('Unknown .kas domain', 'error'); return null; }
    toast('Resolved ' + destAddr + ' → address', 'ok', 2000);
    return kns;
}

function msChangeIndexHint(descriptor) {
    return isHd45Descriptor(descriptor) ? (networkState.msBranchScan?.next_change_index ?? 0xffffffff) : 0xffffffff;
}

async function buildMultisigParams(input, resolvedDest) {
    const fee = exactUnsigned(getCovFee(), 'multisig fee');
    const wsUrl = await resolveNodeUrl();
    const addrIndexEl = byId('input-ms-addr-index');
    const addrIndex = addrIndexEl ? parseInt(addrIndexEl.value) || 0 : 0;
    return { ...input, resolvedDest, fee, wsUrl, addrIndex };
}

function buildBranchMultisigPskb(p, branchSources) {
    return create_multisig_pskb_multi_js(JSON.stringify({
        descriptor: p.descriptor,
        sources_json: JSON.stringify(branchSources.map((utxo) => ({
            address: utxo.address, tx_id: utxo.tx_id, index: utxo.outpoint_index,
        }))),
        dest_address: p.resolvedDest,
        amount_sompi: p.amountSompi.toString(),
        fee_sompi: p.fee.toString(),
        cosigner_index: syncCosignerBranch(p.descriptor),
        change_index_hint: networkState.msBranchScan?.next_change_index ?? 0xffffffff,
        ws_url: p.wsUrl,
    }));
}

function buildSelectedMultisigPskb(p) {
    const csv = selectedUtxoIndices(networkState.msCachedUtxos, transactionState.msSelectedUtxoIds).join(',');
    return create_multisig_pskb_selected(JSON.stringify({
        descriptor: p.descriptor,
        source_address: p.sourceAddr,
        dest_address: p.resolvedDest,
        amount_sompi: p.amountSompi.toString(),
        fee_sompi: p.fee.toString(),
        change_address: p.sourceAddr,
        ws_url: p.wsUrl,
        addr_index: p.addrIndex,
        change_index_hint: msChangeIndexHint(p.descriptor),
        utxo_csv: csv,
    }));
}

function buildAutoMultisigPskb(p) {
    return create_multisig_pskb(JSON.stringify({
        descriptor: p.descriptor,
        source_address: p.sourceAddr,
        dest_address: p.resolvedDest,
        amount_sompi: p.amountSompi.toString(),
        fee_sompi: p.fee.toString(),
        change_address: p.sourceAddr,
        ws_url: p.wsUrl,
        addr_index: p.addrIndex,
        change_index_hint: msChangeIndexHint(p.descriptor),
    }));
}

function hasManualUtxoSelection() {
    return !!transactionState.msSelectedUtxoIds && transactionState.msSelectedUtxoIds.length > 0;
}

function buildMultisigPskb(p) {
    const branchSources = selectedBranchSources();
    if (branchSources.length > 3) throw new Error('Select between 1 and 3 multisig UTXOs');
    if (branchSources.length > 0) return buildBranchMultisigPskb(p, branchSources);
    if (hasManualUtxoSelection()) return buildSelectedMultisigPskb(p);
    return buildAutoMultisigPskb(p);
}

export async function handleMultisigCreate() {
    const input = readMultisigCreateInput();
    if (!input) return;
    const resolvedDest = resolveMultisigDestination(input.destAddr);
    if (!resolvedDest) return;

    showLoading('Building multisig PSKB...');
    try {
        const pskbHex = await buildMultisigPskb(await buildMultisigParams(input, resolvedDest));
        hideLoading();
        console.log('[Companion] Multisig PSKB created: ' + pskbHex.length / 2 + ' bytes');
        openPsktReview(pskbHex, { kind: 'multisig-send', destinationAddress: resolvedDest });
    } catch (e) {
        hideLoading();
        toast('Multisig TX failed: ' + e, 'error', 5000);
        console.error('Multisig TX error:', e);
    }
}
function sumUtxoAmounts(utxos) {
    return utxos.reduce((sum, utxo) => sum + exactUnsigned(utxo.amount, 'UTXO amount'), 0n);
}

function setMultisigMaximum(total, fee) {
    byId('input-ms-amount').value = sompiToKasFixed(total > fee ? total - fee : 0n);
}

function selectedMultisigTotal() {
    const branchSources = selectedBranchSources();
    if (branchSources.length > 0) return sumUtxoAmounts(branchSources);
    if (transactionState.msSelectedUtxoIds && transactionState.msSelectedUtxoIds.length > 0 && networkState.msCachedUtxos) {
        return sumUtxoAmounts(selectedUtxos(networkState.msCachedUtxos, transactionState.msSelectedUtxoIds));
    }
    return null;
}

async function fetchMultisigBalance(sourceAddr, fee) {
    showLoading('Fetching balance...');
    try {
        const wsUrl = await resolveNodeUrl();
        const utxosJson = await fetch_utxos_for_address_js(sourceAddr, wsUrl);
        hideLoading();
        const utxos = normalizeUtxos(JSON.parse(utxosJson));
        const total = utxos.reduce((s, u) => s + u.amount, 0n);
        setMultisigMaximum(total, fee);
        byId('ms-balance-info').textContent = 'Balance: ' + sompiToKasString(total) + ' KAS (' + utxos.length + ' UTXOs)';
        if (total === 0n && networkState.msBranchScan?.utxo_count === 0) {
            toast('This multisig receive branch is unfunded. Fund its discovered Receive address first.', 'error', 4000);
        }
    } catch (e) {
        hideLoading();
        toast('Balance fetch failed: ' + e, 'error');
    }
}

export async function handleMsMax() {
    const sourceAddr = byId('input-ms-source').value.trim();
    if (!sourceAddr) { toast('Enter source address first', 'error'); return; }

    const fee = exactUnsigned(getCovFee(), 'multisig fee');
    const selectedTotal = selectedMultisigTotal();
    if (selectedTotal !== null) {
        setMultisigMaximum(selectedTotal, fee);
        return;
    }
    await fetchMultisigBalance(sourceAddr, fee);
}
