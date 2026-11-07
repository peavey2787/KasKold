// Storage screen redraw façade.
use crate::{hw::display::BootDisplay, hw::sdcard::SdCardType, runtime::data::AppData};

mod lists;
mod prompts;
mod settings;

pub(super) fn redraw(ad: &mut AppData, display: &mut BootDisplay<'_>) -> bool {
    lists::redraw(ad, display) || prompts::redraw(ad, display) || settings::redraw(ad, display)
}
