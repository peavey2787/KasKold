// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! What the slot held at the last probe, its lock state, and an unlocked session.

use super::SdCardType;
use core::sync::atomic::{AtomicU8, Ordering};

const CARD_LOCK_UNKNOWN: u8 = 0;
const CARD_LOCK_UNLOCKED: u8 = 1;
const CARD_LOCK_LOCKED: u8 = 2;
static CARD_LOCK_STATUS: AtomicU8 = AtomicU8::new(CARD_LOCK_UNKNOWN);
const NO_CARD: u8 = 0;
const CARD_SDV1: u8 = 1;
const CARD_SDV2_SC: u8 = 2;
const CARD_SDV2_HC: u8 = 3;
static MANUAL_UNLOCK_SESSION: AtomicU8 = AtomicU8::new(NO_CARD);
static KNOWN_CARD: AtomicU8 = AtomicU8::new(NO_CARD);

fn encode_card(card: Option<SdCardType>) -> u8 {
    match card {
        None => NO_CARD,
        Some(SdCardType::SdV1) => CARD_SDV1,
        Some(SdCardType::SdV2Sc) => CARD_SDV2_SC,
        Some(SdCardType::SdV2Hc) => CARD_SDV2_HC,
    }
}

fn decode_card(value: u8) -> Option<SdCardType> {
    match value {
        CARD_SDV1 => Some(SdCardType::SdV1),
        CARD_SDV2_SC => Some(SdCardType::SdV2Sc),
        CARD_SDV2_HC => Some(SdCardType::SdV2Hc),
        _ => None,
    }
}

pub(crate) fn record_card_lock_status(locked: bool) {
    let value = if locked { CARD_LOCK_LOCKED } else { CARD_LOCK_UNLOCKED };
    CARD_LOCK_STATUS.store(value, Ordering::Relaxed);
}

pub(crate) fn card_is_known_locked() -> bool {
    CARD_LOCK_STATUS.load(Ordering::Relaxed) == CARD_LOCK_LOCKED
}

/// Remember what the slot held the last time it was probed, so screens can say
/// so without touching the bus.
pub(crate) fn record_card(card: Option<SdCardType>) {
    KNOWN_CARD.store(encode_card(card), Ordering::Relaxed);
}

/// The card found by the most recent probe or card operation.
pub(crate) fn known_card() -> Option<SdCardType> {
    decode_card(KNOWN_CARD.load(Ordering::Relaxed))
}

pub(crate) fn record_manual_unlock_session(card_type: SdCardType) {
    MANUAL_UNLOCK_SESSION.store(encode_card(Some(card_type)), Ordering::Relaxed);
    record_card_lock_status(false);
}

pub(crate) fn manual_unlock_session_card_type() -> Option<SdCardType> {
    decode_card(MANUAL_UNLOCK_SESSION.load(Ordering::Relaxed))
}

pub(crate) fn clear_manual_unlock_session() {
    MANUAL_UNLOCK_SESSION.store(NO_CARD, Ordering::Relaxed);
}
