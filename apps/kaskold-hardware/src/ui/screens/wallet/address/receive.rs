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

use super::super::super::{
    draw_lato_hint, draw_lato_title, draw_lato_title_opaque, draw_oswald_header, measure_header, measure_hint,
    measure_title, BootDisplay, CornerRadii, Drawable, Line, Point, Primitive, PrimitiveStyle, Rectangle,
    RoundedRectangle, Size, COLOR_BG, COLOR_CARD, COLOR_DANGER, COLOR_TEXT, COLOR_TEXT_DIM, KASPA_TEAL,
};

impl<'a> BootDisplay<'a> {
    /// Draw a derived address with its chain and index controls.
    pub fn draw_address_screen(&mut self, address: &str, index: u16, is_change: bool) {
        self.clear_keep_nav();
        self.draw_indexed_address_title(index, is_change);
        self.draw_address_separator(KASPA_TEAL);
        self.draw_address_lines(address, 44, 205);

        let btn_corner = CornerRadii::new(Size::new(6, 6));

        // Chain + QR actions share one explicit row, alongside the tap-on-address gesture.

        let toggle_label = if is_change { "Change" } else { "Receive" };
        let chain = crate::ui::layout::ADDRESS_CHAIN_ZONE;
        let chain_rect = Rectangle::new(
            Point::new(i32::from(chain.x), i32::from(chain.y)),
            Size::new(u32::from(chain.w), u32::from(chain.h)),
        );
        let (fill, fg) = if is_change { (KASPA_TEAL, COLOR_BG) } else { (COLOR_CARD, KASPA_TEAL) };
        RoundedRectangle::new(chain_rect, btn_corner)
            .into_styled(PrimitiveStyle::with_fill(fill))
            .draw(&mut self.display)
            .ok();
        RoundedRectangle::new(chain_rect, btn_corner)
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display)
            .ok();
        let tw = measure_title(toggle_label);
        draw_lato_title(
            &mut self.display,
            toggle_label,
            i32::from(chain.x) + (i32::from(chain.w) - tw) / 2,
            i32::from(chain.y + chain.h) - 8,
            fg,
        );

        let qr_zone = crate::ui::layout::ADDRESS_QR_ZONE;
        let qr_rect = Rectangle::new(
            Point::new(i32::from(qr_zone.x), i32::from(qr_zone.y)),
            Size::new(u32::from(qr_zone.w), u32::from(qr_zone.h)),
        );
        RoundedRectangle::new(qr_rect, btn_corner)
            .into_styled(PrimitiveStyle::with_fill(COLOR_CARD))
            .draw(&mut self.display)
            .ok();
        RoundedRectangle::new(qr_rect, btn_corner)
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display)
            .ok();
        let qw = measure_title("QR");
        draw_lato_title(
            &mut self.display,
            "QR",
            i32::from(qr_zone.x) + (i32::from(qr_zone.w) - qw) / 2,
            i32::from(qr_zone.y + qr_zone.h) - 8,
            KASPA_TEAL,
        );

        // Bottom nav: [<] [#N] [>] — original wide 3-button layout.
        // With the chain toggle moved to its own row above, these
        // reclaim the full width for easier tapping.
        let prev = crate::ui::layout::ADDRESS_PREV_ZONE;
        let btn_l = Rectangle::new(
            Point::new(i32::from(prev.x), i32::from(prev.y)),
            Size::new(u32::from(prev.w), u32::from(prev.h)),
        );
        RoundedRectangle::new(btn_l, btn_corner)
            .into_styled(PrimitiveStyle::with_fill(COLOR_CARD))
            .draw(&mut self.display)
            .ok();
        RoundedRectangle::new(btn_l, btn_corner)
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display)
            .ok();
        let lw = measure_title("<");
        draw_lato_title(
            &mut self.display,
            "<",
            i32::from(prev.x) + (i32::from(prev.w) - lw) / 2,
            i32::from(prev.y + prev.h) - 8,
            KASPA_TEAL,
        );

        self.draw_address_index_button(index);

        // [>] button
        let next = crate::ui::layout::ADDRESS_NEXT_ZONE;
        let btn_r = Rectangle::new(
            Point::new(i32::from(next.x), i32::from(next.y)),
            Size::new(u32::from(next.w), u32::from(next.h)),
        );
        RoundedRectangle::new(btn_r, btn_corner)
            .into_styled(PrimitiveStyle::with_fill(COLOR_CARD))
            .draw(&mut self.display)
            .ok();
        RoundedRectangle::new(btn_r, btn_corner)
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display)
            .ok();
        let rw = measure_title(">");
        draw_lato_title(
            &mut self.display,
            ">",
            i32::from(next.x) + (i32::from(next.w) - rw) / 2,
            i32::from(next.y + next.h) - 8,
            KASPA_TEAL,
        );
    }

    /// Draw one address with no index: a scanned one, or a hidden-key address.
    pub fn draw_titled_address_screen(&mut self, address: &str, title: &str, valid: bool) {
        self.clear_keep_nav();
        let title_color = if valid { COLOR_TEXT } else { COLOR_DANGER };
        let width = measure_header(title);
        draw_oswald_header(&mut self.display, title, (320 - width) / 2, 30, title_color);
        self.draw_address_separator(if valid { KASPA_TEAL } else { COLOR_DANGER });
        self.draw_address_lines(address, 44, 205);
        let hw = measure_hint("Tap for QR | < Back");
        draw_lato_hint(&mut self.display, "Tap for QR | < Back", (320 - hw) / 2, 232, COLOR_TEXT_DIM);
    }

    fn draw_address_separator(&mut self, color: embedded_graphics::pixelcolor::Rgb565) {
        Line::new(Point::new(20, 40), Point::new(300, 40))
            .into_styled(PrimitiveStyle::with_stroke(color, 1))
            .draw(&mut self.display)
            .ok();
    }

    /// Partial redraw: only title, address text, and #N label.
    /// Toggle button, <, > buttons, back/home icons stay static.
    pub fn update_address_content(&mut self, address: &str, addr_index: u16, is_change: bool) {
        // Clear title area (preserve nav icons)
        Rectangle::new(Point::new(34, 0), Size::new(252, 42))
            .into_styled(PrimitiveStyle::with_fill(COLOR_BG))
            .draw(&mut self.display)
            .ok();
        self.draw_back_button();

        self.draw_indexed_address_title(addr_index, is_change);

        let sep_color = KASPA_TEAL;
        Line::new(Point::new(20, 40), Point::new(300, 40))
            .into_styled(PrimitiveStyle::with_stroke(sep_color, 1))
            .draw(&mut self.display)
            .ok();

        Rectangle::new(Point::new(0, 44), Size::new(320, 131))
            .into_styled(PrimitiveStyle::with_fill(COLOR_BG))
            .draw(&mut self.display)
            .ok();
        self.draw_address_lines(address, 44, 175);

        self.draw_address_index_button(addr_index);
    }
    fn draw_indexed_address_title(&mut self, index: u16, is_change: bool) {
        let mut title: heapless::String<24> = heapless::String::new();
        let label = if is_change { "CHANGE" } else { "RECEIVE" };
        core::fmt::Write::write_fmt(&mut title, format_args!("{label} #{index}")).ok();
        let width = measure_header(title.as_str());
        draw_oswald_header(&mut self.display, &title, (320 - width) / 2, 30, COLOR_TEXT);
    }

    fn draw_address_lines(&mut self, address: &str, top: i32, bottom: i32) {
        const CHARS_PER_LINE: usize = 25;
        const LINE_HEIGHT: i32 = 26;
        let bytes = address.as_bytes();
        let line_count = bytes.len().div_ceil(CHARS_PER_LINE) as i32;
        let mut y = top + (bottom - top - line_count * LINE_HEIGHT) / 2;
        let mut offset = 0usize;
        while offset < bytes.len() && y < bottom {
            let end = core::cmp::min(offset + CHARS_PER_LINE, bytes.len());
            if let Ok(line) = core::str::from_utf8(&bytes[offset..end]) {
                let width = measure_title(line);
                draw_lato_title_opaque(&mut self.display, line, (320 - width) / 2, y, COLOR_TEXT, COLOR_BG);
            }
            y += LINE_HEIGHT;
            offset = end;
        }
    }

    fn draw_address_index_button(&mut self, index: u16) {
        let corner = CornerRadii::new(Size::new(6, 6));
        let zone = crate::ui::layout::ADDRESS_INDEX_ZONE;
        let rectangle = Rectangle::new(
            Point::new(i32::from(zone.x), i32::from(zone.y)),
            Size::new(u32::from(zone.w), u32::from(zone.h)),
        );
        RoundedRectangle::new(rectangle, corner)
            .into_styled(PrimitiveStyle::with_fill(COLOR_CARD))
            .draw(&mut self.display)
            .ok();
        RoundedRectangle::new(rectangle, corner)
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display)
            .ok();
        let mut label: heapless::String<8> = heapless::String::new();
        core::fmt::Write::write_fmt(&mut label, format_args!("#{index}")).ok();
        let width = measure_title(label.as_str());
        draw_lato_title(
            &mut self.display,
            &label,
            i32::from(zone.x) + (i32::from(zone.w) - width) / 2,
            i32::from(zone.y + zone.h) - 8,
            KASPA_TEAL,
        );
    }
}
