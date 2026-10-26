import { covenantState } from '../../../../app/state/index.js';
import { showScreen } from '../../../../app/navigation.js';
import { hexToBytes } from '../../../../core/bytes.js';
import { toast } from '../../../../core/ui/toast.js';
import { covShowPanel } from '../../generation/ui_and_keys.js';
import { covAddActive, covRenderActive, covSaveActive } from '../active.js';
import { verifiedCovenantTerms } from '../../model/script_terms.js';
import { inferCovenantRole } from './role.js';

const HEX_32 = /^[0-9a-f]{64}$/;
const LOCKTIME_DATE_TYPES = new Set(['payjoin', 'timelocked-savings', 'oracle-v1']);

function decodeInvite(hex) {
    const jsonBytes = hexToBytes(hex.slice(8));
    return JSON.parse(new TextDecoder().decode(jsonBytes));
}

// Fields an invite carries that no script can prove: calendar hints for its
// timelocks and the oracle's off-chain key binding record.
function addInviteHints(entry, invite) {
    if (invite.ldi && LOCKTIME_DATE_TYPES.has(entry.type)) entry.locktime_date_iso = invite.ldi;
    if (invite.sdi && entry.type === 'global-allowance') entry.start_date_iso = invite.sdi;
    if (entry.type !== 'oracle-v1') return;
    if (!HEX_32.test(invite.okid || '') || !HEX_32.test(invite.obt || '')) {
        throw new Error('Oracle-v1 invite is missing its covenant key binding record');
    }
    entry.oracle_covenant_key_id_hex = invite.okid;
    entry.oracle_covenant_binding_token_hex = invite.obt;
}

// Every term and the wallet's role come from the redeem script; an invite
// whose address, script or statement disagree is refused.
function buildInviteEntry(invite) {
    if (invite.t !== 'cov-invite' || !invite.addr || !invite.rs) {
        throw new Error('Invalid invite format');
    }
    const entry = {
        ...verifiedCovenantTerms({
            type: invite.ct,
            address: invite.addr,
            redeem_script_hex: invite.rs,
            attestation_statement: invite.oas,
        }),
        loaded: true,
    };
    addInviteHints(entry, invite);
    entry.role = inferCovenantRole(entry);
    return entry;
}

export function importCovenantInvite(hex) {
    const invite = decodeInvite(hex);
    const entry = buildInviteEntry(invite);
    const alreadyActive = covenantState.activeCovenants.some(covenant => covenant.address === entry.address);
    if (!alreadyActive) {
        covAddActive(entry.type, entry);
        covSaveActive();
        covRenderActive();
    }
    showScreen('covenant');
    covShowPanel('menu');
    toast(alreadyActive ? 'Covenant already active' : 'Covenant invite restored', 'ok', alreadyActive ? 2000 : 3000);
    return !alreadyActive;
}
