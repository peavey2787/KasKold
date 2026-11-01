use crate::hw::touch;
use crate::runtime::input::Menu;
use crate::runtime::interactions::menu_selection::selected_visible_item;

pub(super) fn selected_item(menu: &Menu, list_zones: &[touch::TouchZone; 4], x: u16, y: u16) -> Option<u8> {
    selected_visible_item(menu, list_zones, x, y)
}
