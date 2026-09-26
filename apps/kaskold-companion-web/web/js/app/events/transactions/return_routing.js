import { navigationState, walletSession } from '../../state/index.js';
import { showScreen } from '../../navigation.js';
import { covReturnAfterBroadcast } from '../../../features/covenants/scanning_and_swap.js';

export function takeTransactionReturnScreen() {
    const screen = navigationState._broadcastReturnScreen;
    navigationState._broadcastReturnScreen = null;
    return screen;
}

export function showTransactionReturnScreen(screen, { restoreCovenant = true } = {}) {
    // A transaction return is navigation *back* to its owning workflow, not a
    // new forward navigation edge. Recording the broadcast/review screen here
    // makes the next Back action bounce forward again.
    showScreen(screen, { recordHistory: false });
    if (restoreCovenant && screen === 'covenant') covReturnAfterBroadcast();
    return screen;
}

export function returnFromTransaction({
    defaultScreen = 'dashboard',
    restoreCovenant = true,
} = {}) {
    return showTransactionReturnScreen(
        takeTransactionReturnScreen() || defaultScreen,
        { restoreCovenant },
    );
}

export function walletAwareDefaultScreen() {
    return walletSession.hasWallet() ? 'dashboard' : 'welcome';
}
