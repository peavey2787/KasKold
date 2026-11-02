import { newId } from './repository.js';
import { parseKas, usdToMicro, microToUsd, formatKas } from './exact_money.js';

const TYPES = new Set(['Buy', 'Sell', 'Transfer In', 'Transfer Out']);
const HEADER = ['type', 'kas_amount', 'kas_price_usd', 'fee_usd', 'timestamp', 'notes', 'source_tx_id'];

function pushNonBlankRecord(records, row) {
    if (row.some(value => value.trim())) records.push(row);
}

function endCsvRow(state) {
    state.row.push(state.cell);
    pushNonBlankRecord(state.records, state.row);
    state.row = [];
    state.cell = '';
}

// Returns how many extra source characters the quote handling consumed.
function consumeQuote(state, source, index) {
    if (state.quoted && source[index + 1] === '"') { state.cell += '"'; return 1; }
    state.quoted = !state.quoted;
    return 0;
}

// Returns how many extra source characters the line break consumed (CRLF pairs).
function consumeLineBreak(state, source, index) {
    const pairedCrLf = source[index] === '\r' && source[index + 1] === '\n';
    endCsvRow(state);
    return pairedCrLf ? 1 : 0;
}

function parseRecords(text) {
    const state = { records: [], row: [], cell: '', quoted: false };
    const source = String(text ?? '');
    for (let index = 0; index < source.length; index += 1) {
        const char = source[index];
        if (char === '"') { index += consumeQuote(state, source, index); continue; }
        if (state.quoted) { state.cell += char; continue; }
        if (char === ',') { state.row.push(state.cell); state.cell = ''; continue; }
        if (char === '\n' || char === '\r') { index += consumeLineBreak(state, source, index); continue; }
        state.cell += char;
    }
    if (state.quoted) throw new Error('CSV contains an unterminated quoted field');
    endCsvRow(state);
    return state.records;
}

function csvEscape(value) {
    const text = String(value ?? '');
    return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function parseType(value) {
    const type = String(value ?? '').trim();
    if (!TYPES.has(type)) throw new Error(`Unsupported transaction type: ${type}`);
    return type;
}

function parseTimestamp(value) {
    const timestampMs = Date.parse(String(value ?? '').trim());
    if (!Number.isFinite(timestampMs)) throw new Error(`Invalid transaction timestamp: ${value}`);
    return timestampMs;
}

export function parsePortfolioCsv(text, portfolioId) {
    const records = parseRecords(text);
    if (!records.length) return [];
    const header = records[0].map(value => value.trim().toLowerCase());
    if (HEADER.some((field, index) => header[index] !== field)) throw new Error(`CSV header must be: ${HEADER.join(',')}`);
    return records.slice(1).map((cells, index) => {
        if (cells.length !== HEADER.length) throw new Error(`CSV row ${index + 2} has ${cells.length} fields; expected ${HEADER.length}`);
        return {
            id: newId(), portfolioId, type: parseType(cells[0]),
            kasSompi: parseKas(cells[1]).toString(),
            priceMicroUsd: usdToMicro(cells[2] || '0', 'CSV KAS price').toString(),
            feeMicroUsd: usdToMicro(cells[3] || '0', 'CSV fee').toString(),
            timestampMs: parseTimestamp(cells[4]), notes: cells[5].slice(0, 500),
            sourceTxId: cells[6].trim() || null, createdAt: Date.now() + index,
        };
    });
}

export function exportPortfolioCsv(entries) {
    const rows = [HEADER.join(',')];
    for (const entry of entries) {
        rows.push([
            entry.type,
            formatKas(entry.kasSompi),
            microToUsd(entry.priceMicroUsd, 6),
            microToUsd(entry.feeMicroUsd || '0', 6),
            new Date(entry.timestampMs).toISOString(),
            entry.notes || '',
            entry.sourceTxId || '',
        ].map(csvEscape).join(','));
    }
    return `${rows.join('\n')}\n`;
}
