// PSKT review screen: destination, totals, inspect toggle, and next-action buttons.
// Drives the real openPsktReview with a faked pskt_summary payload.

import { assert, assertEqual, fakeWasm, mountCompanionShell, test } from './harness.mjs';

const JS = '/apps/kaskold-companion-web/web/js';
const DEST = 'kaspa:qdestination';
const CHANGE = 'kaspa:qmultisigchange';
const KAS = 100000000;

function input(overrides = {}) {
    return {
        script_kind: 'multisig', amount_sompi: KAS, prev_tx_id: 'aa'.repeat(32), prev_index: 0,
        sigs_present: 0, multisig_m: 2, multisig_n: 3, ...overrides,
    };
}

function output(address, amount_sompi) {
    return { script_kind: 'p2pk', address, amount_sompi };
}

function summary(overrides = {}) {
    return {
        format: 'pskb', tx_version: 0, input_count: 1, output_count: 1, fee_sompi: 1000,
        total_in_sompi: KAS, total_out_sompi: KAS - 1000, inputs: [input()],
        outputs: [output(DEST, KAS - 1000)], finalize_ready: false, ...overrides,
    };
}

async function openReview(payload, context = null) {
    await mountCompanionShell();
    fakeWasm({ pskt_summary: () => JSON.stringify(payload) });
    const review = await import(`${JS}/features/transactions/pskt_multisig/review.js`);
    return review.openPsktReview('00', context);
}

const text = (id) => document.getElementById(id).textContent;
const hidden = (id) => document.getElementById(id).classList.contains('hidden');
const sendContext = { kind: 'multisig-send', destinationAddress: DEST };

test('Review shows the single external destination address', async () => {
    await openReview(summary(), sendContext);
    assertEqual(text('pskt-to-address'), DEST);
});

test('Review summarizes multiple external destinations without listing them', async () => {
    await openReview(summary({
        output_count: 2,
        outputs: [output(DEST, 1000), output('kaspa:qsecond', 2000)],
    }));
    assertEqual(text('pskt-to-address'), '2 destinations — Inspect to view');
});

test('Review reports no external address when every output is change', async () => {
    await openReview(summary({ outputs: [output(CHANGE, KAS - 1000)] }), sendContext);
    assertEqual(text('pskt-to-address'), 'No external address — Inspect to view');
});

test('Inspect details start hidden and toggle with aria-expanded', async () => {
    await openReview(summary(), sendContext);
    const button = document.getElementById('btn-pskt-inspect');
    assert(hidden('pskt-inspect-details'), 'details hidden on open');
    assertEqual(button.getAttribute('aria-expanded'), 'false');

    button.click();
    assert(!hidden('pskt-inspect-details'), 'details shown after first click');
    assertEqual(button.getAttribute('aria-expanded'), 'true');
    assertEqual(button.textContent, 'Hide details');

    button.click();
    assert(hidden('pskt-inspect-details'), 'details hidden after second click');
    assertEqual(button.getAttribute('aria-expanded'), 'false');
    assertEqual(button.textContent, 'Inspect');
});

test('Multisig change is excluded from the send total and labelled as change', async () => {
    await openReview(summary({
        output_count: 2,
        total_out_sompi: KAS,
        outputs: [output(DEST, KAS / 2), output(CHANGE, KAS / 2)],
    }), sendContext);
    assertEqual(text('pskt-send-total'), '0.5');
    assertEqual(text('pskt-change-total'), '0.5');
    const labels = [...document.querySelectorAll('#pskt-outputs .pskt-kind')].map(node => node.textContent);
    assertEqual(labels[1], 'MULTISIG CHANGE · P2PK');
});

test('Partially signed multisig input offers relay and hides finalize', async () => {
    await openReview(summary({ inputs: [input({ sigs_present: 1 })] }), sendContext);
    assertEqual(document.getElementById('btn-pskt-relay').textContent, 'Relay to next signer');
    assert(!hidden('btn-pskt-relay'), 'relay visible while signatures are missing');
    assert(hidden('btn-pskt-finalize'), 'finalize hidden until ready');
});

test('Unsigned single-signer input offers send to signer', async () => {
    await openReview(summary({ inputs: [input({ multisig_m: null, multisig_n: null })] }), sendContext);
    assertEqual(document.getElementById('btn-pskt-relay').textContent, 'Send to signer');
    assert(!hidden('btn-pskt-relay'), 'relay visible while unsigned');
});

test('Fully signed transaction shows finalize and hides the signing action', async () => {
    await openReview(summary({ inputs: [input({ sigs_present: 2 })], finalize_ready: true }), sendContext);
    assert(!hidden('btn-pskt-finalize'), 'finalize visible when ready');
    assert(hidden('btn-pskt-relay'), 'relay hidden when ready');
});
