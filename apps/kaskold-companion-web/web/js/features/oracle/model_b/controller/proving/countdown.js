import { oracleState } from '../../../../../app/state/index.js';
// Oracle roll review countdown.

function isOnPsktReview() {
  const review = document.getElementById('pskt-review');
  return Boolean(review && !review.classList.contains('hidden') && review.style.display !== 'none');
}

function ensureCountdownBox() {
  const existing = document.getElementById('oracle-mb-countdown');
  if (existing) return existing;
  const button = document.getElementById('btn-pskt-finalize');
  if (!button || !button.parentNode) return null;
  const box = document.createElement('div');
  box.id = 'oracle-mb-countdown';
  box.classList.add('oracle-proof-countdown');
  button.parentNode.insertBefore(box, button.nextSibling);
  return box;
}

function countdownMessage() {
  const remainingMs = (oracleState._oracleMbProveDeadline || 0) - Date.now();
  if (!oracleState._oracleMbProveDeadline || remainingMs <= 0) {
    return 'Proof finishing — it will broadcast automatically any moment…';
  }
  const seconds = Math.ceil(remainingMs / 1000);
  return (oracleState._oracleMbAutoBroadcast ? 'Signed. ' : '')
    + 'Proof proving — auto-broadcast in ~' + seconds + 's';
}

function updateCountdown() {
  const awaiting = oracleState._oracleMbPreSignAwaiting || oracleState._oracleMbAutoBroadcast;
  const existing = document.getElementById('oracle-mb-countdown');
  if (!awaiting || !isOnPsktReview()) {
    if (existing) existing.style.display = 'none';
    return;
  }
  const box = ensureCountdownBox();
  if (!box) return;
  box.style.display = 'block';
  box.textContent = countdownMessage();
}

let countdownTimer = null;

export function startOracleMbCountdown() {
  if (countdownTimer !== null) return;
  updateCountdown();
  countdownTimer = setInterval(() => updateCountdown(), 1000);
}

export function stopOracleMbCountdown() {
  if (countdownTimer !== null) clearInterval(countdownTimer);
  countdownTimer = null;
  const box = document.getElementById('oracle-mb-countdown');
  if (box) box.style.display = 'none';
}
