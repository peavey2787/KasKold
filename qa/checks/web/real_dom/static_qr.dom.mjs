// QR codes shown on the Companion's QR screen must render as images.
// Regression: SVG passed through the markup sanitizer was shown as literal text.

import { assert, assertEqual, decoded, encoderSvg, fakeWasm, mountCompanionShell, test } from './harness.mjs';

const JS = '/apps/kaskold-companion-web/web/js';

async function renderedQr() {
    const container = document.getElementById('qr-container');
    assert(container, 'qr-container exists');
    const images = container.querySelectorAll('img');
    assertEqual(images.length, 1, 'one QR image');
    assert(!container.textContent.includes('<svg'), 'no SVG source shown as text');
    await decoded(images[0]);
    return images[0];
}

test('Private Swap protocol QR renders as an image', async () => {
    await mountCompanionShell();
    fakeWasm({ generate_qr_svg_text: () => encoderSvg() });
    const ui = await import(`${JS}/features/covenants/private_swap/ui.js`);
    ui.showPrivateSwapProtocolQr('{"t":"swap"}', 'Private Swap offer', 'Scan on the counterparty');
    await renderedQr();
    assertEqual(document.getElementById('qr-display-title').textContent, 'Private Swap offer');
    assertEqual(document.getElementById('qr-frame-info').textContent, 'Scan on the counterparty');
});

test('Crowdfunding contribution invite QR renders as an image', async () => {
    await mountCompanionShell();
    fakeWasm({ generate_qr_svg_text: () => encoderSvg() });
    const { covenantState } = await import(`${JS}/app/state/index.js`);
    covenantState.lastCovenantResult = {
        type: 'crowdfund', crowdfund_role: 'contributor', campaign_id: 'ab'.repeat(16),
        address: 'kaspa:qcontribution', redeem_script_hex: '51', contributor_pubkey_hex: '11'.repeat(32),
        crowdfund_salt_hex: '22'.repeat(8),
    };
    const invites = await import(`${JS}/features/covenants/crowdfund/invites.js`);
    invites.shareCrowdfundContribution();
    await renderedQr();
});

test('Static payload QR presenter renders one image and its caption', async () => {
    await mountCompanionShell();
    fakeWasm({ generate_qr_svg_text: () => encoderSvg() });
    const presenter = await import(`${JS}/features/qr_display/static_payload.js`);
    presenter.showStaticPayloadQr('{"t":"cov-invite"}', 'Share invite', 'details');
    await renderedQr();
    assertEqual(document.getElementById('qr-frame-info').textContent, 'details');
});
