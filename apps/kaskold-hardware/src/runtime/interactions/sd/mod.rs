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

use crate::{
    hw::display,
    runtime::data::AppData,
    services::{audio as sound, backup as sd_backup, storage_device as sdcard},
};
use offline_signer::derivation::hmac::zeroize_buf;

mod backup;
mod common;
mod exports;
pub(crate) mod imports;

pub use common::context::SdTouchContext;
use common::{
    context::SdIoContext,
    encryption_prompt::{run_encryption_prompt, EncryptionPayload, EncryptionPromptWorkflow, PromptDestination},
    filename::{run_filename_workflow, FilenameWorkflow},
    import_scan::{scan_by_rule, ImportScanRule},
    run_sd_file_list_context, run_sd_list_context, shared, FileListWorkflow,
};

pub(crate) use shared::{
    build_filename_83, format_auto_name, generate_trng_nonce, scan_auto_increment, write_file_to_sd,
};

use shared::{parse_descriptor, sd_file_exists};

pub use common::routing::handle_sd_touch;

#[cfg(all(feature = "workflow-test-auto", not(feature = "workflow-hil-auto")))]
pub(crate) use backup::seed::workflow_prepare_seed_backup_filename;
#[cfg(feature = "workflow-test-auto")]
pub(crate) use backup::workflow_import_backup_payload;
#[cfg(feature = "workflow-test-auto")]
pub(crate) use exports::kpub::workflow_import_text_payload;
#[cfg(feature = "workflow-test-auto")]
pub(crate) use exports::kspt_export::workflow_seal_envelope;
#[cfg(feature = "workflow-test-auto")]
pub(crate) use imports::{workflow_import_payload, workflow_import_transaction_payload};
