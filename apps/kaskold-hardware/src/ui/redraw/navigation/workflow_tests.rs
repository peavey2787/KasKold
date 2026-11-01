//! Workflow-test screens kept out of the production root.

use crate::{
    hw::display,
    runtime::{data::AppData, input::AppState},
};

pub(super) fn redraw(ad: &mut AppData, boot_display: &mut display::BootDisplay<'_>) -> bool {
    match ad.navigation.app.state {
        AppState::WorkflowTestsMenu => {
            boot_display.update_menu_content("E2E TESTS", &ad.navigation.workflow_tests_menu)
        }
        AppState::WorkflowTestsCategory { category } => redraw_workflow_category(ad, boot_display, category),
        AppState::WorkflowTestsResult => redraw_workflow_result(ad, boot_display),
        _ => return false,
    }
    true
}

fn redraw_workflow_category(ad: &AppData, boot_display: &mut display::BootDisplay<'_>, category: u8) {
    let title = crate::runtime::workflow_tests::WorkflowCategory::from_raw(category)
        .map(crate::runtime::workflow_tests::WorkflowCategory::label)
        .unwrap_or("E2E CATEGORY");
    boot_display.update_menu_content(title, &ad.navigation.workflow_category_menu);
}

fn redraw_workflow_result(ad: &AppData, boot_display: &mut display::BootDisplay<'_>) {
    let result = ad.workflow_tests.result;
    let mut message: heapless::String<48> = heapless::String::new();
    let _ = core::fmt::Write::write_fmt(&mut message, format_args!("Contracts {}/{}", result.passed, result.total));
    if result.failed == 0 {
        boot_display.draw_success_screen(message.as_str());
    } else {
        boot_display.draw_error_back_screen(message.as_str());
    }
}
