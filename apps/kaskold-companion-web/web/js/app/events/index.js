// KasKold Companion Web — app/events/index
// Explicit event registration; no shared namespace mutation.

import { bindCoreEvents } from './system/core.js';
import { bindOracleEvents } from './contracts/oracle.js';
import { bindStealthEvents } from './transactions/stealth.js';
import { bindCovenantCreationEvents } from './contracts/covenant_creation.js';
import { bindCovenantActionsEvents } from './contracts/covenant_actions.js';
import { bindCovenantSpecializedEvents } from './contracts/covenant_specialized.js';
import { bindVaultRecoveryEvents } from './contracts/vault_recovery_events.js';
import { bindCovenantLoadingEvents } from './contracts/covenant_loading.js';
import { bindTransactionsEvents } from './transactions/transactions.js';
import { bindWalletSettingsEvents } from './wallet/wallet_settings.js';
import { bindPortfolioEvents } from '../../features/portfolio/index.js';

function bindSafely(name, binder, failures) {
    try {
        binder();
    } catch (error) {
        failures.push({ name, error });
        console.error(`Companion event binding failed (${name}):`, error);
    }
}

export function bindEvents() {
    const failures = [];
    bindSafely('core', () => { bindCoreEvents(); }, failures);
    bindSafely('oracle', () => { bindOracleEvents(); }, failures);
    bindSafely('stealth', () => { bindStealthEvents(); }, failures);
    bindSafely('covenant creation', () => { bindCovenantCreationEvents(); }, failures);
    bindSafely('covenant actions', () => { bindCovenantActionsEvents(); }, failures);
    bindSafely('specialized covenants', () => { bindCovenantSpecializedEvents(); }, failures);
    bindSafely('tagged vault and recovery', () => { bindVaultRecoveryEvents(); }, failures);
    bindSafely('covenant loading', () => { bindCovenantLoadingEvents(); }, failures);
    bindSafely('transactions', () => { bindTransactionsEvents(); }, failures);
    bindSafely('settings and wallet', () => { bindWalletSettingsEvents(); }, failures);
    bindSafely('portfolio', () => { bindPortfolioEvents(); }, failures);
    return failures;
}
