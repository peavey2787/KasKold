import { covenantState } from '../../../app/state/index.js';
import { toast } from '../../../core/ui/toast.js';
import { showStaticPayloadQr } from '../../qr_display/static_payload.js';
import { addContribution, campaignFromResult, contributionFromResult, normalizeCampaign, validateContribution } from './model.js';
import { persistContributions } from './persistence.js';
import { renderContributionList } from './sweep.js';

export function shareCrowdfundCampaign() {
    const result = organizerResult();
    if (!result) return;
    const campaign = campaignFromResult(result);
    showStaticPayloadQr(JSON.stringify({
        ...campaign,
        goal: campaign.goal.toString(),
        daa: campaign.daa.toString(),
    }), 'Crowdfunding Campaign Invite');
}

export function shareCrowdfundContribution() {
    const result = covenantState.lastCovenantResult;
    if (!result || result.type !== 'crowdfund') return;
    const contribution = contributionFromResult(result);
    const payload = {
        v: 2,
        t: 'crowdfund-contribution',
        campaign_id: result.campaign_id,
        contribution,
    };
    showStaticPayloadQr(JSON.stringify(payload), 'Crowdfunding Contribution Invite');
}

export function importCrowdfundContribution(raw) {
    const invite = decodeJson(raw);
    if (invite?.v !== 2 || invite?.t !== 'crowdfund-contribution') throw new Error('Not a current crowdfunding contribution invite');
    const result = organizerResult();
    if (!result) throw new Error('Load the organizer crowdfunding record first');
    if (String(invite.campaign_id || '').toLowerCase() !== String(result.campaign_id || '').toLowerCase()) {
        throw new Error('Contribution belongs to a different crowdfunding campaign');
    }
    validateContribution(invite.contribution);
    const list = addContribution(invite.contribution);
    persistContributions(list);
    renderContributionList();
    toast('Contributor address added', 'ok', 2000);
}

export function parseCrowdfundCampaign(raw) {
    return normalizeCampaign(decodeJson(raw));
}

function decodeJson(raw) {
    const bytes = raw instanceof Uint8Array ? raw : new Uint8Array(raw);
    return JSON.parse(new TextDecoder().decode(bytes));
}

function organizerResult() {
    const result = covenantState.lastCovenantResult;
    if (!result || result.type !== 'crowdfund' || result.crowdfund_role !== 'organizer') {
        toast('Load the organizer crowdfunding record first', 'error');
        return null;
    }
    return result;
}
