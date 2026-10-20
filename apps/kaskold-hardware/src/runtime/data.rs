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

// runtime/data.rs — Focused firmware state aggregate
//
// AppData is the stable handler-facing root. Each subsystem owns mutable state
// in a dedicated struct, while hardware peripherals remain outside AppData
// because their lifetimes are tied to `main`.

mod export;
mod navigation;
mod placement;
#[cfg(feature = "provisioning-ui")]
mod pop_it;
mod presentation;
mod qr;
mod runtime;
mod settings;
mod signing;
mod stego;
mod storage;
mod wallet;
#[cfg(feature = "workflow-tests")]
mod workflow_tests;

pub use self::export::ExportState;
pub use self::navigation::NavigationState;
#[cfg(feature = "provisioning-ui")]
pub use self::pop_it::PopItState;
#[cfg(not(feature = "hardware-tests"))]
pub(crate) use self::presentation::OperationExecution;
pub use self::presentation::{ModalState, OperationKind, OperationPhase, PresentationState};
#[cfg(all(
    not(feature = "hardware-tests"),
    any(not(feature = "workflow-test-auto"), all(feature = "m5stack", feature = "workflow-runtime-auto"))
))]
pub use self::qr::CameraScanFault;
pub use self::qr::{OutgoingQrPurpose, QrState};
pub use self::runtime::{DestructiveHoldState, RuntimeState};
pub use self::settings::{ScreenDimTimeout, SettingsState};
pub use self::signing::{
    AntiKleptoPhase, CovenantSigningMode, CovenantSigningPhase, OutputOwnership, PrivateSwapMode, PrivateSwapPhase,
    SigningState,
};
pub use self::stego::StegoState;
#[cfg(feature = "m5stack")]
pub use self::storage::RtcVerification;
pub use self::storage::{
    AdvancedAvailability, ConfirmationState, DeviceStorageIntent, DuressActivation, EncryptedFileOperation,
    EncryptedPayloadKind, PendingStorageAction, PersistenceBackendState, PolicyIntegrity, StorageState, TextFileKind,
    TextFileList, UnlockFeedback,
};
pub use self::wallet::WalletState;
#[cfg(feature = "workflow-tests")]
pub(crate) use self::workflow_tests::WorkflowTestState;

/// Stable root state passed to firmware controllers and renderers.
pub struct AppData {
    pub runtime: RuntimeState,
    pub presentation: PresentationState,
    pub navigation: NavigationState,
    pub wallet: WalletState,
    pub export: ExportState,
    pub storage: StorageState,
    pub qr: QrState,
    pub signing: SigningState,
    pub stego: StegoState,
    pub settings: SettingsState,
    #[cfg(feature = "provisioning-ui")]
    pub pop_it: PopItState,
    #[cfg(feature = "workflow-tests")]
    pub(crate) workflow_tests: WorkflowTestState,
}
