import { covenantWatcherState } from '../../../../boundary.js';

export function isFundsSpent(total) {
    const lastBalance = covenantWatcherState._covWatcherLastBalance;
    return total === 0n && lastBalance !== null && lastBalance > 0n;
}

export function setPlainStatus(st, text) {
    st.textContent = text;
    st.style.color = '';
}
