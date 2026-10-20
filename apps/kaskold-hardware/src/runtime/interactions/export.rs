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

// controllers/export.rs — Stable export-controller façade.

mod address;
mod context;
mod derivation;
pub(crate) use self::derivation::derive_watch_account;
mod index_keypad;
mod kpub;
pub(crate) mod menus;
mod private_key;
pub(crate) mod seed_backup;
mod seed_qr;
mod watch_only;
pub(crate) use watch_only::prepare_multisig_kpub_qr;
mod xprv;

use crate::runtime::interactions::feedback::RedrawFlag;
use crate::runtime::interactions::TouchInput;

pub use context::ExportTouchContext;

/// Handle touch events for export/display screens.
#[inline(never)]
pub fn handle_export_touch(mut context: ExportTouchContext<'_, '_, '_>) -> Option<bool> {
    let TouchInput { x, y, is_back } = context.input;
    if let Some(result) = seed_backup::handle(context.ad, is_back) {
        return Some(result);
    }
    if let Some(result) = address::handle(context.ad, context.boot_display, x, y, is_back) {
        return Some(result);
    }
    if let Some(result) = seed_qr::handle(context.ad, x, y, is_back) {
        return Some(result);
    }
    if let Some(result) = kpub::handle(context.ad, context.delay, context.i2c, x, is_back) {
        return Some(result);
    }
    if let Some(result) = menus::root::handle(context.menu()) {
        return Some(result);
    }
    if let Some(result) = menus::seed::handle(context.menu()) {
        return Some(result);
    }
    if let Some(result) = watch_only::handle(context.storage()) {
        return Some(result);
    }
    if let Some(result) = menus::signing_keys::handle(context.ad, context.list_zones, x, y, is_back) {
        return Some(result);
    }
    if let Some(result) = xprv::handle(context.storage()) {
        return Some(result);
    }
    let ExportTouchContext { ad, boot_display, delay, liveness, .. } = context;
    private_key::handle(ad, boot_display, delay, liveness, x, y, is_back)
}
