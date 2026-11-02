use std::{vec, vec::Vec};

use super::NavigationOwner::*;
use super::*;

const EVERY_OWNER: [NavigationOwner; 9] = [
    Onboarding, Main, Seeds, Settings, Signing, Export, Storage, Stego, Multisig,
];

/// Every owner that may hand over to `to`.
fn owners_reaching(to: NavigationOwner) -> Vec<NavigationOwner> {
    EVERY_OWNER
        .into_iter()
        .filter(|from| owner_transition_allowed(*from, to))
        .collect()
}

#[test]
fn a_workflow_opens_only_from_the_owners_that_lead_to_it() {
    for (to, expected) in [
        (Onboarding, vec![Onboarding, Seeds]),
        (
            Main,
            vec![
                Onboarding, Seeds, Settings, Signing, Export, Storage, Stego, Multisig,
            ],
        ),
        (
            Seeds,
            vec![Main, Settings, Signing, Export, Storage, Stego, Multisig],
        ),
        (Settings, vec![Main, Seeds]),
        (
            Signing,
            vec![Main, Seeds, Settings, Export, Storage, Multisig],
        ),
        (
            Export,
            vec![Main, Seeds, Settings, Signing, Storage, Stego, Multisig],
        ),
        (
            Storage,
            vec![Seeds, Settings, Signing, Export, Stego, Multisig],
        ),
        (Stego, vec![Settings, Storage]),
        (Multisig, vec![Seeds, Settings, Signing, Export, Storage]),
    ] {
        assert_eq!(owners_reaching(to), expected, "{to:?}");
    }
}

#[cfg(feature = "workflow-tests")]
#[test]
fn workflow_tests_join_settings_in_both_directions_only() {
    assert_eq!(owners_reaching(WorkflowTests), vec![Settings]);
    assert!(owner_transition_allowed(WorkflowTests, Settings));
    assert!(!owner_transition_allowed(WorkflowTests, Main));
}

#[test]
fn a_handler_group_names_the_workflow_that_owns_its_screens() {
    for (group, owner) in [
        (HandlerGroup::Settings, Settings),
        (HandlerGroup::Persistence, Storage),
        (HandlerGroup::Seed, Seeds),
        (HandlerGroup::Export, Export),
        (HandlerGroup::Sd, Storage),
        (HandlerGroup::Stego, Stego),
        (HandlerGroup::Tx, Signing),
    ] {
        for current in EVERY_OWNER {
            assert_eq!(owner_from_handler(group, current), owner, "{group:?}");
        }
    }
}

#[test]
fn a_menu_screen_stays_with_its_workflow_unless_that_workflow_has_no_menus() {
    for current in [Onboarding, Main, Seeds, Settings, Signing, Export, Multisig] {
        assert_eq!(owner_from_handler(HandlerGroup::Menu, current), current);
    }
    for current in [Storage, Stego] {
        assert_eq!(owner_from_handler(HandlerGroup::Menu, current), Main);
    }
}

#[test]
fn a_qr_screen_stays_with_the_workflow_that_produced_its_payload() {
    for current in [Signing, Export, Storage, Multisig] {
        let owner = owner_for_intent(DeviceStorageIntent::StartFresh, AppState::ShowQR, current);
        assert_eq!(owner, current);
    }
    for current in [Onboarding, Main, Seeds, Settings, Stego] {
        let owner = owner_for_intent(DeviceStorageIntent::None, AppState::ShowQR, current);
        assert_eq!(owner, Signing);
    }
}
