#!/usr/bin/env python3
"""Generate the short user-facing KasKold PDF guides from repository facts.

The detailed Markdown documentation remains authoritative. These PDFs are
compact printable operator guides and deliberately avoid release claims that
require physical-device evidence.
"""
from __future__ import annotations

from pathlib import Path
from typing import Iterable, Sequence

from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import (
    BaseDocTemplate,
    Frame,
    Image,
    KeepTogether,
    PageBreak,
    PageTemplate,
    Paragraph,
    Spacer,
    Table,
    TableStyle,
)

ROOT = Path(__file__).resolve().parents[3]
GUIDES = ROOT / "docs/guides"
LOGO = ROOT / "branding/source/kaskold-mark.png"
VERSION = "2.0.0"

PAGE_W, PAGE_H = letter
MARGIN_X = 0.58 * inch
MARGIN_TOP = 0.42 * inch
MARGIN_BOTTOM = 0.48 * inch
TEAL = colors.HexColor("#08B7C7")
BLUE = colors.HexColor("#118AEF")
INK = colors.HexColor("#111827")
BODY = colors.HexColor("#273449")
MUTED = colors.HexColor("#5F6C80")
PANEL = colors.HexColor("#EDF8FB")
HEADER = colors.HexColor("#08111D")
GRID = colors.HexColor("#AEBBCD")

styles = getSampleStyleSheet()
TITLE = ParagraphStyle(
    "KasKoldTitle",
    parent=styles["Title"],
    fontName="Helvetica-Bold",
    fontSize=21,
    leading=24,
    textColor=INK,
    alignment=TA_CENTER,
    spaceAfter=6,
)
SUBTITLE = ParagraphStyle(
    "KasKoldSubtitle",
    parent=styles["Normal"],
    fontName="Helvetica",
    fontSize=10.5,
    leading=13,
    textColor=BODY,
    alignment=TA_CENTER,
    spaceAfter=10,
)
H1 = ParagraphStyle(
    "KasKoldH1",
    parent=styles["Heading2"],
    fontName="Helvetica-Bold",
    fontSize=14.5,
    leading=17,
    textColor=INK,
    spaceBefore=7,
    spaceAfter=5,
)
BODY_STYLE = ParagraphStyle(
    "KasKoldBody",
    parent=styles["BodyText"],
    fontName="Helvetica",
    fontSize=9.1,
    leading=12.3,
    textColor=BODY,
    spaceAfter=4,
)
BULLET = ParagraphStyle(
    "KasKoldBullet",
    parent=BODY_STYLE,
    leftIndent=10,
    firstLineIndent=-7,
    bulletIndent=2,
    spaceAfter=3,
)
CALLOUT = ParagraphStyle(
    "KasKoldCallout",
    parent=BODY_STYLE,
    fontName="Helvetica-Bold",
    fontSize=8.9,
    leading=11.3,
    textColor=INK,
)
TABLE_HEAD = ParagraphStyle(
    "KasKoldTableHead",
    parent=BODY_STYLE,
    fontName="Helvetica-Bold",
    textColor=colors.white,
    fontSize=8.0,
    leading=10,
)
TABLE_CELL = ParagraphStyle(
    "KasKoldTableCell",
    parent=BODY_STYLE,
    fontSize=7.8,
    leading=9.8,
    spaceAfter=0,
)


def p(text: str, style=BODY_STYLE) -> Paragraph:
    return Paragraph(text, style)


def bullet(text: str) -> Paragraph:
    return Paragraph(f"- {text}", BULLET)


def section(number: int, title: str, body: Sequence[object]) -> list[object]:
    return [KeepTogether([p(f"{number}. {title}", H1), *body])]


def callout(text: str) -> Table:
    box = Table([[p(text, CALLOUT)]], colWidths=[PAGE_W - 2 * MARGIN_X])
    box.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), PANEL),
        ("BOX", (0, 0), (-1, -1), 0.8, BLUE),
        ("LEFTPADDING", (0, 0), (-1, -1), 8),
        ("RIGHTPADDING", (0, 0), (-1, -1), 8),
        ("TOPPADDING", (0, 0), (-1, -1), 6),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
    ]))
    return box


def data_table(headers: Sequence[str], rows: Sequence[Sequence[str]], widths: Sequence[float]) -> Table:
    data = [[p(h, TABLE_HEAD) for h in headers]]
    data += [[p(cell, TABLE_CELL) for cell in row] for row in rows]
    table = Table(data, colWidths=list(widths), repeatRows=1)
    table.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, 0), HEADER),
        ("GRID", (0, 0), (-1, -1), 0.45, GRID),
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("LEFTPADDING", (0, 0), (-1, -1), 5),
        ("RIGHTPADDING", (0, 0), (-1, -1), 5),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
    ]))
    return table


def header_footer(canvas, doc) -> None:
    canvas.saveState()
    canvas.setStrokeColor(TEAL)
    canvas.setLineWidth(1.4)
    canvas.line(0, PAGE_H - 0.18 * inch, PAGE_W, PAGE_H - 0.18 * inch)
    canvas.setFont("Helvetica", 7)
    canvas.setFillColor(MUTED)
    canvas.drawString(MARGIN_X - 0.08 * inch, 0.22 * inch, f"KasKold {VERSION}")
    canvas.drawRightString(PAGE_W - MARGIN_X + 0.08 * inch, 0.22 * inch, f"Page {doc.page}")
    canvas.restoreState()


def title_block(title: str, subtitle: str) -> list[object]:
    logo = Image(str(LOGO), width=0.74 * inch, height=0.74 * inch)
    logo.hAlign = "CENTER"
    return [logo, Spacer(1, 4), p(title, TITLE), p(subtitle, SUBTITLE)]


def build(filename: str, title: str, subtitle: str, story: Iterable[object]) -> None:
    out = GUIDES / filename
    frame = Frame(
        MARGIN_X,
        MARGIN_BOTTOM,
        PAGE_W - 2 * MARGIN_X,
        PAGE_H - MARGIN_TOP - MARGIN_BOTTOM,
        id="main",
        topPadding=0,
        bottomPadding=0,
        leftPadding=0,
        rightPadding=0,
    )
    doc = BaseDocTemplate(
        str(out),
        pagesize=letter,
        title=title,
        author="KasKold Project",
        subject=subtitle,
        leftMargin=MARGIN_X,
        rightMargin=MARGIN_X,
        topMargin=MARGIN_TOP,
        bottomMargin=MARGIN_BOTTOM,
    )
    doc.addPageTemplates([PageTemplate(id="guide", frames=[frame], onPage=header_footer)])
    doc.build([*title_block(title, subtitle), *story])


def companion_guide() -> None:
    usable = PAGE_W - 2 * MARGIN_X
    story: list[object] = [
        callout(
            "KasKold Companion is strictly watch-only on Web, Android, and iOS. "
            "Wallet creation, recovery phrases, private keys, and signing belong only to KasKold Hardware or KasKold Vault."
        ),
        *section(1, "Start with kpub management", [
            p("Companion opens on the public-account surface. Load or select a kpub exported by Hardware or Vault. Broadcast TX and Multisig remain available directly from startup."),
            data_table(
                ["Companion can", "Companion cannot"],
                [
                    ["Manage kpubs, addresses, balances, UTXOs and history", "Create or restore a private wallet"],
                    ["Construct ordinary, multisig, covenant and stealth transactions", "Store a mnemonic, seed, xprv or private key"],
                    ["Prepare signing requests and accept signed responses", "Sign locally"],
                    ["Finalize and broadcast", "Unlock a software signer"],
                ],
                [0.5 * usable, 0.5 * usable],
            )
        ]),
        *section(2, "Prepare and authorize a spend", [
            bullet("Enter destination, amount, fee policy, and optional UTXO controls in Companion."),
            bullet("Companion prepares the PSKT/KSPT signing request from public/watch-only state."),
            bullet("Transfer the request to KasKold Hardware or Vault Web/Android/iOS."),
            bullet("Review recipient, amount, fee, change, network, and relevant covenant/multisig context on the signer, then approve there."),
            bullet("Return the signed response to Companion for finalization and broadcast."),
        ]),
        *section(3, "Signer choices", [
            data_table(
                ["Signer", "Role"],
                [
                    ["KasKold Hardware", "Purpose-built offline CoreS3 signer"],
                    ["KasKold Vault Web", "Browser software signer without blockchain watcher/node/broadcast capability"],
                    ["KasKold Vault Android / iOS", "Dedicated network-isolated native signer apps"],
                ],
                [0.35 * usable, 0.65 * usable],
            )
        ]),
        *section(4, "Node, privacy, and safety", [
            bullet("A public Kaspa node can observe queried addresses and can misrepresent network state. Use a trusted/private node when that trade-off matters."),
            bullet("Treat Companion as potentially hostile for authorization purposes: the Hardware/Vault review is the authority for a spend."),
            bullet("Companion never becomes a signer by changing a mode; there is no software-custody mode in Companion."),
        ]),
    ]
    build(
        "Companion_User_Guide.pdf",
        "KasKold Companion User Guide",
        "Watch-only kpub management, transaction preparation, external signing, finalization, and broadcast",
        story,
    )

def quick_start() -> None:
    usable = PAGE_W - 2 * MARGIN_X
    story: list[object] = [
        callout(
            "KasKold is security-critical self-custody software. Keep tested recovery backups and authorize spends on the signer, not on Companion."
        ),
        *section(1, "Choose the application", [
            data_table(
                ["Application", "Role"],
                [
                    ["KasKold Hardware", "Create/restore/review/sign on purpose-built CoreS3 hardware"],
                    ["Vault Web", "Create/restore/review/sign in a browser signer with no blockchain watcher"],
                    ["Vault Android / iOS", "Create/restore/review/sign in dedicated native signer apps"],
                    ["Companion Web / Android / iOS", "Watch-only kpub management, network state, transaction construction and broadcast"],
                ],
                [0.36 * usable, 0.64 * usable],
            )
        ]),
        *section(2, "Create or restore", [
            bullet("Create or restore the wallet only in KasKold Hardware or a Vault app."),
            bullet("Back up the BIP39 mnemonic plus optional passphrase before relying on the wallet."),
            bullet("Export only the public kpub to Companion."),
        ]),
        *section(3, "Use Companion", [
            bullet("Load/manage the kpub on Companion startup."),
            bullet("Use watch-only tools for balances, history, Broadcast TX, Multisig, and transaction construction."),
            bullet("When a signature is needed, transfer the signing request to Hardware/Vault and return the signed response."),
        ]),
        *section(4, "Verify releases and qualification", [
            p("Use <b>make release</b> for the reproducible normal-release build and manifest verification. Secure Boot provisioning is separate and explicit. CoreS3 Lite remains qualification-pending until physical HIL evidence is recorded."),
        ]),
    ]
    build(
        "KasKold_Quick_Start_Guide.pdf",
        "KasKold Quick Start Guide",
        "Hardware and Vault custody, watch-only Companion, recovery, signing, and release verification",
        story,
    )

def security_architecture() -> None:
    usable = PAGE_W - 2 * MARGIN_X
    story: list[object] = [
        callout(
            "Assurance is layered, not absolute: automated QA and reproducible builds do not make consumer hardware tamper resistant or formally verified."
        ),
        *section(1, "Trust and custody boundaries", [
            data_table(
                ["Component", "Security role"],
                [
                    ["KasKold Hardware", "Offline key custody, independent parsing/review/signing, QR/SD exchange"],
                    ["KasKold Vault Web", "Software custody/review/signing; no blockchain watcher/node/broadcast path"],
                    ["KasKold Vault Android / iOS", "Dedicated network-isolated software signer applications"],
                    ["KasKold Companion Web / Android / iOS", "Strictly watch-only online state, transaction construction, finalization and broadcast"],
                ],
                [0.31 * usable, 0.69 * usable],
            )
        ]),
        *section(2, "Secret boundary", [
            bullet("Companion does not depend on hot-wallet, vault-runtime, or offline-signer and cannot create/restore/sign wallets."),
            bullet("Vault applications reuse the shared Rust vault-runtime and hardened signing implementation; UI layers do not reimplement cryptography."),
            bullet("Ordinary signer operations use constrained Rust state and do not expose private-key bytes as a side effect of signing."),
        ]),
        *section(3, "Entropy and key lifecycle", [
            p("Hardware seed creation uses the documented checked hardware entropy pipeline. Vault software wallet creation uses the platform CSPRNG through Rust. Transient buffers are zeroized where the implementation owns them."),
        ]),
        *section(4, "Transaction and QR authorization", [
            bullet("KSPT v4 multi-frame assembly is session-bound and rejects conflicting/mixed framing."),
            bullet("Hardware and Vault parse and review signing intent independently of Companion."),
            bullet("Vault scanning enters review state; signing requires a separate explicit approval action."),
        ]),
        *section(5, "Compatibility identifiers and residual risks", [
            p("Some legacy wire and cryptographic domain identifiers intentionally retain historical KasSigner spelling because renaming them would alter compatibility or cryptographic semantics."),
            bullet("Residual risks include physical/fault attacks, compromised online clients/nodes/toolchains, weak backups/passwords, and unqualified hardware variants."),
        ]),
    ]
    build(
        "KasKold_Security_Architecture.pdf",
        "KasKold Security Architecture",
        "Hardware and Vault custody boundaries, watch-only Companion, key lifecycle, and authorization",
        story,
    )

def user_guide() -> None:
    usable = PAGE_W - 2 * MARGIN_X
    story: list[object] = [
        callout(
            "Authorization rule: Companion is watch-only. Wallet creation/restoration and transaction signing occur only in KasKold Hardware or KasKold Vault."
        ),
        *section(1, "Product family", [
            data_table(
                ["Application", "Purpose"],
                [
                    ["KasKold Hardware", "Purpose-built create/restore/review/sign device"],
                    ["KasKold Vault Web", "Browser create/restore/review/sign application without blockchain watcher"],
                    ["KasKold Vault Android / iOS", "Dedicated native create/restore/review/sign applications"],
                    ["KasKold Companion Web / Android / iOS", "Watch-only kpub/network/transaction/broadcast applications"],
                ],
                [0.38 * usable, 0.62 * usable],
            )
        ]),
        *section(2, "Wallet lifecycle and recovery", [
            bullet("Create or restore supported wallet sources only in Hardware/Vault."),
            bullet("Use mnemonic + optional passphrase as the durable cross-device recovery path."),
            bullet("Export the public kpub to Companion; never import a mnemonic/private key into Companion."),
        ]),
        *section(3, "Companion workflow", [
            bullet("Start with kpub management; Broadcast TX and Multisig remain directly available."),
            bullet("Use Companion for online observation, transaction construction, signing-request transport, finalization, and broadcast."),
            bullet("There is no Companion hot-wallet or local-signing mode."),
        ]),
        *section(4, "Hardware and Vault signing", [
            bullet("Scan/import the complete signing request and independently verify recipient, amount, fee, change, network and advanced context."),
            bullet("Approve only on the signer. Return the signed response to Companion."),
            bullet("Vault Web/Android/iOS and Hardware share the hardened Rust signing/protocol implementation rather than reimplementing cryptography in UI code."),
        ]),
        *section(5, "Hardware and qualification", [
            data_table(
                ["Target", "Support"],
                [
                    ["M5Stack CoreS3", "Hardware-qualified"],
                    ["M5Stack CoreS3 Lite", "Shared CoreS3 adapter/profile; build-supported; qualification pending"],
                ],
                [0.35 * usable, 0.65 * usable],
            )
        ]),
        *section(6, "Security habits", [
            bullet("Keep multiple tested recovery backups and start with small amounts after workflow changes."),
            bullet("Use a trusted/private node when address-query privacy matters."),
            bullet("Treat CoreS3 Lite as qualification-pending until physical HIL evidence exists."),
        ]),
    ]
    build(
        "KasKold_User_Guide.pdf",
        "KasKold User Guide",
        "Hardware, Vault, watch-only Companion, backup/recovery, signing, and maintenance",
        story,
    )

def main() -> int:
    GUIDES.mkdir(parents=True, exist_ok=True)
    for generator in (companion_guide, quick_start, security_architecture, user_guide):
        generator()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
