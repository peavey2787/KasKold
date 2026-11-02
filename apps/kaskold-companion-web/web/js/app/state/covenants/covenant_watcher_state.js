// @ts-check
// Focused covenantWatcher session state.
/** @type {Record<string, any>} */
export const covenantWatcherState = Object.seal({
    '_covActiveWatcherTimer': null,
    '_covWatcherLastBalance': null,
    '_covWatcherOutpoint': null,
    '_covWatcherSpendPath': undefined,
    '_covWatcherTimer': null,
});
