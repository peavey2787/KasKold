import { hexToBytes } from '../../../../../core/bytes.js';
import { read_commit_reveal_reveal } from '../../../../../wasm/api.js';

export function verifyCommitRevealTransaction(transaction) {
    const input = transaction.inputs?.[0];
    const signatureScriptHex = input?.signature_script || '';
    if (signatureScriptHex.length < 10) throw new Error('No sig_script in TX');

    const reveal = JSON.parse(read_commit_reveal_reveal(signatureScriptHex));
    const fullPreimage = hexToBytes(reveal.preimage_hex);
    if (fullPreimage.length <= 8) {
        throw new Error('Commit-reveal preimage is missing its 8-byte salt');
    }
    let preimageText;
    try { preimageText = new TextDecoder().decode(fullPreimage.slice(8)); }
    catch { preimageText = reveal.preimage_hex; }
    return {
        preimageText,
        committedHash: reveal.committed_hash,
        computedHash: reveal.computed_hash,
        matches: reveal.matches,
        timestamp: transaction.block_time
            ? new Date(transaction.block_time).toLocaleString()
            : 'DAA: ' + (input?.previous_outpoint_resolved_daa_score || 'unknown'),
    };
}
