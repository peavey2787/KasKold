import { el, setContent } from './elements.js';

/**
 * Show multi-frame QR scan progress: one dot per frame plus a count.
 *
 * @param {Element|null} target
 * @param {{total: number, count: number, bits: boolean[]}} progress decoder_progress() result
 */
export function renderScanProgress(target, progress) {
    if (!target || !(progress?.total > 0)) return;
    const dots = Array.from({ length: progress.total }, (_, index) => el('span', {
        className: `scanner-progress-dot${progress.bits?.[index] ? ' scanner-progress-dot-active' : ''}`,
    }));
    setContent(target, dots, el('div', { className: 'u-mt-6px-text-12px' }, `${progress.count} / ${progress.total} frames`));
}
