// @ts-check
// Covenant types whose owner path IS the timelock branch, and what is valid until it matures.
export const TIMELOCKED_OWNER_REASONS = new Map([
    ['merkle-whitelist', 'only whitelisted spends are valid'],
    ['commit-reveal', 'only the reveal path is valid'],
    ['payjoin', 'only the joint-spend path is valid'],
    ['oracle-v1', 'only an oracle-attested beneficiary claim is valid'],
    ['crowdfund', 'only a sweep that reached the goal is valid'],
]);
