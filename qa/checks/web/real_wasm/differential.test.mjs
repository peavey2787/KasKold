// JavaScript reads node answers before Rust does. The two readers must agree on
// which coins exist: a coin only one of them accepts is a spend that fails late
// or a balance that lies.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { explicitUtxo, KEY_1G_ADDRESS, ACCOUNT_KEY, real, webModule } from './support.mjs';

const { parseUtxosJson } = await import(webModule('core/utxo.js'));

const REFUSED_BY_BOTH = [
    '50000000', '0', '-1', '"-1"', '1.5', '"1.5"', '1e3', '"1e3"', '9007199254740992', '18446744073709551615',
    '18446744073709551616', '"18446744073709551616"', 'null', 'true', '""', '" 5"', '"0x10"', '"007"', '"+5"', '[]', '{}',
];
const ACCEPTED_BY_BOTH = [
    ['"50000000"', 50_000_000n], ['"0"', 0n], ['"18446744073709551615"', 18_446_744_073_709_551_615n],
];

const wallet = real.import_kpub(ACCOUNT_KEY, 'mainnet');

function coinsWithAmount(amountJson) {
    const placeholder = JSON.stringify(explicitUtxo({ amount: 0 }));
    return `[${placeholder.replace('"amount":0', `"amount":${amountJson}`)}]`;
}

function javascriptReads(coins) {
    try {
        return parseUtxosJson(coins)[0].amount;
    } catch {
        return null;
    }
}

async function rustReads(coins) {
    try {
        await real.create_send_pskb_with_utxos(wallet, KEY_1G_ADDRESS, 1n, 1n, coins);
        return true;
    } catch (error) {
        return !String(error).includes('Bad UTXOs JSON');
    }
}

test('both readers refuse a coin amount that is not an exact unsigned 64-bit integer', async () => {
    for (const amount of REFUSED_BY_BOTH) {
        const coins = coinsWithAmount(amount);
        assert.equal(javascriptReads(coins), null, `JavaScript read ${amount}`);
        assert.equal(await rustReads(coins), false, `Rust read ${amount}`);
    }
});

test('both readers take an exact amount and read the same value', async () => {
    for (const [amount, value] of ACCEPTED_BY_BOTH) {
        const coins = coinsWithAmount(amount);
        assert.equal(javascriptReads(coins), value, `JavaScript misread ${amount}`);
        assert.equal(await rustReads(coins), true, `Rust refused ${amount}`);
    }
});
