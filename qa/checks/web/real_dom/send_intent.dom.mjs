// A built send must match the user's request exactly before it can reach a signer.

import { assertEqual, test } from './harness.mjs';

const BUILDING = '/apps/kaskold-companion-web/web/js/features/transactions/send/compose/transaction_building.js';
const DESTINATION = 'kaspa:destination';
const CHANGE = 'kaspa:change-7';
const WALLET = JSON.stringify({ next_change_index: 7, change_addresses: { 7: CHANGE } });

function summary(overrides = {}) {
    return {
        total_in_sompi: '1000',
        fee_sompi: '10',
        outputs: [
            { address: DESTINATION, amount_sompi: '600', derivation_branch: null },
            { address: CHANGE, amount_sompi: '390', derivation_branch: 1, derivation_index: 7 },
        ],
        ...overrides,
    };
}

function withOutput(index, overrides) {
    const built = summary();
    built.outputs[index] = { ...built.outputs[index], ...overrides };
    return built;
}

async function rejection(built, wallet = WALLET) {
    const { assertStandardSendIntent } = await import(BUILDING);
    try {
        assertStandardSendIntent(built, wallet, DESTINATION, 600n, 10n);
    } catch (error) {
        return error.message;
    }
    return null;
}

test('a transaction matching the request passes', async () => {
    assertEqual(await rejection(summary()), null);
});

test('an exact spend with no change passes', async () => {
    const built = summary({ total_in_sompi: '610', outputs: [summary().outputs[0]] });
    assertEqual(await rejection(built), null);
});

const TAMPERED = {
    'a redirected destination': withOutput(0, { address: 'kaspa:attacker' }),
    'a changed destination amount': withOutput(0, { amount_sompi: '601' }),
    'a destination on the change branch': withOutput(0, { derivation_branch: 1 }),
    'a changed fee': summary({ fee_sompi: '11' }),
    'redirected change': withOutput(1, { address: 'kaspa:attacker' }),
    'a changed change amount': withOutput(1, { amount_sompi: '389' }),
    'a stale change index': withOutput(1, { derivation_index: 6 }),
    'spending more than the inputs': summary({ total_in_sompi: '609' }),
    'an extra external output': summary({
        outputs: [...summary().outputs, { address: 'kaspa:attacker', amount_sompi: '1', derivation_branch: null }],
    }),
    'a duplicated destination payment': summary({
        outputs: [summary().outputs[0], summary().outputs[0], summary().outputs[1]],
    }),
    'a missing change output': summary({ outputs: [summary().outputs[0]] }),
    'unexpected change on an exact spend': summary({ total_in_sompi: '610' }),
    'an extra output on an exact spend': summary({
        total_in_sompi: '610',
        outputs: [summary().outputs[0], { address: 'kaspa:attacker', amount_sompi: '0', derivation_branch: null }],
    }),
};

for (const [name, built] of Object.entries(TAMPERED)) {
    test(`${name} is rejected`, async () => {
        assertEqual(typeof await rejection(built), 'string');
    });
}

test('wallet state that cannot be read or lacks the change address is rejected', async () => {
    for (const wallet of ['not json', '{}', JSON.stringify({ next_change_index: 8, change_addresses: { 7: CHANGE } })]) {
        assertEqual(typeof await rejection(summary(), wallet), 'string');
    }
});
