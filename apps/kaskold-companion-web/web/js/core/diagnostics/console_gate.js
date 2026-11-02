// @ts-check
// Companion logs carry addresses, amounts and transaction summaries. They stay
// silent unless the user turns on diagnostics for this browser profile.
const DEBUG_KEY = 'kaskold.debug';
/** @typedef {'log' | 'info' | 'debug' | 'warn' | 'error'} GatedMethod */
/** @type {ReadonlyArray<GatedMethod>} */
const GATED_METHODS = ['log', 'info', 'debug', 'warn', 'error'];

/**
 * @param {Pick<Storage, 'getItem'>} storage
 */
function diagnosticsEnabled(storage) {
    try {
        return storage.getItem(DEBUG_KEY) === '1';
    } catch {
        return false;
    }
}

/**
 * @param {Pick<Console, GatedMethod>} target
 * @param {Pick<Storage, 'getItem'>} storage
 */
export function gateConsole(target, storage) {
    if (diagnosticsEnabled(storage)) return;
    for (const method of GATED_METHODS) target[method] = () => {};
}
