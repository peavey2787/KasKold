// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0
//! Application navigation state and stable menu labels.
pub use kaskold_hardware_core::screen_flow::{is_scan_state, AppState};
/// Main menu items sourced from the authoritative production UI graph.
pub const MAIN_MENU_ITEMS: &[&str] = &crate::runtime::navigation::ui_graph::MAIN_MENU_LABELS;
/// Confirm menu (used in TX review confirm page), also graph-owned.
pub const CONFIRM_MENU_ITEMS: &[&str] = &crate::runtime::navigation::ui_graph::CONFIRM_TX_LABELS;
