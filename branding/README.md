[KasKold](../README.md) › [Documentation](../docs/README.md) › Branding

# KasKold brand assets

`branding/source/` contains the canonical raster artwork supplied for KasKold:

- `kaskold-mark.png` — circular mirrored-K product mark.
- `kaskold-favicon.png` — compact rounded-square single-K mark optimized for tiny web icon sizes.
- `kaskold-wordmark.png` — KasKold wordmark with “Keep your Kas safe!”.
- `kaskold-vault-wordmark.png` — cropped KasKold Vault wordmark used by the Vault web, Android, and iOS headers.

The current source artwork is raster-only, so generated platform assets are
reproducible derivatives of those exact PNGs rather than independently redrawn
logos. The web favicon is derived from `kaskold-favicon.png`; the other platform
mark assets are derived from `kaskold-mark.png`. Do not hand-edit platform copies;
regenerate them from the matching canonical source.

Derived assets are grouped under `branding/web/`, `branding/android/`,
`branding/ios/`, and `branding/firmware/`.
