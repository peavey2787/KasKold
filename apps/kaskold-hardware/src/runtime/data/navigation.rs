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

// runtime/data/navigation.rs — NavigationState

mod history;
mod production;
use history::NavigationHistory;
pub use production::ProductionNavigationState;

pub struct NavigationState {
    pub(crate) owner: crate::runtime::navigation::NavigationOwner,
    pub(crate) committed_state: crate::runtime::input::AppState,
    pub(crate) app: crate::runtime::input::WalletApp,
    pub(crate) history: NavigationHistory,
    pub multisig_menu: crate::runtime::input::Menu,
    pub multisig_descriptors_menu: crate::runtime::input::Menu,
    pub xprv_export_menu: crate::runtime::input::Menu,
    pub settings_menu: crate::runtime::input::Menu,
    pub production: ProductionNavigationState,
    #[cfg(feature = "workflow-tests")]
    pub(crate) workflow_tests_menu: crate::runtime::input::Menu,
    #[cfg(feature = "workflow-tests")]
    pub(crate) workflow_category_menu: crate::runtime::input::Menu,
    #[cfg(feature = "workflow-tests")]
    pub(crate) workflow_tests_return: crate::runtime::navigation::ContinuationRoute,
    pub sd_import_menu: crate::runtime::input::Menu,
}

impl NavigationState {
    pub(super) fn new() -> Self {
        debug_assert!(crate::runtime::navigation::ui_graph::validate_static_graph());
        Self {
            owner: crate::runtime::navigation::NavigationOwner::Main,
            committed_state: crate::runtime::input::AppState::MainMenu,
            app: crate::runtime::input::WalletApp::new(),
            history: NavigationHistory::new(),
            multisig_menu: crate::runtime::input::Menu::from_items(
                crate::runtime::navigation::ui_graph::MULTISIG_LABELS,
            ),
            multisig_descriptors_menu: crate::runtime::input::Menu::from_items(
                crate::runtime::navigation::ui_graph::MULTISIG_DESCRIPTOR_LABELS,
            ),
            xprv_export_menu: crate::runtime::input::Menu::from_items(
                crate::runtime::navigation::ui_graph::XPRV_EXPORT_LABELS,
            ),
            settings_menu: crate::runtime::input::Menu::from_items(
                crate::runtime::navigation::production::SETTINGS_ITEMS,
            ),
            production: ProductionNavigationState::new(),
            sd_import_menu: crate::runtime::input::Menu::from_items(
                crate::runtime::navigation::ui_graph::SD_IMPORT_LABELS,
            ),
            #[cfg(feature = "workflow-tests")]
            workflow_tests_menu: crate::runtime::input::Menu::from_items(
                &crate::runtime::workflow_tests::category_labels(),
            ),
            #[cfg(feature = "workflow-tests")]
            workflow_category_menu: crate::runtime::input::Menu::new(),
            #[cfg(feature = "workflow-tests")]
            workflow_tests_return: crate::runtime::navigation::continuation!(DeveloperMenu),
        }
    }
}
