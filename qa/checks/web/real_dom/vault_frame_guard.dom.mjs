// The Vault must not render its UI inside another page's frame.

import { assertEqual, test } from './harness.mjs';

const GUARD = '/apps/kaskold-vault-web/web/js/vault_frame_guard.js';

test('a framed Vault clears its document and reports refusal', async () => {
    const { refuseFramedVault } = await import(GUARD);
    const root = document.createElement('div');
    root.append(document.createElement('button'));
    assertEqual(refuseFramedVault({ top: {}, self: {} }, root), true);
    assertEqual(root.childNodes.length, 0);
});

test('a top-level Vault keeps its document', async () => {
    const { refuseFramedVault } = await import(GUARD);
    const view = {};
    view.top = view;
    view.self = view;
    const root = document.createElement('div');
    root.append(document.createElement('button'));
    assertEqual(refuseFramedVault(view, root), false);
    assertEqual(root.childNodes.length, 1);
});
