// @ts-check
// Stable covenant_creation event composition.
import { registerCreationNavigation } from './covenant_creation/creation_navigation.js';
import { registerClaimSharing } from './covenant_creation/claim_sharing.js';
import { registerCreationOptions } from './covenant_creation/creation_options.js';
import { registerResultActions } from './covenant_creation/result_actions.js';
import { registerWithdrawal } from './covenant_creation/withdrawal.js';

export function bindCovenantCreationEvents() {
    registerCreationNavigation();
    registerClaimSharing();
    registerCreationOptions();
    registerResultActions();
    registerWithdrawal();
}
