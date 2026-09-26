function transactionScanError(error) {
  const message = String(error?.message || error || 'Transaction QR could not be accepted.');
  if (message.includes('Custody(InvalidToolInput)')) {
    return new Error('This transaction does not match the active Vault wallet or contains invalid wallet-ownership metadata. Confirm the same wallet is selected in Vault and Companion, then copy the exact Companion QR Frame Hex again.');
  }
  return error instanceof Error ? error : new Error(message);
}

function groupInteger(value) {
  return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

function formatSompi(value) {
  return groupInteger(BigInt(String(value || '0')).toString());
}

function formatKas(value) {
  const sompi = BigInt(String(value || '0'));
  const whole = sompi / 100_000_000n;
  const fraction = (sompi % 100_000_000n).toString().padStart(8, '0');
  return `${groupInteger(whole.toString())}.${fraction}`;
}


function setBalancedAddress(element, value) {
  const text = String(value || '');
  if (!/^[a-z0-9]+:[a-z0-9]+$/i.test(text) || text.length < 24) {
    element.textContent = text;
    return;
  }
  const split = Math.ceil(text.length / 2);
  const first = document.createElement('span');
  const second = document.createElement('span');
  first.className = 'balanced-address-line';
  second.className = 'balanced-address-line';
  first.textContent = text.slice(0, split);
  second.textContent = text.slice(split);
  element.classList.add('balanced-address');
  element.setAttribute('aria-label', text);
  element.setAttribute('title', text);
  element.replaceChildren(first, second);
}

function detailCard(title, badge, lines) {
  const article = document.createElement('article');
  article.className = 'vault-inspect-row';

  const heading = document.createElement('div');
  heading.className = 'vault-inspect-row-head';
  const index = document.createElement('span');
  index.className = 'vault-inspect-index';
  index.textContent = title;
  heading.appendChild(index);
  if (badge) {
    const kind = document.createElement('span');
    kind.className = 'vault-inspect-kind';
    kind.textContent = badge;
    heading.appendChild(kind);
  }
  article.appendChild(heading);

  const body = document.createElement('div');
  body.className = 'vault-inspect-row-body';
  for (const [label, value] of lines) {
    const key = document.createElement('span');
    key.className = 'vault-inspect-label';
    key.textContent = label;
    const detail = document.createElement('span');
    detail.className = 'vault-inspect-value mono break';
    if (label === 'Address') setBalancedAddress(detail, value);
    else detail.textContent = value;
    body.append(key, detail);
  }
  article.appendChild(body);
  return article;
}

function responseImageName(prefix, index, total) {
  const width = Math.max(2, String(total).length);
  return `${prefix}-frame-${String(index + 1).padStart(width, '0')}-of-${String(total).padStart(width, '0')}.png`;
}

function sleep(milliseconds) {
  return new Promise(resolve => setTimeout(resolve, milliseconds));
}

async function svgToPngBlob(svg) {
  const source = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
  const url = URL.createObjectURL(source);
  try {
    const image = new Image();
    await new Promise((resolve, reject) => {
      image.onload = resolve;
      image.onerror = () => reject(new Error('QR image could not be rendered.'));
      image.src = url;
    });
    const canvas = document.createElement('canvas');
    canvas.width = 1024;
    canvas.height = 1024;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('QR image canvas is unavailable.');
    context.imageSmoothingEnabled = false;
    context.fillStyle = '#ffffff';
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    return await new Promise((resolve, reject) => {
      canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('QR image could not be encoded.')), 'image/png');
    });
  } finally {
    URL.revokeObjectURL(url);
  }
}

function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function installTransactionWorkflows({ $, api, parse, show, status, openScanner, stopCamera, renderHome }) {
  let review = null;
  let reviewUnit = 'kas';
  let frames = [];
  let frameIndex = 0;
  let timer = null;
  let cyclePaused = false;
  let antiKleptoAwaitingReveal = false;
  let specialRouter = null;
  let scanBegun = false;

  function stopCycle() {
    if (timer !== null) clearInterval(timer);
    timer = null;
  }

  function reset() {
    stopCycle();
    review = null;
    reviewUnit = 'kas';
    frames = [];
    frameIndex = 0;
    cyclePaused = false;
    antiKleptoAwaitingReveal = false;
    scanBegun = false;
  }

  function formatReviewAmount(value) {
    return reviewUnit === 'kas' ? `${formatKas(value)} KAS` : `${formatSompi(value)} Sompi`;
  }

  function reviewDestinations(nextReview) {
    const outputs = Array.isArray(nextReview?.outputs) ? nextReview.outputs : [];
    const external = outputs.filter(output => output.ownership === 'External');
    if (external.length) return external;
    return outputs.filter(output => output.ownership !== 'Change');
  }

  function renderDestinationAddresses() {
    const root = $('review-destinations');
    root.replaceChildren();
    const destinations = reviewDestinations(review);
    if (!destinations.length) {
      const row = document.createElement('div');
      row.className = 'review-destination';
      const label = document.createElement('div');
      label.className = 'review-destination-label';
      label.textContent = 'To';
      const address = document.createElement('div');
      address.className = 'review-destination-address';
      address.textContent = 'No external destination';
      row.append(label, address);
      root.append(row);
      return;
    }
    destinations.forEach((output, index) => {
      const row = document.createElement('div');
      row.className = 'review-destination';
      const label = document.createElement('div');
      label.className = 'review-destination-label';
      label.textContent = destinations.length === 1 ? 'To' : `To ${index + 1}`;
      const address = document.createElement('div');
      address.className = 'review-destination-address';
      setBalancedAddress(address, output.address || 'Not representable as a single address');
      row.append(label, address);
      root.append(row);
    });
  }

  function renderReviewAmounts() {
    if (!review) return;
    $('review-total-amount').textContent = formatReviewAmount(review.externalTotal);
    $('review-fee').textContent = formatReviewAmount(review.fee);
    const change = BigInt(String(review.changeTotal || '0'));
    $('review-change-total').textContent = formatReviewAmount(change);
    $('review-change-row').classList.toggle('hidden', change === 0n);
    const alternate = reviewUnit === 'kas' ? 'Sompi' : 'KAS';
    const toggle = $('review-unit-toggle');
    toggle.textContent = `⇄ ${alternate}`;
    toggle.setAttribute('aria-label', `Show amounts in ${alternate}`);
    toggle.setAttribute('title', `Show amounts in ${alternate}`);
  }

  function renderReview(nextReview) {
    stopCamera();
    review = nextReview;
    reviewUnit = 'kas';
    renderReviewAmounts();
    renderDestinationAddresses();
    show('review');
  }

  function toggleReviewUnit() {
    reviewUnit = reviewUnit === 'kas' ? 'sompi' : 'kas';
    renderReviewAmounts();
  }

  function renderInspection() {
    if (!review) throw new Error('No transaction is pending review.');
    $('review-inspect-network').textContent = review.network || '—';
    $('review-inspect-input-count').textContent = String(review.inputCount ?? review.inputs.length);
    $('review-inspect-output-count').textContent = String(review.outputCount ?? review.outputs.length);
    $('review-inspect-total-in').textContent = formatReviewAmount(review.inputTotal);
    $('review-inspect-total-out').textContent = formatReviewAmount(review.outputTotal);
    $('review-inspect-fee').textContent = formatReviewAmount(review.fee);

    const inputRoot = $('review-inputs-detail');
    const outputRoot = $('review-outputs-detail');
    inputRoot.replaceChildren(...review.inputs.map(input => detailCard(`Input ${input.index + 1}`, input.scriptType, [
      ['Outpoint', input.outpoint || '—'],
      ['Amount', formatReviewAmount(input.amount)],
      ['Address', input.address || 'Not representable as a single address'],
    ])));
    outputRoot.replaceChildren(...review.outputs.map(output => detailCard(`Output ${output.index + 1}`, output.ownership, [
      ['Amount', formatReviewAmount(output.amount)],
      ['Address', output.address || 'Not representable as a single address'],
    ])));
    show('review-inspect');
  }

  function acceptFrame(bytes) {
    if (specialRouter?.(bytes)) return true;
    if (!scanBegun) {
      api('kaskold_vault_begin_scan')();
      scanBegun = true;
    }
    let result;
    try {
      result = parse(api('kaskold_vault_accept_frame')(bytes));
    } catch (error) {
      throw transactionScanError(error);
    }
    if (result.state === 'review') { renderReview(result); return true; }
    $('scan-progress').textContent = result.total > 0
      ? `Signing request: ${result.received}/${result.total} frames received.`
      : 'Signing request frame accepted.';
    return false;
  }

  function renderFrame() {
    if (!frames.length) return;
    const frame = frames[frameIndex];
    $('response-qr').innerHTML = frame.svg;
    $('response-frame-label').textContent = `Frame ${frameIndex + 1}/${frames.length}`;
    $('response-hex').textContent = frame.payloadHex;
    const single = frames.length <= 1;
    $('response-prev').disabled = single;
    $('response-next').disabled = single;
    $('response-pause').disabled = single;
    $('response-pause').textContent = cyclePaused ? '▶' : '⏸';
    $('response-pause').setAttribute('title', cyclePaused ? 'Resume QR animation' : 'Pause QR animation');
    $('response-pause').setAttribute('aria-label', cyclePaused ? 'Resume QR animation' : 'Pause QR animation');
  }

  function startCycle() {
    stopCycle();
    if (cyclePaused || frames.length <= 1) return;
    timer = setInterval(() => {
      frameIndex = (frameIndex + 1) % frames.length;
      renderFrame();
    }, 2000);
  }

  function showResponse() {
    frameIndex = Math.min(frameIndex, Math.max(0, frames.length - 1));
    cyclePaused = frames.length <= 1;
    show('response');
    renderFrame();
    startCycle();
  }

  function pauseCycle() {
    cyclePaused = true;
    stopCycle();
    renderFrame();
  }

  function togglePause() {
    if (frames.length <= 1) return;
    cyclePaused = !cyclePaused;
    renderFrame();
    if (cyclePaused) stopCycle();
    else startCycle();
  }

  function moveFrame(delta) {
    if (!frames.length) return;
    pauseCycle();
    frameIndex = (frameIndex + frames.length + delta) % frames.length;
    renderFrame();
  }

  async function copyCurrentFrameHex() {
    if (!frames.length) return;
    pauseCycle();
    const hex = frames[frameIndex].payloadHex;
    const copyStatus = $('response-copy-status');
    try {
      await navigator.clipboard.writeText(hex);
      copyStatus.textContent = 'Copied to clipboard';
      copyStatus.classList.add('copied');
      setTimeout(() => {
        if (copyStatus.textContent === 'Copied to clipboard') copyStatus.textContent = '';
        copyStatus.classList.remove('copied');
      }, 2200);
    } catch (error) {
      copyStatus.textContent = 'Copy failed';
      copyStatus.classList.remove('copied');
      status(`Could not copy QR Frame Hex: ${error}`, true);
    }
  }

  async function copyAllFrameHex() {
    if (!frames.length) return;
    pauseCycle();
    const copyStatus = $('response-copy-status');
    try {
      await navigator.clipboard.writeText(frames.map(frame => frame.payloadHex).join('\n'));
      copyStatus.textContent = frames.length === 1 ? 'Copied 1 frame' : `Copied all ${frames.length} frames`;
      copyStatus.classList.add('copied');
      setTimeout(() => {
        if (/^Copied/.test(copyStatus.textContent)) copyStatus.textContent = '';
        copyStatus.classList.remove('copied');
      }, 2200);
    } catch (error) {
      copyStatus.textContent = 'Copy failed';
      copyStatus.classList.remove('copied');
      status(`Could not copy all QR Frame Hex values: ${error}`, true);
    }
  }

  async function saveResponseQrImages() {
    if (!frames.length) throw new Error('No QR response frames are available.');
    pauseCycle();
    const prefix = antiKleptoAwaitingReveal ? 'kaskold-nonce-commitment' : 'kaskold-signed';
    for (let index = 0; index < frames.length; index += 1) {
      const blob = await svgToPngBlob(frames[index].svg);
      downloadBlob(blob, responseImageName(prefix, index, frames.length));
      if (index + 1 < frames.length) await sleep(120);
    }
    status(`${frames.length} QR image${frames.length === 1 ? '' : 's'} saved locally.`);
  }

  function approve() {
    const nowUnix = Math.floor(Date.now() / 1000);
    frames = parse(api('kaskold_vault_approve')(nowUnix));
    antiKleptoAwaitingReveal = Boolean(api('kaskold_vault_anti_klepto_awaiting_reveal')());
    frameIndex = 0;
    $('response-scan-reveal').classList.toggle('hidden', !antiKleptoAwaitingReveal);
    $('response-save').textContent = antiKleptoAwaitingReveal ? 'Save Commitment QR Images' : 'Save QR Images';
    $('response-done').classList.toggle('hidden', antiKleptoAwaitingReveal);
    showResponse();
  }

  function rejectAndHome() {
    reset();
    api('kaskold_vault_reject')();
    renderHome(false);
  }

  function openAntiKleptoRevealGuide() {
    stopCycle();
    api('kaskold_vault_begin_anti_klepto_reveal')();
    show('anti-klepto-reveal');
  }

  function scanAntiKleptoReveal() {
    openScanner(
      'Anti-Klepto Host Reveal',
      'Scan the reveal generated for the nonce commitment just returned by this Vault.',
      bytes => {
        try {
          stopCamera();
          frames = parse(api('kaskold_vault_finalize_anti_klepto_reveal')(bytes, Math.floor(Date.now() / 1000)));
          antiKleptoAwaitingReveal = false;
          frameIndex = 0;
          $('response-scan-reveal').classList.add('hidden');
          $('response-save').textContent = 'Save QR Images';
          $('response-done').classList.remove('hidden');
          showResponse();
          status('Host reveal verified. Anti-klepto signatures finalized.');
        } catch (error) { status(String(error), true); }
      },
      () => api('kaskold_vault_reject')(),
    );
  }

  $('begin-scan').onclick = () => {
    try {
      scanBegun = false;
      openScanner(
        'Scan QR',
        'Scan the signing request from KasKold Companion.',
        acceptFrame,
        () => api('kaskold_vault_reject')(),
      );
    } catch (error) { status(String(error), true); }
  };
  $('review-unit-toggle').onclick = toggleReviewUnit;
  $('review-reject').onclick = rejectAndHome;
  $('review-inspect').onclick = () => { try { renderInspection(); } catch (error) { status(String(error), true); } };
  $('review-inspect-done').onclick = () => show('review');
  $('review-approve').onclick = () => { try { approve(); } catch (error) { status(String(error), true); } };
  $('response-prev').onclick = () => moveFrame(-1);
  $('response-pause').onclick = togglePause;
  $('response-next').onclick = () => moveFrame(1);
  $('response-copy-all').onclick = () => { void copyAllFrameHex(); };
  $('response-hex').onclick = () => { void copyCurrentFrameHex(); };
  $('response-hex').onkeydown = event => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    void copyCurrentFrameHex();
  };
  $('response-scan-reveal').onclick = openAntiKleptoRevealGuide;
  $('anti-klepto-scan').onclick = scanAntiKleptoReveal;
  $('anti-klepto-cancel').onclick = rejectAndHome;
  $('response-save').onclick = () => {
    saveResponseQrImages().catch(error => status(String(error), true));
  };
  $('response-done').onclick = rejectAndHome;

  return {
    acceptFrame, renderReview, reset,
    setSpecialRouter(router) { specialRouter = router; },
  };
}
