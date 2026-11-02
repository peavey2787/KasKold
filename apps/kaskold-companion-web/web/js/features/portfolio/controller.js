// @ts-check
import { hideLoading, showLoading, showScreen } from '../../app/navigation.js';
import { elementById } from '../../core/dom.js';
import { toast } from '../../core/ui/toast.js';
import { parseKas, usdToMicro } from './exact_money.js';
import { exportPortfolioCsv, parsePortfolioCsv } from './csv.js';
import { fetchCurrentPriceMicro, loadHistoricalPrices, historicalPriceAt } from './pricing.js';
import {
    createAccount,
    deleteAccount,
    deleteTransaction,
    loadPortfolioStore,
    newId,
    renameAccount,
    savePortfolioStore,
    upsertTransaction,
} from './repository.js';
import { renderPortfolio } from './render.js';
import { fetchWalletHistory, walletHistoryFetchMode } from './wallet_history.js';

/** @type {Record<string, any>} */
const state = {
    store: loadPortfolioStore(),
    selectedAccountId: '',
    mode: 'overview',
    rangeDays: 90,
    livePriceMicro: null,
    historicalPrices: [],
    editorOpen: false,
    editingEntry: null,
    accountEditorMode: null,
    pendingDeleteAccountId: null,
    pendingDeleteTransactionId: null,
    visibleEntries() {
        return this.selectedAccountId
            ? this.store.transactions.filter(entry => entry.portfolioId === this.selectedAccountId)
            : this.store.transactions;
    },
};

function root() { return elementById('portfolio-root'); }
function rerender() { renderPortfolio(root(), state); }

async function refreshPrices() {
    try { state.livePriceMicro = await fetchCurrentPriceMicro(); } catch { state.livePriceMicro = null; }
    try { state.historicalPrices = await loadHistoricalPrices(state.rangeDays); } catch { state.historicalPrices = []; }
    rerender();
}

export function showPortfolio() {
    showScreen('portfolio');
    rerender();
    void refreshPrices();
}

function openAccountEditor(mode) {
    if (mode === 'rename' && !state.selectedAccountId) return;
    state.accountEditorMode = mode;
    state.pendingDeleteAccountId = null;
    state.pendingDeleteTransactionId = null;
    rerender();
}

function saveAccount(form) {
    const name = String(new FormData(form).get('name') || '');
    try {
        if (state.accountEditorMode === 'rename') {
            renameAccount(state.store, state.selectedAccountId, name);
        } else {
            const account = createAccount(state.store, name);
            state.selectedAccountId = account.id;
        }
        state.accountEditorMode = null;
        rerender();
    } catch (error) {
        toast(error.message, 'error');
    }
}

function requestAccountDelete() {
    if (!state.selectedAccountId) return;
    state.pendingDeleteAccountId = state.selectedAccountId;
    state.accountEditorMode = null;
    rerender();
}

function confirmAccountDelete() {
    if (!state.pendingDeleteAccountId) return;
    deleteAccount(state.store, state.pendingDeleteAccountId);
    state.selectedAccountId = '';
    state.pendingDeleteAccountId = null;
    state.editorOpen = false;
    state.editingEntry = null;
    rerender();
}

function startEditor(transactionId = '') {
    if (!state.selectedAccountId) { toast('Select a portfolio first', 'info'); return; }
    state.editingEntry = transactionId
        ? state.store.transactions.find(entry => entry.id === transactionId) || null
        : null;
    state.editorOpen = true;
    state.pendingDeleteTransactionId = null;
    rerender();
}

function parseFormTimestamp(data) {
    const timestampMs = Date.parse(String(data.get('timestamp') || ''));
    if (!Number.isFinite(timestampMs)) throw new Error('Choose a valid date and time');
    return timestampMs;
}

function parseFormPriceMicroUsd(data, timestampMs) {
    const priceText = String(data.get('priceUsd') || '').trim();
    const autoPrice = historicalPriceAt(state.historicalPrices, timestampMs);
    const price = priceText ? usdToMicro(priceText, 'KAS price') : autoPrice;
    if (price <= 0n) throw new Error('Enter a KAS price or refresh historical prices');
    return price;
}

function parseForm(form) {
    const data = new FormData(form);
    const timestampMs = parseFormTimestamp(data);
    const price = parseFormPriceMicroUsd(data, timestampMs);
    return {
        id: form.dataset.transactionId || newId(),
        portfolioId: state.selectedAccountId,
        type: String(data.get('type') || 'Buy'),
        kasSompi: parseKas(data.get('kasAmount')).toString(),
        priceMicroUsd: price.toString(),
        feeMicroUsd: usdToMicro(data.get('feeUsd') || '0', 'fee').toString(),
        timestampMs,
        notes: String(data.get('notes') || '').trim().slice(0, 500),
        sourceTxId: state.editingEntry?.sourceTxId || null,
        createdAt: state.editingEntry?.createdAt || Date.now(),
    };
}

function findCsvSourceEntry(entry) {
    if (!entry.sourceTxId) return null;
    return state.store.transactions.find(candidate =>
        candidate.portfolioId === state.selectedAccountId && candidate.sourceTxId === entry.sourceTxId) || null;
}

function enrichExistingTransaction(existing, entry) {
    if (entry.notes) existing.notes = entry.notes;
    if (BigInt(entry.priceMicroUsd || '0') > 0n) existing.priceMicroUsd = entry.priceMicroUsd;
    if (BigInt(entry.feeMicroUsd || '0') > 0n) existing.feeMicroUsd = entry.feeMicroUsd;
}

function mergeCsvEntries(entries) {
    let added = 0;
    let enriched = 0;
    for (const entry of entries) {
        const existing = findCsvSourceEntry(entry);
        if (existing) { enrichExistingTransaction(existing, entry); enriched += 1; }
        else { state.store.transactions.push(entry); added += 1; }
    }
    return { added, enriched };
}

function csvImportSummary({ added, enriched }) {
    const addedText = `Imported ${added} CSV transaction${added === 1 ? '' : 's'}`;
    if (!enriched) return addedText;
    return `${addedText}; enriched ${enriched} on-chain transaction${enriched === 1 ? '' : 's'}`;
}

async function importCsvFile(file) {
    const entries = parsePortfolioCsv(await file.text(), state.selectedAccountId);
    const counts = mergeCsvEntries(entries);
    savePortfolioStore(state.store);
    rerender();
    toast(csvImportSummary(counts), 'ok');
}

function saveTransactionFromForm(form) {
    try {
        upsertTransaction(state.store, parseForm(form));
        state.editorOpen = false;
        state.editingEntry = null;
        rerender();
        toast('Portfolio transaction saved', 'ok');
    } catch (error) {
        toast(error.message, 'error');
    }
}

function requestTransactionDelete(id) {
    state.pendingDeleteTransactionId = id;
    state.editorOpen = false;
    state.editingEntry = null;
    rerender();
}

function confirmTransactionDelete() {
    if (!state.pendingDeleteTransactionId) return;
    deleteTransaction(state.store, state.pendingDeleteTransactionId);
    state.pendingDeleteTransactionId = null;
    rerender();
}

async function fetchHistory() {
    if (!state.selectedAccountId) { toast('Select a portfolio first', 'info'); return; }
    const requestedMode = walletHistoryFetchMode(state.store, state.selectedAccountId);
    showLoading(requestedMode === 'deep' ? 'Deep scanning wallet history...' : 'Fetching new wallet activity...');
    try {
        const result = await fetchWalletHistory(state.store, state.selectedAccountId);
        state.store.transactions.push(...result.entries);
        const account = state.store.accounts.find(candidate => candidate.id === state.selectedAccountId);
        if (account) account.walletHistory = result.sync;
        savePortfolioStore(state.store);
        rerender();
        const suffix = result.entries.length === 1 ? '' : 's';
        if (result.mode === 'deep') {
            toast(result.entries.length
                ? `Deep scan fetched ${result.entries.length} wallet transaction${suffix}`
                : 'Deep scan complete — no wallet transactions found', 'ok', 4000);
        } else {
            toast(result.entries.length
                ? `Fetched ${result.entries.length} new wallet transaction${suffix}`
                : 'Wallet history is up to date', 'ok', 3500);
        }
    } catch (error) {
        toast(`Wallet history fetch failed: ${error.message}`, 'error', 5000);
    } finally {
        hideLoading();
    }
}

function pickCsv() {
    if (!state.selectedAccountId) { toast('Select a portfolio first', 'info'); return; }
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.csv,text/csv,text/plain';
    input.onchange = async () => {
        const [file] = input.files || [];
        if (!file) return;
        try {
            await importCsvFile(file);
        } catch (error) {
            toast(`CSV import failed: ${error.message}`, 'error', 5000);
        }
    };
    input.click();
}

function exportCsv() {
    if (!state.selectedAccountId) { toast('Select a portfolio first', 'info'); return; }
    const entries = state.visibleEntries();
    if (!entries.length) { toast('No portfolio transactions to export', 'info'); return; }
    const blob = new Blob([exportPortfolioCsv(entries)], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'Companion-Portfolio-Transactions.csv';
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function actionValue(action, prefix) {
    return action.startsWith(`${prefix}:`) ? action.slice(prefix.length + 1) : null;
}

function owningForm(node) {
    let current = node || null;
    while (current) {
        if (current.tagName === 'FORM') return current;
        current = current.parentElement || null;
    }
    return null;
}

function handleSubmitButton(target) {
    const action = target?.dataset?.action || '';
    if (action !== 'submit-account' && action !== 'submit-transaction') return false;
    const form = target.form || owningForm(target);
    if (!form) {
        toast('Unable to save: form is unavailable', 'error');
        return true;
    }
    if (action === 'submit-account') saveAccount(form);
    else saveTransactionFromForm(form);
    return true;
}

function cancelInlineAction() {
    state.accountEditorMode = null;
    state.pendingDeleteAccountId = null;
    state.pendingDeleteTransactionId = null;
    rerender();
}

const SIMPLE_ACTION_HANDLERS = new Map([
    ['new-account', () => openAccountEditor('new')],
    ['rename-account', () => openAccountEditor('rename')],
    ['delete-account', () => requestAccountDelete()],
    ['confirm-account-delete', () => confirmAccountDelete()],
    ['confirm-transaction-delete', () => confirmTransactionDelete()],
    ['cancel-inline-action', () => cancelInlineAction()],
    ['new-transaction', () => startEditor()],
    ['cancel-editor', () => { state.editorOpen = false; state.editingEntry = null; rerender(); }],
    ['refresh-price', () => refreshPrices()],
    ['import-wallet-history', () => pickCsv()],
    ['fetch-wallet-history', () => fetchHistory()],
    ['import-csv', () => pickCsv()],
    ['export-csv', () => exportCsv()],
]);

async function handleButton(action) {
    const mode = actionValue(action, 'mode');
    if (mode) { state.mode = mode; rerender(); return; }
    const range = actionValue(action, 'range');
    if (range) { state.rangeDays = Number.parseInt(range, 10); await refreshPrices(); return; }
    const edit = actionValue(action, 'edit');
    if (edit) { startEditor(edit); return; }
    const remove = actionValue(action, 'delete');
    if (remove) { requestTransactionDelete(remove); return; }
    const handler = SIMPLE_ACTION_HANDLERS.get(action);
    if (handler) await handler();
}

export function bindPortfolioEvents() {
    const container = root();
    container.addEventListener('click', event => {
        const target = /** @type {HTMLButtonElement | null} */ (/** @type {HTMLElement} */ (event.target).closest('[data-action]'));
        if (!target || target.tagName === 'SELECT' || target.tagName === 'FORM') return;
        if (target.type === 'submit') {
            if (handleSubmitButton(target)) event.preventDefault();
            return;
        }
        event.preventDefault();
        void handleButton(target.dataset.action || '');
    });
    container.addEventListener('change', event => {
        const field = /** @type {HTMLInputElement} */ (event.target);
        if (field.dataset.action !== 'select-account') return;
        state.selectedAccountId = field.value;
        state.editorOpen = false;
        state.editingEntry = null;
        state.accountEditorMode = null;
        state.pendingDeleteAccountId = null;
        state.pendingDeleteTransactionId = null;
        rerender();
    });
    container.addEventListener('submit', event => {
        event.preventDefault();
        const form = /** @type {HTMLElement} */ (event.target);
        if (form.dataset.action === 'transaction-form') saveTransactionFromForm(form);
        else if (form.dataset.action === 'account-form') saveAccount(form);
    });
}
