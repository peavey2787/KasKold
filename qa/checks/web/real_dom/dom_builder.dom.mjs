// Hostile values must stay text when they flow through the DOM builders, and
// generated QR SVG must render only as an inert image.

import { assert, assertEqual, decoded, encoderSvg, mount, test } from './harness.mjs';

const JS = '/apps/kaskold-companion-web/web/js';
const HOSTILE = '<img src=x onerror="window.__pwned=1"><script>window.__pwned=1</script><b>fake balance</b>';

test('element builder renders hostile strings as literal text', async () => {
    const { el, setContent, span } = await import(`${JS}/core/ui/elements.js`);
    const host = mount();
    setContent(host, el('div', { className: 'row' }, HOSTILE, span('u-text-teal', HOSTILE)));
    await new Promise(resolve => setTimeout(resolve, 20));
    assertEqual(host.querySelectorAll('img, script, b').length, 0, 'no parsed elements');
    assertEqual(host.textContent, HOSTILE + HOSTILE);
    assertEqual(globalThis.__pwned, undefined, 'no handler ran');
});

test('element builder keeps only http(s) links and rejects unknown attributes', async () => {
    const { el } = await import(`${JS}/core/ui/elements.js`);
    assertEqual(el('a', { attrs: { href: 'javascript:alert(1)' } }).hasAttribute('href'), false);
    assertEqual(el('a', { attrs: { href: 'data:text/html,x' } }).hasAttribute('href'), false);
    assertEqual(el('a', { attrs: { href: 'https://explorer.kaspa.org/a' } }).getAttribute('href'), 'https://explorer.kaspa.org/a');
    let threw = false;
    try { el('div', { attrs: { onclick: 'alert(1)' } }); } catch (_) { threw = true; }
    assert(threw, 'event-handler attributes are refused');
});

test('element builder sets dataset, checked state and skips empty children', async () => {
    const { el } = await import(`${JS}/core/ui/elements.js`);
    const input = el('input', { attrs: { type: 'checkbox' }, checked: true, dataset: { utxoIdx: 3 } }, null, false, undefined);
    assertEqual(input.checked, true);
    assertEqual(input.dataset.utxoIdx, '3');
    assertEqual(input.childNodes.length, 0);
});

test('Companion QR image renders the encoder SVG and refuses other markup', async () => {
    const { qrImage } = await import(`${JS}/core/qr/svg_image.js`);
    const image = qrImage(encoderSvg(), 'test QR');
    mount().append(image);
    await decoded(image);
    assertEqual(image.alt, 'test QR');
    let threw = false;
    try { qrImage(HOSTILE); } catch (_) { threw = true; }
    assert(threw, 'non-SVG markup is refused');
});

test('SVG script cannot run when shown through the QR image path', async () => {
    const { qrImage } = await import(`${JS}/core/qr/svg_image.js`);
    const hostileSvg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4" onload="window.__svgRan=1">'
        + '<script>window.__svgRan=1</script><rect width="4" height="4"/></svg>';
    const image = qrImage(hostileSvg);
    mount().append(image);
    await decoded(image);
    assertEqual(globalThis.__svgRan, undefined, 'SVG scripts stay inert inside an image');
});

test('Vault QR helper renders an image and clears on empty output', async () => {
    const { showQrSvg } = await import('/apps/kaskold-vault-web/web/js/ui/qr.js');
    const host = mount();
    showQrSvg(host, encoderSvg(), 'vault QR');
    await decoded(host.querySelector('img'));
    showQrSvg(host, '');
    assertEqual(host.childNodes.length, 0);
});

test('scan progress shows one dot per frame and the received count', async () => {
    const { renderScanProgress } = await import(`${JS}/core/ui/scan_progress.js`);
    const host = mount();
    renderScanProgress(host, { total: 3, count: 2, bits: [true, false, true] });
    assertEqual(host.querySelectorAll('.scanner-progress-dot').length, 3);
    assertEqual(host.querySelectorAll('.scanner-progress-dot-active').length, 2);
    assert(host.textContent.includes('2 / 3 frames'));
    renderScanProgress(host, { total: 0, count: 0, bits: [] });
    assertEqual(host.querySelectorAll('.scanner-progress-dot').length, 3, 'no progress leaves the last render');
});
