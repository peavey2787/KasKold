# KasKold Vault Air-Gap Model

Vault treats the camera/display QR channel as its transaction transport and intentionally omits blockchain networking.

A signing request is assembled by the session-bound `kaskold-protocol` QR decoder. Mixed/conflicting sessions fail closed. Completion only moves a request into **review** state; it does not authorize a signature. A separate explicit approval call signs the reviewed KSPT and produces session-bound response frames. Reject, lock, or starting another scan clears pending request/response state.

Companion handles balance lookup, transaction construction, network access, finalization, and broadcast. Vault handles custody, derivation, transaction validation/review, and signing.
