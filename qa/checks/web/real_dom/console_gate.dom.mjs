// Companion console output carries wallet data, so it must be silent by default.

import { assertEqual, test } from './harness.mjs';

const GATE = '/apps/kaskold-companion-web/web/js/core/diagnostics/console_gate.js';

function recordingConsole() {
    const calls = [];
    const target = {};
    for (const method of ['log', 'info', 'debug', 'warn', 'error']) {
        target[method] = (...args) => calls.push([method, ...args]);
    }
    return { target, calls };
}

function logEveryLevel(target) {
    for (const method of ['log', 'info', 'debug', 'warn', 'error']) target[method]('kaspa:secret-summary');
}

test('console stays silent without the diagnostics flag', async () => {
    const { gateConsole } = await import(GATE);
    const { target, calls } = recordingConsole();
    gateConsole(target, { getItem: () => null });
    logEveryLevel(target);
    assertEqual(calls.length, 0);
});

test('console stays silent when storage is unavailable or holds another value', async () => {
    const { gateConsole } = await import(GATE);
    for (const storage of [{ getItem() { throw new Error('blocked'); } }, { getItem: () => 'true' }]) {
        const { target, calls } = recordingConsole();
        gateConsole(target, storage);
        logEveryLevel(target);
        assertEqual(calls.length, 0);
    }
});

test('the diagnostics flag keeps every console level', async () => {
    const { gateConsole } = await import(GATE);
    const { target, calls } = recordingConsole();
    gateConsole(target, { getItem: key => (key === 'kaskold.debug' ? '1' : null) });
    logEveryLevel(target);
    assertEqual(calls.length, 5);
});

test('Companion CSP blocks inline styles and remote cleartext sockets', async () => {
    const html = await (await fetch('/apps/kaskold-companion-web/web/index.html')).text();
    const parsed = new DOMParser().parseFromString(html, 'text/html');
    const csp = parsed.querySelector('meta[http-equiv="Content-Security-Policy"]').content;
    const directive = name => csp.split(';').map(part => part.trim()).find(part => part.startsWith(`${name} `)) || '';
    assertEqual(directive('style-src'), "style-src 'self'");
    assertEqual(directive('font-src'), "font-src 'self'");
    assertEqual(directive('connect-src').split(' ').includes('ws:'), false);
    assertEqual(parsed.querySelectorAll('[style], style').length, 0);
});
