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

//! CoreS3 microSD storage facade.
//!
//! GPIO36/37 carry the shared SPI2 clock/MOSI signals. GPIO35 is SD MISO
//! whenever the LCD is deselected and LCD D/C only while LCD CS is asserted.
//! `hw::m5stack::spi_bus` owns SPI2 for the entire firmware lifetime; this
//! storage layer is only an SD protocol client and never reclaims or rewrites
//! the display peripheral.

use esp_hal::delay::Delay;

mod card_state;
pub(crate) use card_state::{
    card_is_known_locked, clear_manual_unlock_session, known_card, manual_unlock_session_card_type, record_card,
    record_card_lock_status, record_manual_unlock_session,
};

mod transport;
pub(crate) use transport::detect_card;
pub use transport::sd_read_block;
pub(crate) use transport::with_sd_card as with_sd_card_impl;
#[cfg(all(feature = "workflow-hil-auto", feature = "m5stack"))]
pub(crate) use transport::workflow_force_erase_locked_card;
pub(crate) use transport::{fast_read_multi_block, fast_write_multi_block, sd_sector_count, sd_write_block};
pub(crate) use transport::{force_erase_locked_card_session, unlock_locked_card_session};

macro_rules! with_sd_card {
    ($i2c:expr, $delay:expr, $operation:expr) => {
        $crate::hw::sdcard::with_sd_card_impl($i2c, $delay, $operation)
    };
}
pub(crate) use with_sd_card;

pub use crate::hw::shared::storage::fat32::{
    create_file, delete_file, find_file_in_root, format_83_display, format_fat32, list_root_dir, list_root_dir_lfn,
    mount_fat32, overwrite_file, read_file, to_83_name, DirEntry, Fat32Info, SdCardType,
};
