import { stealthState } from '../../boundary.js';
// KasKold Companion Web — features/stealth/index/scanning/catch_up
import { byId } from '../../../../core/dom.js';
import { exactUnsignedJsonField } from '../../../../core/exact.js';
import { KSTL_SUBNET_HEX, STEALTH_LOOKBACK_BLUE_SCORE, STEALTH_MAX_WINDOWS } from '../config.js';

// ─── Stealth REST catch-up (indexer-backed, survives pruning) ───
// Current tip blue score via the indexer: /info/blockdag gives the sink hash,
// /blocks/{sink} gives header.blueScore. (virtualDaaScore is NOT the blue score.)
async function stealthGetTipBlueScore(apiBase) {
    // One GET: virtual selected-parent (sink) blue score -> {"blueScore": N}.
    try {
        const response = await fetch(apiBase + '/info/virtual-chain-blue-score',
            { signal: AbortSignal.timeout(10000) });
        const raw = await response.text();
        const bs = exactUnsignedJsonField(raw, 'blueScore', 'virtual-chain blueScore');
        if (bs > 0n) return bs;
    } catch (e) { console.log('[Companion] vc-blue-score failed, falling back to sink block:', e); }
    // Fallback: sink hash from /info/blockdag -> block header blueScore.
    const dag = await (await fetch(apiBase + '/info/blockdag', { signal: AbortSignal.timeout(10000) })).json();
    const sink = dag && dag.sink;
    if (!sink) throw new Error('no sink in /info/blockdag');
    const blockResponse = await fetch(apiBase + '/blocks/' + sink, { signal: AbortSignal.timeout(10000) });
    const blockRaw = await blockResponse.text();
    const bs2 = exactUnsignedJsonField(blockRaw, 'blueScore', 'sink blueScore');
    if (bs2 === 0n) throw new Error('no blueScore for sink block');
    return bs2;
}

const CATCH_UP_MAX_RETRY = 4;
const STEP_DONE = 'done';   // window finished (counted toward progress)
const STEP_RETRY = 'retry'; // back off and try the same window again
const STEP_STOP = 'stop';   // window abandoned without counting as done

const sleep = ms => new Promise(res => setTimeout(res, ms));

// Logs at most one failure per catch-up run so a throttled endpoint stays quiet.
function reportFirstFailure(state, ...args) {
    if (state.reported) return;
    state.reported = true;
    console.log(...args);
}

function buildNewestFirstWindows(tip, startBs) {
    // Build 100-wide [gte, lt) windows up to the cap, NEWEST FIRST so a recent
    // announcement lands in the first batch instead of after the whole walk.
    const wins = [];
    for (let hi = tip + 1n; hi > startBs && wins.length < STEALTH_MAX_WINDOWS; hi -= 100n) {
        const lo = hi - 100n > startBs ? hi - 100n : startBs;
        wins.push([lo, hi]);
    }
    return wins;
}

function fetchWindow(searchUrl, gte, lt) {
    return fetch(searchUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        // Build the JSON integer literals from decimal BigInt strings instead of
        // routing consensus blue scores through JavaScript Number.
        body: `{"acceptingBlueScores":{"gte":${gte.toString()},"lt":${lt.toString()}}}`,
        signal: AbortSignal.timeout(15000)
    });
}

// Honor Retry-After if the server sends it, else exponential
// backoff with jitter: ~0.5s, 1s, 2s, 4s (+ up to 250ms).
function throttleBackoffMs(response, attempt) {
    const ra = parseInt(response.headers.get('retry-after') || '0', 10);
    return ra > 0
        ? ra * 1000
        : (500 * Math.pow(2, attempt)) + Math.floor(Math.random() * 250);
}

// Lane payload = ver(0x01) || R(32) || view_tag(1) => 68 hex, R at offset 1 byte.
// Returns the R hex for a KSTL lane transaction, or null when it is not one.
function laneRFromTx(tx) {
    if ((tx.subnetwork_id || '').toLowerCase() !== KSTL_SUBNET_HEX) return null;
    let pl = (tx.payload || '').toLowerCase();
    if (pl.startsWith('0x')) pl = pl.slice(2);
    if (pl.length < 68 || pl.slice(0, 2) !== '01') return null;
    const rHex = pl.slice(2, 66);
    return /^0+$/.test(rHex) ? null : rHex;
}

function collectLaneRs(txs, state) {
    if (!Array.isArray(txs)) return;
    for (const tx of txs) {
        const rHex = laneRFromTx(tx);
        if (rHex !== null && !state.seen.has(rHex)) {
            state.seen.add(rHex);
            state.foundR.push(rHex);
        }
    }
}

async function handleThrottled(state, response, gte, attempt) {
    if (attempt < CATCH_UP_MAX_RETRY) {
        await sleep(throttleBackoffMs(response, attempt));
        return STEP_RETRY;
    }
    reportFirstFailure(state, '[Companion] tx-search throttled (HTTP ' + response.status + ') at gte=' + gte + ' after ' + CATCH_UP_MAX_RETRY + ' retries');
    return STEP_STOP;
}

async function handleFetchError(state, gte, error, attempt) {
    if (attempt < CATCH_UP_MAX_RETRY) {
        await sleep(500 * Math.pow(2, attempt));
        return STEP_RETRY;
    }
    reportFirstFailure(state, '[Companion] tx-search fetch failed at gte=' + gte + ':', error);
    return STEP_DONE;
}

async function attemptWindow(state, searchUrl, gte, lt, attempt) {
    const response = await fetchWindow(searchUrl, gte, lt);
    if (response.status === 429 || response.status === 503) {
        return handleThrottled(state, response, gte, attempt);
    }
    if (!response.ok) {
        reportFirstFailure(state, '[Companion] tx-search HTTP ' + response.status + ' at gte=' + gte);
        return STEP_STOP;
    }
    collectLaneRs(await response.json(), state);
    return STEP_DONE;
}

// Retry a throttled window instead of silently dropping it. 429 (and 503)
// are transient: back off and retry a few times. A dropped window means a
// missed R, so we spend a little wall time rather than lose a payment.
async function runOne(state, searchUrl, gte, lt) {
    for (let attempt = 0; attempt <= CATCH_UP_MAX_RETRY; attempt++) {
        let step;
        try {
            step = await attemptWindow(state, searchUrl, gte, lt, attempt);
        } catch (e) {
            step = await handleFetchError(state, gte, e, attempt);
        }
        if (step === STEP_RETRY) continue;
        if (step === STEP_STOP) return;
        break;
    }
    state.done++;
}

function updateScanStatus(done, total, rCount) {
    try {
        byId('stealth-scan-status').textContent =
            'Scanning lane… ' + done + '/' + total + ' windows, ' + rCount + ' R';
    } catch { /* The status line may not be on screen. */ }
}

function appendIfNewR(rHex) {
    const list = stealthState.stealthAnnouncementsR;
    return rHex.length === 64 && list.indexOf(rHex) === -1 && list.length < 64;
}

// Pushes newly found R values into the global list; true when any were added.
function appendNewRs(foundR) {
    let added = false;
    for (const rHex of foundR) {
        if (appendIfNewR(rHex)) {
            stealthState.stealthAnnouncementsR.push(rHex);
            added = true;
        }
    }
    return added;
}

function revealLoadedRs() {
    try {
        const list = byId('stealth-r-list');
        if (list) list.textContent = stealthState.stealthAnnouncementsR.length + ' R value(s) loaded';
        const qrBtn = byId('btn-stealth-show-scan-qr');
        if (qrBtn) qrBtn.classList.remove('hidden');
    } catch { /* The panel may not be on screen. */ }
}

// Surface R as soon as a batch finds it: push new ones to the global
// list and reveal the device-QR button, so the user can proceed without
// waiting for the whole walk to finish.
function surfaceFoundRs(foundR) {
    if (foundR.length && appendNewRs(foundR)) revealLoadedRs();
}

// 'stealthRestCatchUp' walks the lookback range in fixed-size windows.
// Keep only KSTL-subnetwork txs and read R from the payload.
// Survives node pruning. Returns a deduped 64-hex R list.
// No self-hosted indexer: the public api-tn10 endpoint caps each range at 100
// blue score (TX_SEARCH_BS_LIMIT) and returns accepted txs with subnetwork_id +
// payload inline.
export async function stealthRestCatchUp(apiBase) {
    const tip = await stealthGetTipBlueScore(apiBase);
    const startBs = tip > STEALTH_LOOKBACK_BLUE_SCORE ? tip - STEALTH_LOOKBACK_BLUE_SCORE : 0n;
    // subnetwork_id MUST be in fields= or the lane filter has nothing to match.
    const searchUrl = apiBase +
        '/transactions/search?fields=transaction_id,subnetwork_id,payload,accepting_block_blue_score&resolve_previous_outpoints=no';
    const wins = buildNewestFirstWindows(tip, startBs);
    console.log('[Companion] REST catch-up: tip=' + tip + ' start=' + startBs +
        ' windows=' + wins.length);
    const state = { foundR: [], seen: new Set(), done: 0, reported: false };
    const CONC = 3; // low concurrency: reliable on the public endpoint, not a hammer
    for (let i = 0; i < wins.length; i += CONC) {
        await Promise.all(wins.slice(i, i + CONC).map(w => runOne(state, searchUrl, w[0], w[1])));
        // Small gap between batches so a steady scan stays well under the
        // endpoint's rate limit even when no 429 has fired yet.
        if (i + CONC < wins.length) { await sleep(150); }
        updateScanStatus(state.done, wins.length, state.foundR.length);
        surfaceFoundRs(state.foundR);
    }
    console.log('[Companion] REST catch-up done: ' + wins.length + ' windows, found ' + state.foundR.length + ' R');
    return state.foundR;
}
