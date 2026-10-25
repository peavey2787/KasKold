import { el } from '../../../core/ui/elements.js';

const STEP_CLASS = 'mbpv-step u-display-flex-direction-column-align-center-gap-7px';
const NUMBER_CLASS = 'mbpv-num u-width-30px-height-30px-rounded-50pct-border-2px-solid-2a4a40';

function provingStep(number, label, active) {
  return el('div', { className: `${STEP_CLASS}${active ? ' active' : ''}`, dataset: { step: number } },
    el('div', { className: NUMBER_CLASS }, String(number)), label);
}

export function showOracleProvingScreen(priceRaw) {
  hideOracleProvingScreen();
  const price = Number(priceRaw) / 1e8;
  const priceText = Number.isFinite(price) && price > 0
    ? ('$' + price.toFixed(8).replace(/0+$/, '').replace(/\.$/, ''))
    : '';
  const overlay = document.createElement('div');
  overlay.id = 'mb-prove';
  overlay.classList.add('price-proof-overlay');
  overlay.append(el('div', { className: 'price-proof-panel' },
    el('div', { className: 'price-proof-animation' },
      el('div', { className: 'price-proof-ring' }),
      el('div', { className: 'price-proof-ring-delayed' }),
      el('div', { className: 'price-proof-spinner' }),
      el('div', { className: 'price-proof-bolt' }, '⚡')),
    el('div', { className: 'price-proof-title' }, 'Proving your price'),
    priceText ? el('div', { className: 'price-proof-value' }, priceText) : null,
    el('div', { className: 'price-proof-description' },
      'Generating a zero-knowledge proof on a GPU, then broadcasting through your node. This usually takes a minute or two. Keep this tab open.'),
    el('div', { id: 'mb-prove-timer' }, '0:00 elapsed'),
    el('div', { className: 'price-proof-metrics' }, provingStep(1, 'Prove', true), provingStep(2, 'Broadcast', false))));
  document.body.appendChild(overlay);
  const startedAt = Date.now();
  overlay._timer = setInterval(() => {
    const elapsed = Math.floor((Date.now() - startedAt) / 1000);
    const timer = document.getElementById('mb-prove-timer');
    if (timer) timer.textContent = Math.floor(elapsed / 60) + ':' + String(elapsed % 60).padStart(2, '0') + ' elapsed';
  }, 1000);
}

export function setOracleProvingStage(stage) {
  const overlay = document.getElementById('mb-prove');
  if (!overlay || stage !== 'broadcast') return;
  const prove = overlay.querySelector('.mbpv-step[data-step="1"]');
  const broadcast = overlay.querySelector('.mbpv-step[data-step="2"]');
  if (prove) {
    prove.classList.remove('active');
    prove.classList.add('done');
    const number = prove.querySelector('.mbpv-num');
    if (number) number.textContent = '✓';
  }
  if (broadcast) broadcast.classList.add('active');
}

export function hideOracleProvingScreen() {
  const overlay = document.getElementById('mb-prove');
  if (!overlay) return;
  if (overlay._timer) clearInterval(overlay._timer);
  overlay.remove();
}
