// Pure BlockAdded-notification parsing helpers.
import { hexToBytes } from '../../../core/bytes.js';

function readUint32LE(data, offset) {
    return (
        data[offset]
        | (data[offset + 1] << 8)
        | (data[offset + 2] << 16)
        | (data[offset + 3] << 24)
    ) >>> 0;
}

function matchesOutpoint(data, offset, txidBytes, outputIndex) {
    if (offset + 41 > data.length) return false;
    if (data[offset] !== 37 || data[offset + 1] !== 0 || data[offset + 2] !== 0 || data[offset + 3] !== 0) return false;
    if (data[offset + 4] !== 0x01) return false;
    for (let index = 0; index < txidBytes.length; index++) {
        if (data[offset + 5 + index] !== txidBytes[index]) return false;
    }
    const encodedIndex = readUint32LE(data, offset + 37);
    return encodedIndex === (Number(outputIndex || 0) >>> 0);
}

function hasSpendingEnvelope(data) {
    const envelopeOffset = data[0] === 0x01 ? 9 : 1;
    return envelopeOffset + 2 < data.length && data[envelopeOffset] === 0xFF && data[envelopeOffset + 2] === 0x3C;
}

function scriptAtOutpoint(data, offset, txidBytes, outpoint, minLength, maxLength) {
    if (!matchesOutpoint(data, offset, txidBytes, outpoint.index)) return null;
    const lengthOffset = offset + 41;
    if (lengthOffset + 4 > data.length) return null;
    const scriptLength = readUint32LE(data, lengthOffset);
    if (scriptLength < minLength || scriptLength > maxLength) return null;
    const scriptOffset = lengthOffset + 4;
    if (scriptOffset + scriptLength > data.length) return null;
    return data.slice(scriptOffset, scriptOffset + scriptLength);
}

function isSearchableEnvelope(data, outpoint) {
    return data instanceof Uint8Array && data.length >= 4 && Boolean(outpoint) && hasSpendingEnvelope(data);
}

function outpointTxidBytes(outpoint) {
    if (typeof outpoint.txid !== 'string' || !/^[0-9a-f]{64}$/i.test(outpoint.txid)) return null;
    return hexToBytes(outpoint.txid);
}

export function findSpendingSignatureScript(data, outpoint, bounds) {
    if (!isSearchableEnvelope(data, outpoint)) return null;
    const txidBytes = outpointTxidBytes(outpoint);
    if (txidBytes === null) return null;
    const minLength = bounds?.minLength ?? 1;
    const maxLength = bounds?.maxLength ?? 2000;

    for (let offset = 4; offset + 45 <= data.length; offset++) {
        const script = scriptAtOutpoint(data, offset, txidBytes, outpoint, minLength, maxLength);
        if (script !== null) return script;
    }
    return null;
}

export function readFirstPush(script, maxLength = 200) {
    if (!(script instanceof Uint8Array) || script.length === 0) return null;
    const opcode = script[0];
    let dataOffset;
    let dataLength;
    if (opcode >= 1 && opcode <= 0x4B) {
        dataOffset = 1;
        dataLength = opcode;
    } else if (opcode === 0x4C && script.length >= 2) {
        dataOffset = 2;
        dataLength = script[1];
    } else {
        return null;
    }
    if (dataLength === 0 || dataLength > maxLength || dataOffset + dataLength > script.length) return null;
    return script.slice(dataOffset, dataOffset + dataLength);
}
