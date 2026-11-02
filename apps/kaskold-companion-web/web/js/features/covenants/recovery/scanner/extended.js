import { networkState } from '../../../../app/state/index.js';
import { decodeCommitReveal } from './extended/commit_reveal.js';
import { decodeGeneric } from './extended/generic.js';
import { decodeMerkleWhitelist } from './extended/merkle.js';
import { decodePayjoin } from './extended/payjoin.js';

const DECODERS = Object.freeze({
    'merkle-whitelist': (params, network) => decodeMerkleWhitelist(params, network),
    payjoin: (params, network) => decodePayjoin(params, network),
    'commit-reveal': (params, network) => decodeCommitReveal(params, network),
});

export function rebuildExtendedRecoveredCovenant(type, params, ownerPublicKey) {
    const decoder = DECODERS[type];
    return decoder
        ? decoder(params, networkState.network, ownerPublicKey)
        : decodeGeneric(type, params, networkState.network);
}
