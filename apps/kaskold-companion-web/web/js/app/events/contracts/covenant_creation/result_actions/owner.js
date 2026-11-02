// @ts-check
import { covenantState } from '../../../../state/index.js';
import { piggyBreakStatus, piggyStatusBanner } from '../result_actions.js';
import { fetchCurrentDaa } from '../../../../../core/node/daa.js';
import { resolveNodeUrl } from '../../../../../core/node/resolver.js';
import { toast } from '../../../../../core/ui/toast.js';
import { covShowPanel } from '../../../../../features/covenants/generation/panel_keys.js';
import { TIMELOCKED_OWNER_REASONS } from '../../../../../features/covenants/model/owner_timelock.js';
import { getCovFee, ownerReceiveAddr } from '../../../../../features/covenants/payload/state.js';
import { handleEscrowSpend } from '../../../../../features/covenants/spending/standard/shipment.js';
import { fetch_utxos_for_address_js } from '../../../../../wasm/api.js';
import { byId, elementById, inputById } from '../../../../../core/dom.js';
import { openUtxoPicker } from '../utxo_picker.js';

import { formatDaaDuration } from '../../../../../core/format.js';
import { sompiToKasString } from '../../../../../core/amounts.js';
import { exactUnsigned } from '../../../../../core/exact.js';
function piggyHelpText(covenant) {
    const hasGoal = exactUnsigned(covenant.threshold_sompi ?? 0n, 'threshold sompi') > 0n;
    const hasDeadline = exactUnsigned(covenant.deadline_daa ?? 0n, 'deadline DAA') > 0n;
    const goal = sompiToKasString(covenant.threshold_sompi ?? 0n);
    if (hasGoal && hasDeadline) return `Break the piggy bank. Requires goal (${goal} KAS) to be reached OR the deadline to have passed.`;
    if (hasGoal) return `Break the piggy bank. Requires goal (${goal} KAS) to be reached.`;
    if (hasDeadline) return 'Break the piggy bank. Requires the deadline to have passed.';
    return 'Break the piggy bank. No conditions set. Can break anytime.';
}

async function showTimelockStatus(covenantType) {
    const restriction = TIMELOCKED_OWNER_REASONS.get(covenantType);
    const locktime = exactUnsigned(covenantState.lastCovenantResult?.locktime_daa ?? 0n, 'owner locktime DAA');
    if (!restriction || locktime === 0n) return;
    try {
        let currentDaa = 0n;
        try { currentDaa = await fetchCurrentDaa(); } catch { /* With no node answer the DAA stays at zero, so time locks read as still locked. */ }
        if (currentDaa === 0n) currentDaa = exactUnsigned(covenantState._lastKnownDaa ?? 0n, 'last known DAA');
        if (currentDaa > 0n && currentDaa < locktime) {
            piggyStatusBanner({
                text: `Owner reclaim NOT available yet: timelock matures in ~${formatDaaDuration(locktime - currentDaa)}. Until then ${restriction}.`,
                color: 'var(--error, #f44336)',
            });
        } else if (currentDaa > 0n) {
            piggyStatusBanner({
                text: 'Timelock matured — owner reclaim available now.',
                color: 'var(--accent, #4caf50)',
            });
        }
    } catch { /* The banner is advisory; the spend path checks the timelock again. */ }
}

function configureOwnerHelp(covenantType) {
    const help = byId('cov-owner-help');
    if (!help) return;
    void showTimelockStatus(covenantType);
    if (covenantType === 'global-allowance') {
        help.textContent = 'Owner reclaim. Sweeps the whole thread back to your address via the free owner path (uncapped). To add funds, use Deposit and pick the wallet UTXOs to fold into the thread. Requires owner signature from your KasKold.';
    } else if (covenantType === 'global-spending-limit') {
        const cap = sompiToKasString(covenantState.lastCovenantResult?.max_withdraw_sompi ?? 0n);
        help.textContent = `Withdraw up to the per-spend cap of ${cap} KAS from the single thread. Leave the amount empty to sweep all, which is allowed only when the balance is at or below the cap. To add funds, use Deposit and pick the wallet UTXOs to fold in (top-up merges whole UTXOs into the thread).`;
    } else if (covenantType === 'dms') {
        help.textContent = 'Heartbeat. Sends funds back to the same covenant address, resetting the CSV inactivity timer. Only costs a network fee.';
    } else if (covenantType === 'additive') {
        help.textContent = piggyHelpText(covenantState.lastCovenantResult);
    } else {
        help.style.display = 'none';
        return;
    }
    help.style.display = '';
}

const OWNER_CREATE_LABELS = new Map([
    ['dms', 'Create Heartbeat TX'],
    ['additive', 'Break Piggy Bank'],
    ['oracle-v1', 'Create Timeout Refund TX'],
]);
const OWNER_RECEIVE_PREFILL_TYPES = new Set(['additive', 'global-allowance']);

function ownerAmountPlaceholder(covenantType) {
    const maxSompi = covenantState.lastCovenantResult?.max_withdraw_sompi;
    if (covenantType === 'global-spending-limit' && maxSompi) {
        return `Max ${sompiToKasString(maxSompi)} KAS, empty = sweep all`;
    }
    return 'Empty = sweep all';
}

function configureOwnerAmount(covenantType) {
    const amount = inputById('cov-owner-amount');
    const label = /** @type {HTMLElement | null | undefined} */ (amount?.previousElementSibling);
    const alwaysSweep = covenantType !== 'global-spending-limit';
    if (amount) {
        amount.style.display = alwaysSweep ? 'none' : '';
        if (alwaysSweep) amount.value = '';
        amount.placeholder = ownerAmountPlaceholder(covenantType);
    }
    if (label?.tagName === 'LABEL') {
        label.style.display = alwaysSweep ? 'none' : '';
        if (!alwaysSweep) label.textContent = 'Amount (KAS) — leave empty to sweep all';
    }
}

function configureOwnerConsolidate(covenantType) {
    const consolidate = byId('btn-cov-owner-consolidate');
    if (!consolidate) return;
    consolidate.style.display = covenantType === 'dms' ? '' : 'none';
    if (covenantType === 'dms') consolidate.textContent = 'Consolidate UTXOs (batched heartbeat)';
}

function configureOwnerDestination(covenantType) {
    const destination = inputById('cov-owner-dest');
    if (covenantType === 'dms') {
        destination.value = covenantState.lastCovenantResult.address || '';
        destination.readOnly = true;
        return;
    }
    destination.readOnly = false;
    if (!OWNER_RECEIVE_PREFILL_TYPES.has(covenantType)) return;
    const ownAddress = ownerReceiveAddr();
    if (ownAddress) destination.value = ownAddress;
}

function configureOwnerControls(covenantType) {
    const create = byId('btn-cov-owner-create');
    if (create) create.textContent = OWNER_CREATE_LABELS.get(covenantType) ?? 'Create Owner Spend TX';
    const panel = byId('cov-owner-panel');
    if (panel) panel.dataset.covOwnerMode = covenantType === 'dms' ? 'heartbeat' : '';
    configureOwnerConsolidate(covenantType);
    configureOwnerDestination(covenantType);
}

function openOwnerPanel(covenantType) {
    covShowPanel('owner');
    inputById('cov-owner-addr').value = covenantState.lastCovenantResult.address || '';
    inputById('cov-owner-script').value = covenantState.lastCovenantResult.redeem_script_hex || '';
    const panel = byId('cov-owner-panel');
    if (panel) panel.dataset.covOwnerType = covenantType;
    configureOwnerHelp(covenantType);
    configureOwnerAmount(covenantType);
    configureOwnerControls(covenantType);
}

async function openSinglePiggy(utxos) {
    openOwnerPanel('additive');
    try {
        const total = utxos.reduce((sum, utxo) => sum + BigInt(utxo.amount), 0n);
        piggyStatusBanner(await piggyBreakStatus(total, getCovFee(utxos.length || 1)));
    } catch { /* The banner is advisory; the spend path checks the goal again. */ }
}

async function handleAdditiveOwner() {
    try {
        const wsUrl = await resolveNodeUrl();
        const utxos = JSON.parse(await fetch_utxos_for_address_js(covenantState.lastCovenantResult.address, wsUrl));
        if (utxos.length <= 1) {
            await openSinglePiggy(utxos);
            return;
        }
    } catch { /* Without a UTXO answer the picker opens and asks the node again. */ }
    openUtxoPicker(ownerReceiveAddr());
}

async function handleOwnerAction() {
    if (!covenantState.lastCovenantResult) {
        toast('No covenant loaded', 'error');
        return;
    }
    const covenantType = covenantState.lastCovenantResult.type || '';
    if (covenantType === 'escrow') {
        const branch = covenantState.lastCovenantResult.role === 'arbiter' ? 'arbiter-award-seller' : 'buyer-release';
        await handleEscrowSpend(branch);
        return;
    }
    if (covenantType === 'additive') {
        await handleAdditiveOwner();
        return;
    }
    openOwnerPanel(covenantType);
}

export function registerOwnerAction() {
    elementById('btn-cov-res-owner').onclick = () => handleOwnerAction();
}
