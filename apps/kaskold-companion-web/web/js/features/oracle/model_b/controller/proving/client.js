export function createOracleProverClient(baseUrl) {
    const base = (baseUrl || '').replace(/\/+$/, '');
    return async function get(path) {
        const response = await fetch(base + path, { signal: AbortSignal.timeout(15000) });
        let body = null;
        try { body = await response.json(); } catch { /* A body that is not JSON leaves the status code to report the failure. */ }
        return { ok: response.ok, status: response.status, body };
    };
}
