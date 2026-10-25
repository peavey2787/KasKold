/**
 * Display a generated QR SVG as an inert image.
 *
 * The SVG never enters the DOM as markup: it is loaded as an image, where
 * scripts and event attributes cannot run, so the renderer needs no sanitizer.
 */

const SVG_OPENING = '<svg';

/**
 * @param {string} svg SVG text produced by the WASM QR encoder
 * @param {string} [alt]
 * @returns {HTMLImageElement}
 */
export function qrImage(svg, alt = 'QR code') {
    const text = String(svg ?? '');
    if (!text.startsWith(SVG_OPENING)) throw new Error('QR encoder returned no SVG');
    const image = document.createElement('img');
    image.className = 'qr-svg-image';
    image.alt = alt;
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(text)}`;
    return image;
}

/** Replace a container's content with one QR image. */
export function showQrSvg(container, svg, alt) {
    if (!container) return;
    container.replaceChildren(qrImage(svg, alt));
}
