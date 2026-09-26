import { showScreen } from '../../../app/navigation.js';
import { bytesToHex, hexToBytes } from '../../../core/bytes.js';
import { decodeQrImageFile } from '../../../core/qr/image_file.js';
import { byId } from '../../../core/dom.js';
import { toast } from '../../../core/ui/toast.js';
import { stopScanner } from '../../stealth/index/camera.js';
import { decode_qr_frame, reset_qr_decoder } from '../../../wasm/api.js';

const STORAGE_KEY = 'kaskold-companion-multisig-descriptors-v1';
const MAX_DESCRIPTORS = 32;

function validDescriptor(text) {
    const value = String(text || '').trim();
    return value.startsWith('multi(') || value.startsWith('multi_hd(') || value.startsWith('multi_hd45(');
}


function normalizedDescriptor(text) {
    const value = String(text || '').trim();
    return validDescriptor(value) ? value : '';
}

function descriptorFromBytes(bytes) {
    const raw = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes || []);
    const direct = normalizedDescriptor(new TextDecoder().decode(raw));
    if (direct) return direct;
    const result = decode_qr_frame(bytesToHex(raw));
    if (!result) return '';
    return normalizedDescriptor(new TextDecoder().decode(hexToBytes(result)));
}

function populateManagedDescriptor(descriptor, message) {
    byId('input-ms-descriptor-saved').value = descriptor;
    if (message) toast(message, 'ok', 1600);
}

export function handleSavedDescriptorScan(data) {
    try {
        const descriptor = descriptorFromBytes(new Uint8Array(data));
        if (!descriptor) return false;
        stopScanner();
        populateManagedDescriptor(descriptor, 'Descriptor scanned');
        return true;
    } catch (error) {
        console.error('Descriptor scan error:', error);
        toast('Could not read descriptor QR', 'error', 3000);
        return false;
    }
}

export async function uploadSavedDescriptorQrImage(file) {
    if (!file) return false;
    try {
        const code = await decodeQrImageFile(file);
        let descriptor = normalizedDescriptor(code?.data);
        if (!descriptor && code?.binaryData?.length) {
            try { reset_qr_decoder(); } catch (_) {}
            descriptor = descriptorFromBytes(new Uint8Array(code.binaryData));
        }
        if (!descriptor) throw new Error('The QR image does not contain a valid multisig descriptor');
        populateManagedDescriptor(descriptor, 'Descriptor QR image loaded');
        return true;
    } catch (error) {
        toast(`Descriptor QR import failed: ${error.message || error}`, 'error', 5000);
        return false;
    } finally {
        try { reset_qr_decoder(); } catch (_) {}
    }
}

function loadAll() {
    try {
        const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
        if (!Array.isArray(parsed)) return [];
        return parsed.filter(item => item && typeof item.name === 'string' && typeof item.descriptor === 'string').slice(0, MAX_DESCRIPTORS);
    } catch (_) {
        return [];
    }
}

function saveAll(items) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items.slice(0, MAX_DESCRIPTORS)));
}

export function renderSavedMultisigSpendOptions() {
    const select = byId('select-ms-saved-descriptor');
    const descriptorInput = byId('input-ms-descriptor');
    if (!select || !descriptorInput) return;

    const items = loadAll();
    const current = descriptorInput.value.trim();
    select.replaceChildren();

    const placeholder = document.createElement('option');
    placeholder.value = '';
    placeholder.textContent = items.length === 0 ? 'No saved descriptors yet' : 'Choose a saved descriptor...';
    select.appendChild(placeholder);
    select.disabled = items.length === 0;

    items.forEach((item, index) => {
        const option = document.createElement('option');
        option.value = String(index);
        option.textContent = item.name;
        select.appendChild(option);
        if (item.descriptor === current) select.value = option.value;
    });
}

export function useSelectedMultisigDescriptor() {
    const select = byId('select-ms-saved-descriptor');
    const index = Number.parseInt(select?.value ?? '', 10);
    if (!Number.isInteger(index) || index < 0) return false;
    const item = loadAll()[index];
    if (!item || !validDescriptor(item.descriptor)) return false;
    byId('input-ms-descriptor').value = item.descriptor;
    toast(`${item.name} loaded`, 'ok', 1500);
    return true;
}

export function renderSavedMultisigDescriptors() {
    const root = byId('ms-descriptor-list');
    root.replaceChildren();
    const items = loadAll();
    if (items.length === 0) {
        const empty = document.createElement('div');
        empty.className = 'send-balance-ref';
        empty.textContent = 'No saved descriptors yet.';
        root.appendChild(empty);
        return;
    }
    items.forEach((item, index) => {
        const row = document.createElement('div');
        row.className = 'card u-mt-8px';
        const name = document.createElement('strong');
        name.textContent = item.name;
        const preview = document.createElement('div');
        preview.className = 'send-balance-ref u-mt-4px';
        preview.textContent = item.descriptor;
        const use = document.createElement('button');
        use.className = 'btn btn-secondary u-mt-8px';
        use.textContent = 'Use for Spend';
        use.onclick = () => {
            byId('input-ms-descriptor').value = item.descriptor;
            renderSavedMultisigSpendOptions();
            showScreen('multisig-spend');
        };
        const remove = document.createElement('button');
        remove.className = 'btn btn-link btn-danger';
        remove.textContent = 'Delete';
        remove.onclick = () => {
            const next = loadAll();
            next.splice(index, 1);
            saveAll(next);
            renderSavedMultisigDescriptors();
            renderSavedMultisigSpendOptions();
        };
        row.append(name, preview, use, remove);
        root.appendChild(row);
    });
}

export function saveMultisigDescriptor() {
    const name = byId('input-ms-descriptor-name').value.trim();
    const descriptor = byId('input-ms-descriptor-saved').value.trim();
    if (!name) { toast('Enter a friendly name', 'error'); return; }
    if (!validDescriptor(descriptor)) { toast('Enter a valid multisig descriptor', 'error'); return; }
    const items = loadAll();
    const duplicate = items.findIndex(item => item.descriptor === descriptor);
    const record = { name: name.slice(0, 64), descriptor };
    if (duplicate >= 0) items[duplicate] = record;
    else {
        if (items.length >= MAX_DESCRIPTORS) { toast(`You can save up to ${MAX_DESCRIPTORS} descriptors`, 'error'); return; }
        items.push(record);
    }
    saveAll(items);
    byId('input-ms-descriptor-name').value = '';
    byId('input-ms-descriptor-saved').value = '';
    renderSavedMultisigDescriptors();
    renderSavedMultisigSpendOptions();
    toast('Descriptor saved', 'ok', 1500);
}
