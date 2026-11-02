// @ts-check
// Focused covenant session state.
/** @type {Record<string, any>} */
export const covenantState = Object.seal({
    '_covPayloadHex': undefined,
    '_lastKnownDaa': undefined,
    'activeCovenants': undefined,
    'lastCovenantResult': null,
    '_pickerBeneClaim': null,
    'covFeeLevel': 'normal',
    '_kasFreezePathCPostBroadcast': undefined,
});
