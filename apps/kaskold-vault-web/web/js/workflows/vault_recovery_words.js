export function normalizeRecoveryPhraseInput(value) {
  const text = String(value ?? '').trim();
  if (!text) return '';

  const lines = text.split(/\r?\n/).map(line => line.trim()).filter(Boolean);
  const hasNumberedLine = lines.some(line => /^\d+\s*[.)]\s*/.test(line));
  if (!hasNumberedLine) {
    return text.split(/\s+/).filter(Boolean).map(word => word.toLowerCase()).join(' ');
  }

  const words = lines.map((line, index) => {
    const match = line.match(/^(\d+)\s*[.)]\s+([A-Za-z]+)\s*$/);
    if (!match) {
      throw new Error('Numbered recovery words must use one "N. word" entry per line.');
    }
    const position = Number(match[1]);
    if (position !== index + 1) {
      throw new Error(`Numbered recovery words must be sequential; expected ${index + 1}.`);
    }
    return match[2].toLowerCase();
  });

  if (words.length !== 12 && words.length !== 24) {
    throw new Error('Recovery phrase must contain 12 or 24 words.');
  }
  return words.join(' ');
}

export function createRecoveryWordPager({ $, api, parse, show, goBack, history }) {
  let words = [];
  let index = 0;
  let requiresAcknowledgement = false;
  let showingAll = false;
  let done = null;

  function render() {
    if (!words.length) throw new Error('No recovery words are available.');
    $('backup-word-position').textContent = `Word ${index + 1} of ${words.length}`;
    $('backup-word').textContent = words[index];
    $('backup-prev').disabled = index === 0;
    $('backup-next').textContent = index + 1 === words.length
      ? (requiresAcknowledgement ? 'Continue' : 'Done')
      : 'Next';
  }

  function showOneAtATime() {
    showingAll = false;
    $('backup-all-words').classList.add('hidden');
    $('backup-word').classList.remove('hidden');
    $('backup-word-position').classList.remove('hidden');
    $('backup-prev').disabled = index === 0;
    $('backup-next').disabled = false;
    $('backup-show-all').textContent = 'Show All';
    render();
  }

  function toggleAll() {
    if (!$('backup-all-words').classList.contains('hidden')) {
      showOneAtATime();
      return;
    }
    showingAll = true;
    $('backup-all-words').textContent = words.map((word, wordIndex) => `${wordIndex + 1}. ${word}`).join('\n');
    $('backup-all-words').classList.remove('hidden');
    $('backup-word').classList.add('hidden');
    $('backup-word-position').classList.add('hidden');
    $('backup-prev').disabled = true;
    $('backup-next').disabled = false;
    $('backup-next').textContent = requiresAcknowledgement ? 'Continue' : 'Done';
    $('backup-show-all').textContent = 'Show One at a Time';
  }

  function start(phrase, { acknowledgement = false, onDone = null, resetHistory = false } = {}) {
    const nextWords = String(phrase || '').trim().split(/\s+/).filter(Boolean);
    if (nextWords.length !== 12 && nextWords.length !== 24) {
      throw new Error('Recovery phrase must contain 12 or 24 words.');
    }
    words = nextWords;
    index = 0;
    requiresAcknowledgement = acknowledgement;
    showingAll = false;
    done = onDone;
    if (resetHistory) history.length = 0;
    $('backup-all-words').classList.add('hidden');
    $('backup-word').classList.remove('hidden');
    $('backup-word-position').classList.remove('hidden');
    $('backup-show-all').textContent = 'Show All';
    show('backup', !resetHistory);
    render();
  }

  function clear() {
    words.fill('');
    words = [];
    index = 0;
    requiresAcknowledgement = false;
    showingAll = false;
    done = null;
    for (const id of ['backup-word', 'backup-all-words', 'backup-word-position']) $(id).textContent = '';
    $('backup-all-words').classList.add('hidden');
    $('backup-word').classList.remove('hidden');
    $('backup-word-position').classList.remove('hidden');
    $('backup-show-all').textContent = 'Show All';
    $('backup-prev').disabled = false;
    $('backup-next').disabled = false;
  }

  function reveal() {
    const { value } = parse(api('kaskold_vault_backup_words')());
    start(value, { onDone: goBack });
  }

  function install() {
    $('backup-show-all').onclick = toggleAll;
    $('backup-prev').onclick = () => { if (index > 0) { index -= 1; render(); } };
    $('backup-next').onclick = () => {
      if (!showingAll && index + 1 < words.length) { index += 1; render(); return; }
      if (requiresAcknowledgement) { show('backup-ack'); return; }
      const callback = done;
      clear();
      if (callback) callback(); else goBack();
    };
  }

  return { start, clear, reveal, install };
}
