import { applyScriptTerms } from '../model/script_terms.js';
import { covenantState, covenantWatcherState, oracleState } from '../../../app/state/index.js';
import { toast } from '../../../core/ui/toast.js';
import { covShowPanel } from '../generation/panel_keys.js';
import { covRenderMetaLine } from '../watch/ui/metadata.js';
import { covUpdateResultButtons } from '../watch/ui/result_buttons.js';
import { covWatcherStart, covWatcherStop } from '../watch/watcher/polling/lifecycle.js';
import { isWatchedCovenantType } from '../watch/watcher/types.js';
import { byId } from '../../../core/dom.js';

function isTxid(value) {
    return Boolean(value) && value.length === 64;
}

// Generic covenant watcher: store outpoint for BlockAdded spend detection.
function rememberBroadcastOutpoint(broadcastTxid, type) {
    if (!isTxid(broadcastTxid) || covenantWatcherState._covWatcherOutpoint || !isWatchedCovenantType(type)) return;
    covenantWatcherState._covWatcherOutpoint = { txid: broadcastTxid, index: 0 };
    console.log('[Companion] Stored covenant outpoint from broadcast:', broadcastTxid);
}

function setResultTextFields(c) {
    if (byId('cov-result-addr')) byId('cov-result-addr').textContent = c.address || '';
    if (byId('cov-result-script')) byId('cov-result-script').textContent = c.redeem_script_hex || '';
}

function setResultTxid(broadcastTxid) {
    if (!byId('cov-result-txid') || !isTxid(broadcastTxid)) return;
    byId('cov-result-txid').textContent = broadcastTxid;
    byId('cov-result-txid').onclick = () => { navigator.clipboard.writeText(broadcastTxid); toast('TX ID copied', 'ok'); };
    if (byId('cov-result-txid-wrap')) byId('cov-result-txid-wrap').style.display = '';
}

// Repopulate the result panel fields. Otherwise this leaves them
// stale/empty (briefly shows "0 KAS, not funded" if the user
// landed here right after broadcast). Mirrors what the
// active-list click handler does when loading a covenant.
function repopulateResultPanel(c, broadcastTxid) {
    applyScriptTerms(c);
    setResultTextFields(c);
    setResultTxid(broadcastTxid);
    if (byId('cov-result-extra')) covRenderMetaLine(c);
    if (byId('cov-result-balance')) {
        byId('cov-result-balance').textContent = 'Loading...';
        byId('cov-result-balance').style.display = '';
    }
}

// Restart watcher to pick up new UTXO state after broadcast.
function restartWatcher() {
    covWatcherStop();
    covenantWatcherState._covWatcherOutpoint = null;
    covWatcherStart();
}

export function covReturnAfterBroadcast() {
    // Oracle roll: re-open a LIVE card; covShowPanel -> oracleMbCardOpen restarts the 1s age tick, the 12s poll, and the block watcher.
    if (oracleState._oracleMbReturn) { oracleState._oracleMbReturn = false; covShowPanel('oracle-mb'); return; }
    const c = covenantState.lastCovenantResult;
    if (!c) { covShowPanel('menu'); return; }
    const broadcastTxid = byId('broadcast-result-txid')?.textContent?.trim() || '';
    rememberBroadcastOutpoint(broadcastTxid, c.type);
    covShowPanel('result');
    covUpdateResultButtons(c.type || '');
    repopulateResultPanel(c, broadcastTxid);
    setTimeout(() => { if (byId('btn-cov-res-balance')) byId('btn-cov-res-balance').click(); }, 500);
    restartWatcher();
}
