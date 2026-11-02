import {
    covenantState,
    navigationState,
    networkState,
    fetch_utxos_for_address_js,
} from '../boundary.js';
import { resolveNodeUrl } from '../../../core/node/resolver.js';
import { toast } from '../../../core/ui/toast.js';
import { openSendScreen, setFeeLevel, toggleSendUtxos, updateFeeCardAmounts } from '../../transactions/send/compose/send_form.js';
// KasKold Companion Web — features/covenants/generation/fund
import { byId } from '../../../core/dom.js';

const THREAD_TYPES = ['global-allowance', 'global-spending-limit'];
const DEPOSIT_TYPES = ['additive', 'timelocked-savings', 'dms'];

function oracleBindingReady(result) {
    if (result.type !== 'oracle-v1') return true;
    return /^[0-9a-f]{64}$/.test(result.oracle_covenant_binding_token_hex || '');
}

// A TOP-UP (the covenant address already holds the thread) needs the whole-UTXO fold.
// Initial funding (genesis, empty address) behaves like a normal covenant deposit.
async function isThreadTopup(result) {
    try {
        const wsUrl = await resolveNodeUrl();
        const utxos = JSON.parse(await fetch_utxos_for_address_js(result.address, wsUrl));
        return Array.isArray(utxos) && utxos.length > 0;
    } catch { return false; }
}

function setToggleLabel(label) {
    const toggle = byId('btn-toggle-utxos');
    if (toggle) toggle.textContent = label;
}

function applyTopupView() {
    const amountWrap = byId('send-amount-wrap');
    if (amountWrap) amountWrap.style.display = 'none';
    const list = byId('send-utxo-list');
    if (list && list.style.display === 'none' && networkState.cachedUtxos && networkState.cachedUtxos.length) {
        toggleSendUtxos(); // expand the picker now that UTXOs are loaded
    }
    setToggleLabel('Select UTXO(s) to fold into the thread ▾');
    toast('Top-up: pick the UTXO(s) to fold into the thread (whole UTXOs, no change).', 'info', 4000);
}

// Piggy / savings / DMS deposit: keep the amount field and leave the picker collapsed.
// Savings and DMS pick-up engages the payload-aware deposit fee in the picker.
function applyDepositView(isThreadType) {
    if (isThreadType) { const amountWrap = byId('send-amount-wrap'); if (amountWrap) amountWrap.style.display = ''; }
    const list = byId('send-utxo-list');
    if (list && list.style.display !== 'none') {
        toggleSendUtxos(); // collapse if a prior state left it open
    }
    setToggleLabel('Select UTXO(s) to deposit ▸');
    toast('Open the UTXO picker and choose what to deposit. A dust-sized change is folded into the deposit.', 'info', 4000);
}

async function prepareSendScreen(result) {
    // Piggy bank "Add Funds" routes through the same funding (Send) screen as the
    // first deposit: UTXO picker + amount + a send fee that scales with inputs.
    // A plain deposit adds another covenant UTXO; the additive script breaks a
    // multi-UTXO piggy fine, so Add Funds falls through to openSendScreen.
    navigationState._broadcastReturnScreen = 'covenant';
    await openSendScreen();
    byId('input-dest').value = result.address;
    updateFeeCardAmounts();
    setFeeLevel('normal');
}

export async function handleCovFund() {
    const result = covenantState.lastCovenantResult;
    if (!result) { toast('No covenant address', 'error'); return; }
    if (!oracleBindingReady(result)) {
        toast('Bind the isolated Oracle covenant key to this exact script before funding', 'error', 4500);
        return;
    }
    await prepareSendScreen(result);
    // Thread covenants full-spend the chosen wallet UTXO(s) into the thread: the amount
    // is bypassed, so the amount field is dropped and the UTXO picker is surfaced.
    const isThreadType = THREAD_TYPES.includes(result.type);
    if (isThreadType && await isThreadTopup(result)) {
        applyTopupView();
    } else if (isThreadType || DEPOSIT_TYPES.includes(result.type)) {
        applyDepositView(isThreadType);
    } else {
        toast('Sending to covenant address', 'info', 2000);
    }
}
