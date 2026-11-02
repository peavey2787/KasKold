// @ts-check
// Focused ZK Crowdfunding session state. Campaign material is copied into the
// active covenant record/owner backup after creation; no window.* globals are used.
/** @type {Record<string, any>} */
export const crowdfundState = Object.seal({
    'role': 'organizer',
    'setup': null,
    'importedCampaign': null,
    'contributions': [],
    'watcherTimer': null,
});
