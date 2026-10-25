// Minimal test registry for suites that run inside a real browser DOM.

const registered = [];

/** Register a named test; the body may be async. */
export function test(name, body) {
    registered.push({ name, body });
}

/** Install the WASM export fakes a test needs; unfaked exports throw. */
export function fakeWasm(fakes) {
    globalThis.__kaskoldWasmFakes = { ...fakes };
}

export function assert(condition, message) {
    if (!condition) throw new Error(message || 'assertion failed');
}

export function assertEqual(actual, expected, message) {
    if (!Object.is(actual, expected)) {
        throw new Error(`${message || 'values differ'}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}

export async function assertRejects(operation, pattern, message) {
    try {
        await operation();
    } catch (error) {
        const text = String(error && error.message || error);
        if (pattern && !pattern.test(text)) throw new Error(`${message || 'wrong error'}: ${text}`);
        return error;
    }
    throw new Error(message || 'expected the operation to fail');
}

/** A detached container attached to the document for the duration of a test. */
export function mount(html = '') {
    const host = document.createElement('div');
    host.dataset.testHost = 'true';
    if (html) host.insertAdjacentHTML('afterbegin', html);
    document.body.append(host);
    return host;
}

export async function runRegisteredTests() {
    // The facade loads the runner's fake package, whose exports call fakeWasm().
    const wasm = await import('/apps/kaskold-companion-web/web/js/wasm/api.js');
    await wasm.init();
    const results = [];
    for (const { name, body } of registered) {
        globalThis.__kaskoldWasmFakes = {};
        try {
            await body();
            results.push({ name, ok: true });
        } catch (error) {
            results.push({ name, ok: false, error: String(error && error.stack || error) });
        } finally {
            for (const host of document.querySelectorAll('[data-test-host]')) host.remove();
        }
    }
    return results;
}

/**
 * Mount the real Companion application markup (the generated index.html body)
 * so screen navigation and element lookups behave as they do in production.
 */
export async function mountCompanionShell() {
    const response = await fetch('/apps/kaskold-companion-web/web/index.html', { cache: 'no-store' });
    const markup = new DOMParser().parseFromString(await response.text(), 'text/html');
    const app = markup.getElementById('app');
    if (!app) throw new Error('Companion shell has no #app element');
    const host = mount();
    host.append(document.importNode(app, true));
    return host;
}

/** A QR-shaped SVG in the exact form the WASM encoder emits. */
export function encoderSvg(size = 21) {
    const total = size + 4;
    let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${total} ${total}" shape-rendering="crispEdges"><rect width="${total}" height="${total}" fill="white"/>`;
    for (let index = 0; index < size * size; index += 3) {
        svg += `<rect x="${(index % size) + 2}" y="${Math.floor(index / size) + 2}" width="1" height="1" fill="black"/>`;
    }
    return `${svg}</svg>`;
}

/** Resolve once an image has decoded, rejecting when the browser cannot render it. */
export async function decoded(image) {
    await image.decode();
    if (!(image.naturalWidth > 0)) throw new Error('image decoded with zero width');
    return image;
}
