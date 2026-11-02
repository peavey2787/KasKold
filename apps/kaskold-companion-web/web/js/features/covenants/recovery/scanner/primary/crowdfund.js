import { crowdfund_campaign_id } from '../../../../../wasm/api.js';
import { hexToBytes } from '../../../../../core/bytes.js';
import { readLen, readU64, readVstr } from '../payload_reader.js';
import { contributionJson } from '../../../crowdfund/model.js';
import { baseRecoveredRecord, readStoredScript } from './common.js';

function readVhex(hex, offset) {
    const { len, endOff } = readLen(hex, offset);
    const value = hex.slice(endOff, endOff + len * 2);
    if (value.length !== len * 2) throw new Error('Recovered crowdfunding field is truncated');
    return { value, endOff: endOff + len * 2 };
}

function readCrowdfundFields(params, startOffset) {
    let offset = startOffset;
    const contributorPubkeyHex = params.slice(offset, offset + 64); offset += 64;
    const salt = readVhex(params, offset); offset = salt.endOff;
    const goalSompi = readU64(params, offset); offset += 16;
    const locktimeDaa = readU64(params, offset); offset += 16;
    const organizer = readVstr(params, offset, hexToBytes); offset = organizer.endOff;
    const name = readVstr(params, offset, hexToBytes); offset = name.endOff;
    const vk = readVhex(params, offset); offset = vk.endOff;
    const provingKey = readVhex(params, offset); offset = provingKey.endOff;
    const campaignId = params.slice(offset, offset + 64); offset += 64;
    const role = readVstr(params, offset, hexToBytes); offset = role.endOff;
    const contributions = readVstr(params, offset, hexToBytes); offset = contributions.endOff;
    const date = readVstr(params, offset, hexToBytes); offset = date.endOff;
    if (offset !== params.length) throw new Error('Recovered crowdfunding payload has trailing data');
    return {
        contributorPubkeyHex, salt, goalSompi, locktimeDaa, organizer, name,
        vk, provingKey, campaignId, role, contributions, date,
    };
}

function assertCrowdfundParameters(fields) {
    if (!/^[0-9a-f]{64}$/.test(fields.contributorPubkeyHex)) throw new Error('Recovered crowdfunding contributor key is invalid');
    if (!/^[0-9a-f]{16}$/.test(fields.salt.value)) throw new Error('Recovered crowdfunding salt is invalid');
    if (fields.goalSompi === 0n || fields.locktimeDaa === 0n) throw new Error('Recovered crowdfunding goal/deadline is invalid');
    if (!fields.organizer.str.includes(':')) throw new Error('Recovered crowdfunding organizer destination is invalid');
}

function assertCrowdfundCampaign(fields) {
    const { vk, organizer, goalSompi, locktimeDaa, campaignId, role } = fields;
    if (!vk.value || vk.value.length > 32768
        || crowdfund_campaign_id(organizer.str, goalSompi, locktimeDaa, vk.value) !== campaignId) {
        throw new Error('Recovered crowdfunding verifying key/campaign identity is invalid');
    }
    if (role.str !== 'organizer' && role.str !== 'contributor') throw new Error('Recovered crowdfunding role is invalid');
}

export function rebuildCrowdfund(type, params) {
    const stored = readStoredScript(params);
    const fields = readCrowdfundFields(params, stored.offset);
    assertCrowdfundParameters(fields);
    assertCrowdfundCampaign(fields);
    const contributionSet = contributionJson(fields.contributions.str || '[]');
    return {
        ...baseRecoveredRecord(type, stored.redeemScriptHex, 'owner'),
        contributor_pubkey_hex: fields.contributorPubkeyHex,
        crowdfund_salt_hex: fields.salt.value,
        goal_sompi: fields.goalSompi,
        locktime_daa: fields.locktimeDaa,
        organizer_address: fields.organizer.str,
        campaign_name: fields.name.str,
        vk_hex: fields.vk.value,
        crowdfund_pk_hex: fields.provingKey.value,
        campaign_id: fields.campaignId,
        crowdfund_role: fields.role.str,
        crowdfund_contributions_json: contributionSet,
        ...(fields.date.str ? { locktime_date_iso: fields.date.str } : {}),
    };
}
