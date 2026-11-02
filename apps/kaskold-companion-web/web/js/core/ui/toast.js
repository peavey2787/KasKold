// @ts-check
import { uiState } from '../../app/state/index.js';
import { byId } from '../dom.js';


/**
 * @param {string} message
 * @param {string} [type]
 * @param {number} [duration]
 */
export function toast(message, type = 'info', duration = 3000) {
    const element = byId('toast');
    if (!element) return;
    element.textContent = message;
    element.className = `toast toast-${type} visible`;
    if (uiState.toastTimer) clearTimeout(uiState.toastTimer);
    uiState.toastTimer = setTimeout(() => {
        element.classList.remove('visible');
        uiState.toastTimer = null;
    }, duration);
}
