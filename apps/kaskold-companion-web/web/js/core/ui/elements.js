/**
 * DOM construction without HTML parsing.
 *
 * Dynamic UI is built from elements whose text is always assigned as text, so
 * values that originate from a node, an invite or a QR can never become markup.
 * Attribute names are an allowlist and links accept only http(s) targets.
 */

const ATTRIBUTES = new Set([
    'alt', 'aria-expanded', 'aria-hidden', 'aria-label', 'for', 'href', 'max', 'min',
    'name', 'placeholder', 'rel', 'role', 'step', 'tabindex', 'target', 'title', 'type', 'value',
]);

function safeHref(value) {
    try {
        const url = new URL(String(value));
        return url.protocol === 'https:' || url.protocol === 'http:' ? url.href : '';
    } catch (_) {
        return '';
    }
}

function setAttribute(node, name, value) {
    if (!ATTRIBUTES.has(name)) throw new Error(`Unsupported element attribute: ${name}`);
    if (value === null || value === undefined || value === false) return;
    const text = name === 'href' ? safeHref(value) : String(value);
    if (name === 'href' && !text) return;
    node.setAttribute(name, text);
}

function appendChildren(node, children) {
    for (const child of children.flat(Infinity)) {
        if (child === null || child === undefined || child === false) continue;
        node.append(typeof child === 'object' ? child : document.createTextNode(String(child)));
    }
}

/**
 * Build an element.
 *
 * @param {string} tag
 * @param {{className?: string, id?: string, dataset?: Record<string, unknown>,
 *          attrs?: Record<string, unknown>, checked?: boolean, disabled?: boolean}} [options]
 * @param {...(Node|string|number|bigint|null|undefined|false|Array<unknown>)} children
 * @returns {HTMLElement}
 */
export function el(tag, options = {}, ...children) {
    const node = document.createElement(tag);
    if (options.className) node.className = options.className;
    if (options.id) node.id = options.id;
    for (const [key, value] of Object.entries(options.dataset || {})) {
        if (value !== null && value !== undefined) node.dataset[key] = String(value);
    }
    for (const [name, value] of Object.entries(options.attrs || {})) setAttribute(node, name, value);
    if (options.checked !== undefined) node.checked = Boolean(options.checked);
    if (options.disabled !== undefined) node.disabled = Boolean(options.disabled);
    appendChildren(node, children);
    return node;
}

/** Replace an element's content with built nodes or plain text. */
export function setContent(target, ...children) {
    if (!target) return;
    target.replaceChildren();
    appendChildren(target, children);
}

/** A span with a utility class, the most common status fragment. */
export function span(className, ...children) {
    return el('span', { className }, ...children);
}
