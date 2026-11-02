// @ts-check
import { el } from '../../../../../core/ui/elements.js';
import { stealthState } from '../../../boundary.js';
import { byId, elementById, inputById } from '../../../../../core/dom.js';
import { toast } from '../../../../../core/ui/toast.js';

export function ensureStealthManualRSection() {
    if (byId('stealth-manual-r-input')) return;
    const section = document.createElement('div');
    section.id = 'stealth-manual-r-section';
    section.classList.add('u-mt-8px');
    const input = el('input', {
        className: 'input-text', id: 'stealth-manual-r-input',
        attrs: { type: 'text', placeholder: "64-char hex (sender's ephemeral R)" },
    });
    input.setAttribute('autocomplete', 'off');
    input.spellcheck = false;
    section.append(
        el('label', { className: 'input-label' }, 'Manual R entry (64 hex — out-of-band fallback)'),
        input,
        el('button', { className: 'btn btn-outline u-full-width-mt-4px', id: 'btn-stealth-add-r' }, 'Add R Value'),
        el('div', { className: 'u-text-11px-mt-4px-text-text-dim', id: 'stealth-r-list' }));
    const panel = byId('stealth-scan-panel');
    const backButton = byId('btn-stealth-scan-back');
    const anchor = backButton?.closest('.back-home-row') || backButton;
    if (!panel) {
        console.error('[Companion] stealth scan panel is unavailable');
        return;
    }
    if (anchor && anchor.parentNode === panel) panel.insertBefore(section, anchor);
    else panel.insertBefore(section, null);
    elementById('btn-stealth-add-r').onclick = () => {
        if (stealthState._stealthCatchupRunning) {
            toast('Lane scan still running, please wait', 'error');
            return;
        }
        const value = inputById('stealth-manual-r-input').value.trim();
        if (value.length !== 64 || !/^[0-9a-fA-F]+$/.test(value)) {
            toast('R must be 64 hex chars', 'error');
            return;
        }
        if (!stealthState.stealthAnnouncementsR.includes(value)) {
            stealthState.stealthAnnouncementsR.push(value);
        }
        inputById('stealth-manual-r-input').value = '';
        elementById('stealth-r-list').textContent = `${stealthState.stealthAnnouncementsR.length} R value(s) loaded`;
        elementById('btn-stealth-show-scan-qr').classList.remove('hidden');
    };
}
