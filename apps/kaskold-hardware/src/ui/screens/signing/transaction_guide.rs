// KasKold — Air-gapped offline signing device for Kaspa
// License: GPL-3.0-or-later.

use super::super::{
    draw_lato_18, draw_lato_body, draw_lato_title, draw_oswald_header, measure_18, measure_body, measure_header,
    measure_title, BootDisplay, CornerRadii, Drawable, Line, Point, Primitive, PrimitiveStyle, Rectangle,
    RoundedRectangle, Size, COLOR_BG, COLOR_TEXT, COLOR_TEXT_DIM, KASPA_TEAL,
};

impl<'a> BootDisplay<'a> {
    /// Explain the second Companion -> KasKold anti-klepto handoff before
    /// reopening the camera. This prevents the reveal scan from looking like
    /// the signing workflow unexpectedly restarted.
    pub fn draw_anti_klepto_reveal_guide(&mut self) {
        self.clear_keep_nav();

        let title = "SCAN COMPANION QR";
        let title_width = measure_header(title);
        draw_oswald_header(&mut self.display, title, (320 - title_width) / 2, 30, KASPA_TEAL);
        Line::new(Point::new(20, 40), Point::new(300, 40))
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display)
            .ok();

        let lines = ["Companion should now show", "the next signing QR.", "Press OK, then scan it", "with KasKold."];
        for (index, line) in lines.iter().enumerate() {
            let width = measure_18(line);
            draw_lato_18(&mut self.display, line, (320 - width) / 2, 70 + index as i32 * 24, COLOR_TEXT);
        }

        let hint = "Signing session stays active";
        let hint_width = measure_body(hint);
        draw_lato_body(&mut self.display, hint, (320 - hint_width) / 2, 165, COLOR_TEXT_DIM);

        let zone = crate::ui::layout::ERROR_OK_ZONE;
        let rectangle = Rectangle::new(
            Point::new(i32::from(zone.x), i32::from(zone.y)),
            Size::new(u32::from(zone.w), u32::from(zone.h)),
        );
        RoundedRectangle::new(rectangle, CornerRadii::new(Size::new(6, 6)))
            .into_styled(PrimitiveStyle::with_fill(KASPA_TEAL))
            .draw(&mut self.display)
            .ok();
        let ok_width = measure_title("OK");
        draw_lato_title(
            &mut self.display,
            "OK",
            i32::from(zone.x) + (i32::from(zone.w) - ok_width) / 2,
            i32::from(zone.y) + 28,
            COLOR_BG,
        );
    }
}
