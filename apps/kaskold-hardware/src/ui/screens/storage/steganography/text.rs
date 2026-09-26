// KasKold — Air-gapped offline signing device for Kaspa
// License: GPL-3.0-or-later.

use super::{
    BootDisplay,
    COLOR_BG,
    COLOR_CARD,
    COLOR_CARD_BORDER,
    COLOR_HINT,
    COLOR_TEXT,
    COLOR_TEXT_DIM,
    CornerRadii,
    Drawable,
    KASPA_ACCENT,
    KASPA_TEAL,
    Line,
    Point,
    Primitive,
    PrimitiveStyle,
    Rectangle,
    RoundedRectangle,
    Size,
    draw_lato_body,
    draw_lato_hint,
    draw_oswald_header,
    measure_body,
    measure_header,
    measure_hint,
};


fn char_window(value: &str, start_char: usize, max_chars: usize) -> &str {
    let start = value
        .char_indices()
        .nth(start_char)
        .map_or(value.len(), |(index, _)| index);
    let end = value[start..]
        .char_indices()
        .nth(max_chars)
        .map_or(value.len(), |(offset, _)| start + offset);
    &value[start..end]
}

impl<'a> BootDisplay<'a> {
/// Draw .TXT file picker with LFN display names — standard template layout
    pub fn draw_stego_txt_pick(&mut self, disp_names: &[[u8; 32]; 8], disp_lens: &[u8; 8], count: u8) {
        self.draw_stego_file_picker("SELECT TXT", disp_names, disp_lens, count, None, false);
    }

/// Draw descriptor preview and make its non-secret role explicit
    pub fn draw_stego_desc_preview(&mut self, desc: &str) {
        self.clear_keep_nav();
        let tw = measure_header("DESCRIPTOR PREVIEW");
        draw_oswald_header(&mut self.display, "DESCRIPTOR PREVIEW", (320 - tw) / 2, 25, KASPA_TEAL);
        Line::new(Point::new(20, 35), Point::new(300, 35))
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display).ok();

        // Show descriptor text (wrap at ~32 Unicode scalar values per line, max 3 lines).
        // The descriptor may originate from an external carrier, so never byte-slice a `str`.
        let character_count = desc.chars().count();
        let line1 = char_window(desc, 0, 32);
        let l1w = measure_body(line1);
        draw_lato_body(&mut self.display, line1, (320 - l1w) / 2, 55, KASPA_ACCENT);

        if character_count > 32 {
            let line2 = char_window(desc, 32, 32);
            let l2w = measure_body(line2);
            draw_lato_body(&mut self.display, line2, (320 - l2w) / 2, 73, KASPA_ACCENT);
        }
        if character_count > 64 {
            let line3 = char_window(desc, 64, 30);
            let l3w = measure_body(line3);
            draw_lato_body(&mut self.display, line3, (320 - l3w) / 2, 91, KASPA_ACCENT);
            if character_count > 94 {
                let dots_w = measure_body("..");
                draw_lato_body(
                    &mut self.display,
                    "..",
                    ((320 - l3w) / 2 + l3w - dots_w).max(0),
                    91,
                    KASPA_ACCENT,
                );
            }
        }

        // The descriptor is deliberately public carrier text, not a password.
        let vw = measure_hint("VISIBLE CARRIER TEXT - NOT A PASSWORD");
        draw_lato_hint(&mut self.display, "VISIBLE CARRIER TEXT - NOT A PASSWORD", (320 - vw) / 2, 118, COLOR_HINT);

        // Character count
        let mut len_buf: heapless::String<16> = heapless::String::new();
        core::fmt::Write::write_fmt(&mut len_buf, format_args!("{character_count} characters")).ok();
        let lw = measure_hint(len_buf.as_str());
        draw_lato_hint(&mut self.display, len_buf.as_str(), (320 - lw) / 2, 136, COLOR_HINT);

        // Hint text
        let hw = measure_hint("Descriptor mode also writes it to EXIF.");
        draw_lato_hint(&mut self.display, "Descriptor mode also writes it to EXIF.", (320 - hw) / 2, 150, COLOR_TEXT_DIM);

        // EDIT / USE buttons
        let btn_corner = CornerRadii::new(Size::new(6, 6));
        let edit_rect = Rectangle::new(Point::new(20, 185), Size::new(130, 40));
        RoundedRectangle::new(edit_rect, btn_corner)
            .into_styled(PrimitiveStyle::with_fill(COLOR_CARD))
            .draw(&mut self.display).ok();
        RoundedRectangle::new(edit_rect, btn_corner)
            .into_styled(PrimitiveStyle::with_stroke(COLOR_CARD_BORDER, 1))
            .draw(&mut self.display).ok();
        let ew = measure_body("EDIT");
        draw_lato_body(&mut self.display, "EDIT", 20 + (130 - ew) / 2, 211, COLOR_TEXT);

        let use_rect = Rectangle::new(Point::new(170, 185), Size::new(130, 40));
        RoundedRectangle::new(use_rect, btn_corner)
            .into_styled(PrimitiveStyle::with_fill(KASPA_TEAL))
            .draw(&mut self.display).ok();
        let uw = measure_body("USE");
        draw_lato_body(&mut self.display, "USE", 170 + (130 - uw) / 2, 211, COLOR_BG);

    }

}
