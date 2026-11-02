# Privacy notice (draft)

**Status: draft for legal review. It describes what the software does today; it
has not been reviewed by counsel and must not ship as a final notice until it has.**

KasKold is non-custodial. No KasKold server holds your keys, your recovery
phrase, your passphrase or your wallet balance, and there is no account to create.
The apps carry no advertising or analytics code.

## The signing device

The KasKold hardware firmware has no network stack. It exchanges data only through
QR codes you scan or display and microSD cards you insert. Your secrets stay on the
device, encrypted when you choose to save them there, or on media you create.

## KasKold Vault for Android

The Vault app declares no internet permission, so the operating system gives it no
way to reach a network. Its data stays on the phone, and any backup you export goes
where you put it.

## KasKold Companion (web and Android)

The Companion is a watch-only wallet. It never holds your private keys, but it has
to reach the network to show balances and broadcast transactions. It stores its
settings, labels and watch-only data locally in your browser or app, and they
are never sent to KasKold.

Each service below sees your IP address and the request itself, as any web server
does. Each operator applies its own privacy terms. Setting your own node in the
Companion replaces the public Kaspa nodes.

| Service | Operator | When it is used | What it receives |
| --- | --- | --- | --- |
| Public Kaspa nodes (`*.kaspa.stream`, `*.kaspa.red`, `*.kaspa.green`, `*.kaspa.blue`), or a node you choose | Community operators | Whenever the Companion is connected | Your wallet addresses, and transactions you broadcast |
| `api.kaspa.org` and explorer APIs for the selected network | Kaspa ecosystem operators | Balances, history, used-address detection, covenant recovery | Your wallet addresses and covenant addresses |
| `api.kasplex.org`, `krc721.stream` | Token indexer operators | Viewing KRC-20 and KRC-721 assets | Your wallet addresses |
| `api.coingecko.com` | CoinGecko | Showing prices and portfolio charts | The asset and currency requested, not your addresses |
| `api.open-meteo.com`, `geocoding-api.open-meteo.com` | Open-Meteo | Only when you use the weather cover in the Android app | The city you search for and the coordinates of the city you pick; the app does not read your device location |
| `keeper.kaskold.com` | KasKold | Only if you turn on the stealth indexer in Settings (off by default), and when you use the Oracle Model B prover | Neither the stealth index download nor the Oracle Model B price quote names an address of yours |

Anyone watching the public Kaspa network can see the addresses and amounts of every
transaction. That is a property of the network, not of this software.

## Oracle Model B fee

Each new Oracle Model B price roll adds a service fee of 0.3 KAS, paid to the KasKold fee
address `kaspa:qq5zdr7cwuyrqu0zmr03v3qx0tnf6psmangl4f9aecp8a4xkjmz0x5e2ejesy`. The
fee is an output of a transaction you review and sign, and it is public on the
network. No other KasKold feature charges a fee.

## Points for legal review

- Operator of `keeper.kaskold.com` as a legal entity, its contact address, and its
  server log retention.
- Whether regional law treats any of the above as personal data processing by
  KasKold, and what notices, rights or age terms follow from that.
- Wording of the Oracle Model B fee disclosure and any related consumer or
  financial-services obligations.
