# Getting Started — KasKold Hardware

KasKold Hardware is the dedicated offline signer in the KasKold product family. The qualified target is **M5Stack CoreS3**. **CoreS3 Lite is build-supported but remains hardware-qualification pending** until its full device test matrix has been completed.

## Basic flow

1. Build or install KasKold Hardware for the correct CoreS3 profile.
2. Create or restore the wallet on the offline device.
3. From **Connect Companion**, export the public account (`kpub`) as QR.
4. Load that public account in KasKold Companion. Companion remains watch-only for that account.
5. Prepare a transaction in Companion and display its KSPT QR request.
6. Scan the request on Hardware, review the transaction on the signer, approve it, and scan the signed response back into Companion.
7. Companion merges/finalizes the signature and broadcasts.

KasKold Hardware never needs blockchain networking. Keep seed/private-key material on the signer and verify destination, amount, fee, and network on the signer before approval.

See [CoreS3](../hardware/CORES3.md), [CoreS3 Lite](../hardware/CORES3_LITE.md), and [Pop It!](../hardware/POP_IT.md).
