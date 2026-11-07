// QR presentation routing.
use super::AppData;
use crate::controllers::TouchInput;

mod presentation;

pub(super) fn handle(ad: &mut AppData, input: TouchInput) -> Option<bool> {
    presentation::handle(ad, input)
}
