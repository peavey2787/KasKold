// Wallet-owned address and history state. Wallet payload access lives in wallet_session.js.
/** @type {Record<string, any>} */
export const walletState = Object.seal({
    'fundedChangeIndices': undefined,
    'fundedReceiveIndices': undefined,
    'usedChangeIndices': undefined,
    'usedReceiveIndices': undefined,
    'standardChangeReservations': undefined,
    'historyEntries': undefined,
    'addressHistoryEnabled': undefined,
});
