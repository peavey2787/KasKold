import { walletSession } from '../../app/state/index.js';
import { byId } from '../../core/dom.js';
import { hardenedWalletCleanup, requestWalletRuntimeReset } from './state_reset.js';

let unloadDialogBound = false;
let unloadDialogPreviousFocus = null;

export function oneTimeWalletActive() {
    return walletSession.hasWallet() && walletSession.profile() === null;
}

export function syncWalletUnloadAction() {
    const button = byId('btn-reset-wallet');
    if (!button) return;
    const oneTime = oneTimeWalletActive();
    button.textContent = 'Unload wallet';
    button.title = oneTime
        ? 'Unload this temporary wallet without deleting saved wallets'
        : 'Unload the currently loaded wallet without deleting saved wallets';
}

function closeUnloadConfirmation({ restoreFocus = true } = {}) {
    byId('wallet-unload-modal')?.classList.add('hidden');
    if (restoreFocus && unloadDialogPreviousFocus?.focus) unloadDialogPreviousFocus.focus();
    unloadDialogPreviousFocus = null;
}

function confirmUnloadWallet() {
    closeUnloadConfirmation({ restoreFocus: false });
    hardenedWalletCleanup();
    requestWalletRuntimeReset();
}

function bindUnloadDialogOnce() {
    if (unloadDialogBound) return;
    const modal = byId('wallet-unload-modal');
    const cancel = byId('btn-wallet-unload-cancel');
    const confirmButton = byId('btn-wallet-unload-confirm');
    if (!modal || !cancel || !confirmButton) return;

    cancel.onclick = () => closeUnloadConfirmation();
    confirmButton.onclick = () => confirmUnloadWallet();
    modal.onclick = event => {
        if (event.target === modal) closeUnloadConfirmation();
    };
    modal.addEventListener('keydown', event => {
        if (event.key !== 'Escape') return;
        event.preventDefault();
        closeUnloadConfirmation();
    });
    unloadDialogBound = true;
}

export function resetWallet() {
    if (!walletSession.hasWallet()) return false;
    bindUnloadDialogOnce();

    const modal = byId('wallet-unload-modal');
    const message = byId('wallet-unload-message');
    const cancel = byId('btn-wallet-unload-cancel');
    if (!modal || !message || !cancel) return false;

    message.textContent = oneTimeWalletActive()
        ? 'The temporary wallet and any in-progress transaction or session state will be discarded.'
        : 'Any in-progress transaction or session state will be discarded. Your saved wallet will remain available in Manage Wallets.';
    unloadDialogPreviousFocus = document.activeElement;
    modal.classList.remove('hidden');
    cancel.focus();
    return true;
}
