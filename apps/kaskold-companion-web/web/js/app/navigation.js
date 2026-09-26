import { covenantState, navigationState, networkState, walletSession } from './state/index.js';
import { covLoadActive } from '../features/covenants/recovery/active.js';
import { startAutoRefresh, stopAutoRefresh } from '../features/wallet/core.js';
import { get_virtual_daa_score, init, version } from '../wasm/api.js';
import { markWasmFailed, markWasmReady } from '../wasm/runtime.js';
import { parseCovenantJson } from '../features/covenants/model/exact_fields.js';
// KasKold Companion Web — app/navigation
// Owns application startup, screen navigation, and session restoration.

import { byId } from '../core/dom.js';
import { activateScreen, visibleScreenName } from '../core/ui/screen_dom.js';
import { installBackHomeControls } from '../core/ui/navigation_controls.js';

import { bindEvents } from './events/index.js';
import { resolveNodeUrl } from '../core/node/resolver.js';


// ─── Init ───

function loadingPresentation(message) {
    const raw = String(message || '').trim() || 'Loading…';
    const text = raw.toLowerCase();

    if (text.includes('connecting') || text.includes('kaspa node')) {
        return {
            title: 'Connecting to a Kaspa node',
            detail: text.includes('attempt')
                ? raw
                : 'Please wait while Companion establishes the node connection.',
            note: 'Network features will be available as soon as the connection is ready.',
        };
    }
    if (text.includes('transaction history') || text.includes('wallet history') || text.includes('wallet activity')) {
        return {
            title: 'Loading wallet history',
            detail: raw,
            note: 'Reading wallet activity using the current node connection.',
        };
    }
    if (text.includes('wallet')) {
        return {
            title: 'Loading wallet',
            detail: raw,
            note: 'Preparing the selected watch-only wallet and its latest data.',
        };
    }
    if (text.includes('utxo')) {
        return {
            title: text.includes('building') || text.includes('consolidat') ? 'Preparing transaction' : 'Fetching UTXOs',
            detail: raw,
            note: text.includes('building') || text.includes('consolidat')
                ? 'Preparing the unsigned transaction. Nothing is broadcast yet.'
                : 'Reading UTXO data using the current node connection.',
        };
    }
    if (text.includes('fetching balance') || text.includes('checking balance')) {
        return {
            title: 'Refreshing balance',
            detail: raw,
            note: 'Reading the latest balance using the current node connection.',
        };
    }
    if (text.includes('fetching tx')) {
        return {
            title: 'Fetching transaction',
            detail: raw,
            note: 'Reading transaction data using the current node connection.',
        };
    }
    if (text.includes('token') || text.includes('nft')) {
        return {
            title: 'Loading assets',
            detail: raw,
            note: 'Reading asset data for the loaded watch-only wallet.',
        };
    }
    if (text.includes('broadcast')) {
        return {
            title: 'Broadcasting transaction',
            detail: raw,
            note: 'Submitting the signed transaction through the current Kaspa node connection.',
        };
    }
    if (text.includes('building') || text.includes('creating transaction') || text.includes('computing') || text.includes('consolidat')) {
        return {
            title: 'Preparing transaction',
            detail: raw,
            note: 'Preparing the unsigned transaction. Nothing is broadcast yet.',
        };
    }
    if (text.includes('scanning')) {
        return {
            title: 'Scanning',
            detail: raw,
            note: 'Please wait while Companion completes the requested scan.',
        };
    }
    return {
        title: 'Please wait',
        detail: raw,
        note: 'Companion is completing the requested operation.',
    };
}
function sleep(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
}

function showStartupNodeAttempt(attempt) {
    const title = byId('loading-title');
    const detail = byId('loading-msg');
    const note = byId('loading-note');
    if (title) title.textContent = 'Connecting to a Kaspa node';
    if (detail) detail.textContent = attempt <= 1
        ? 'Please wait while Companion establishes the node connection.'
        : `Still connecting… attempt ${attempt}. Please wait.`;
    if (note) note.textContent = 'Companion will open automatically when the connection is ready.';
    byId('loading').classList.remove('hidden');
}

async function waitForInitialNodeConnection() {
    let attempt = 1;
    for (;;) {
        showStartupNodeAttempt(attempt);
        try {
            const wsUrl = await resolveNodeUrl();
            // A successful wRPC call proves that the resolved endpoint is not just
            // syntactically valid but actually reachable from this browser.
            await get_virtual_daa_score(wsUrl);
            return wsUrl;
        } catch (error) {
            console.log(`[Companion] Startup node connection attempt ${attempt} failed:`, error);
        }
        attempt += 1;
        await sleep(Math.min(3000, 800 + attempt * 250));
    }
}

export async function start() {
    installBackHomeControls(navigateHome);
    showScreen('welcome', { recordHistory: false });
    showStartupNodeAttempt(1);
    setStartupStatus('Initializing the Companion watch-only engine…', 'loading');
    const eventBindingFailures = bindEvents();
    let wasmStarted = false;

    try {
        await init();
        markWasmReady();
        wasmStarted = true;
        console.log(version());
        if (eventBindingFailures.length > 0) {
            setStartupStatus('Companion loaded, but some controls could not be connected. Check the browser console.', 'warning');
        } else {
            setStartupStatus('Companion is ready.', 'ready');
        }
    } catch (error) {
        markWasmFailed(error);
        console.error('KasKold Companion WebAssembly initialization failed:', error);
        setStartupStatus(
            'Companion controls are available, but the WebAssembly engine is missing. Run `make companion`, then reload this page.',
            'error',
        );
    }

    // Restore covenant context from sessionStorage (survives reload, dies on tab close)
    try {
        const saved = sessionStorage.getItem('lastCovenantResult');
        if (saved) covenantState.lastCovenantResult = parseCovenantJson(saved);
    } catch (_) {}
    covLoadActive();

    if (wasmStarted) {
        try {
            await waitForInitialNodeConnection();
            setStatus('online', 'Connected');
            const { routeStartupKpub } = await import('../features/wallet/kpub_manager/index.js');
            const startupRoute = routeStartupKpub();
            if (startupRoute.state === 'loaded') {
                const suffix = eventBindingFailures.length > 0
                    ? ' Some controls could not be connected; check the browser console.'
                    : '';
                setStartupStatus(`Loaded saved wallet “${startupRoute.entry.name}”.${suffix}`, eventBindingFailures.length > 0 ? 'warning' : 'ready');
            } else if (startupRoute.state === 'failed') {
                setStartupStatus(
                    `Companion could not load saved wallet “${startupRoute.entry.name}”. Choose another saved wallet or load a new one.`,
                    'warning',
                );
            } else if (startupRoute.state === 'selection') {
                setStartupStatus('', 'ready');
            } else {
                setStartupStatus('No saved wallets yet. Use Manage Wallets to add one.', 'ready');
            }
            hideLoading();
        } catch (error) {
            console.error('Companion saved-kpub startup failed:', error);
            hideLoading();
            setStartupStatus('Companion is ready, but saved wallets could not be read from browser storage.', 'warning');
        }
    } else {
        hideLoading();
    }

}

export function setStartupStatus(message, state) {
    const status = byId('companion-startup-status');
    if (!status) return;
    const text = String(message || '').trim();
    status.textContent = text;
    status.dataset.state = state || '';
    status.classList.toggle('hidden', text.length === 0);
}

navigationState.currentScreenName = 'welcome';

const MAX_SCREEN_HISTORY = 64;

function recordHistory(nextScreen) {
    const current = visibleScreenName(navigationState.currentScreenName || 'welcome');
    if (!current || current === nextScreen) return;
    const history = navigationState.screenHistory;
    if (history.at(-1) !== current) history.push(current);
    if (history.length > MAX_SCREEN_HISTORY) history.splice(0, history.length - MAX_SCREEN_HISTORY);
}

export function showScreen(name, options = {}) {
    if (options.recordHistory !== false) recordHistory(name);
    if (!activateScreen(name)) return false;
    navigationState.currentScreenName = name;
    // Auto-refresh when on dashboard
    if (name === 'dashboard' && walletSession.hasWallet()) {
        startAutoRefresh();
    } else {
        stopAutoRefresh();
    }
    return true;
}

export function navigateBack(fallback) {
    const current = visibleScreenName(navigationState.currentScreenName || 'welcome');
    const defaultTarget = fallback || (walletSession.hasWallet() ? 'dashboard' : 'welcome');
    let target = defaultTarget;
    while (navigationState.screenHistory.length > 0) {
        const candidate = navigationState.screenHistory.pop();
        if (candidate && candidate !== current && document.getElementById(`screen-${candidate}`)) {
            target = candidate;
            break;
        }
    }
    closeGearMenu();
    return showScreen(target, { recordHistory: false });
}

export function navigateHome() {
    navigationState.screenHistory.length = 0;
    closeGearMenu();
    const target = walletSession.hasWallet() ? 'dashboard' : 'welcome';
    return showScreen(target, { recordHistory: false });
}

export function clearNavigationHistory() {
    navigationState.screenHistory.length = 0;
}
export function showLoading(msg) {
    const presentation = loadingPresentation(msg || 'Loading…');
    const title = byId('loading-title');
    const detail = byId('loading-msg');
    const note = byId('loading-note');
    if (title) title.textContent = presentation.title;
    if (detail) detail.textContent = presentation.detail;
    if (note) note.textContent = presentation.note;
    byId('loading').classList.remove('hidden');
}
export function hideLoading() {
    byId('loading').classList.add('hidden');
}
export function setStatus(state, label) {
    const dot = document.querySelector('#status-dot .dot');
    const lbl = document.querySelector('#status-dot .label');
    const networkTag = document.querySelector('#status-dot .network-tag');
    if (!dot || !lbl) return;
    dot.className = `dot ${state}`;
    lbl.textContent = label;
    if (!networkTag) return;
    const isTestnet = networkState.network !== 'mainnet';
    networkTag.textContent = isTestnet ? `[${networkState.network.toUpperCase()}]` : '';
    networkTag.classList.toggle('hidden', !isTestnet);
}
export function toggleGearMenu() {
    // Retained for compatibility with older callers. The settings cog now opens
    // Node Connection directly and no longer owns a secondary gear menu.
    return false;
}
export function closeGearMenu() {
    byId('btn-header-settings')?.classList.remove('active');
}
