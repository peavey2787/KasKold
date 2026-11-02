import { networkState } from '../../../../app/state/index.js';
import { kasToSompi } from '../../../../core/amounts.js';
import { byId } from '../../../../core/dom.js';
import { resolveFutureDaa } from '../../../../core/node/future_daa.js';
import { toast } from '../../../../core/ui/toast.js';
import { covenant_global_allowance, covenant_global_spending_limit, decode_address } from '../../../../wasm/api.js';

function positiveSeconds(inputId, message) {
    const text = byId(inputId).value.trim();
    if (!/^\d+$/.test(text) || BigInt(text) === 0n) {
        toast(message, 'error');
        return null;
    }
    return BigInt(text);
}

function positiveSompi(inputId, message) {
    try {
        const value = kasToSompi(byId(inputId).value);
        if (value === 0n) throw new Error('zero');
        return value;
    } catch {
        toast(message, 'error');
        return null;
    }
}

export async function buildGlobalSpendingLimit(ownerPk) {
    const sompi = positiveSompi('cov-splimit-max', 'Enter max withdrawal in KAS');
    if (sompi === null) return;
    const cooldownSeconds = positiveSeconds('cov-splimit-cooldown', 'Set a cooldown period');
    if (cooldownSeconds === null) return;
    const cooldownDaa = cooldownSeconds * 10n;
    const resultJson = covenant_global_spending_limit(ownerPk, sompi, cooldownDaa, networkState.network);
    return { resultJson, extra: { max_withdraw_sompi: sompi, cooldown_daa: cooldownDaa } };
}

// Resolves the beneficiary input (address or x-only pubkey) to a 64-char hex pubkey. Returns null after toasting.
function resolveBeneficiaryPkHex(benePk) {
    if (!benePk || benePk.length < 32) {
        toast('Scan or paste the beneficiary address or x-only pubkey', 'error');
        return null;
    }
    if (benePk.startsWith('kpub1:')) {
        toast('Use the beneficiary single address or x-only pubkey, not a kpub. A kpub would expose their whole account.', 'error', 6500);
        return null;
    }
    let benePkHex = benePk;
    if (benePk.includes(':')) { // an address; Portal decodes and validates it
        try {
            benePkHex = JSON.parse(decode_address(benePk)).payload;
        } catch (error) {
            toast('Invalid address: ' + error, 'error');
            return null;
        }
    }
    if (!benePkHex || benePkHex.length !== 64) {
        toast('Beneficiary pubkey must be 64 hex chars', 'error');
        return null;
    }
    return benePkHex;
}

// Reads the allowance cooldown period in seconds. Returns null after toasting on failure.
function resolveAllowanceCooldownSeconds() {
    const periodValue = byId('cov-allowance-period').value;
    if (periodValue === 'custom') {
        return positiveSeconds('cov-allowance-seq', 'Set a custom cooldown time');
    }
    if (!/^\d+$/.test(periodValue) || BigInt(periodValue) === 0n) {
        toast('Set a cooldown time', 'error');
        return null;
    }
    return BigInt(periodValue);
}

// Resolves an optional start date to a DAA score; 0n when no start is given. Returns null after toasting.
async function resolveAllowanceStartDaa(startValue) {
    if (!startValue) return 0n;
    try {
        return (await resolveFutureDaa(startValue)).daa;
    } catch (error) {
        toast(error.message, 'error');
        return null;
    }
}

export async function buildGlobalAllowance(ownerPk) {
    const benePkHex = resolveBeneficiaryPkHex(byId('cov-allowance-bene-pk').value.trim());
    if (benePkHex === null) return;

    const sompi = positiveSompi('cov-allowance-max', 'Enter max withdrawal in KAS');
    if (sompi === null) return;

    const cooldownSeconds = resolveAllowanceCooldownSeconds();
    if (cooldownSeconds === null) return;
    const cooldownDaa = cooldownSeconds * 10n;

    const startValue = byId('cov-allowance-start')?.value || '';
    const startDaa = await resolveAllowanceStartDaa(startValue);
    if (startDaa === null) return;

    const resultJson = covenant_global_allowance(
        ownerPk,
        benePkHex,
        sompi,
        cooldownDaa,
        startDaa,
        networkState.network,
    );
    const extra = {
        beneficiary_pubkey_hex: benePkHex,
        max_withdraw_sompi: sompi,
        cooldown_daa: cooldownDaa,
        start_daa: startDaa,
    };
    if (startValue) extra.start_date_iso = new Date(startValue).toISOString();
    return { resultJson, extra };
}
