// Show runtime-generated QR SVGs as inert images: the SVG never reaches the
// HTML parser, so the page needs no markup sanitizer.

export function showQrSvg(target, svg, alt = 'QR code') {
  if (!target) return;
  const text = String(svg ?? '');
  if (!text) {
    target.replaceChildren();
    return;
  }
  if (!text.startsWith('<svg')) throw new Error('Vault QR renderer returned no SVG.');
  const image = document.createElement('img');
  image.alt = alt;
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(text)}`;
  target.replaceChildren(image);
}
