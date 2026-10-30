// Companion logs carry addresses, amounts and transaction summaries. They stay
// silent unless the user turns on diagnostics for this browser profile.
const DEBUG_KEY = 'kaskold.debug';
const GATED_METHODS = ['log', 'info', 'debug', 'warn', 'error'];

function diagnosticsEnabled(storage) {
    try {
        return storage.getItem(DEBUG_KEY) === '1';
    } catch (_) {
        return false;
    }
}

export function gateConsole(target, storage) {
    if (diagnosticsEnabled(storage)) return;
    for (const method of GATED_METHODS) target[method] = () => {};
}
