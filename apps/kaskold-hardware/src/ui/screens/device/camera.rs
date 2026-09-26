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


use super::super::{
    BootDisplay,
    COLOR_BG,
    COLOR_TEXT,
    DrawTarget,
    Drawable,
    KASPA_TEAL,
    Line,
    Point,
    Primitive,
    PrimitiveStyle,
    Rectangle,
    Rgb565,
    Size,
    draw_oswald_header,
    measure_header,
};

fn clear_camera_surface(display: &mut impl DrawTarget<Color = Rgb565>) {
    Rectangle::new(Point::new(0, 0), Size::new(320, 240))
        .into_styled(PrimitiveStyle::with_fill(COLOR_BG))
        .draw(display)
        .ok();
}

fn draw_back_chrome(display: &mut impl DrawTarget<Color = Rgb565>) {
    use embedded_graphics::image::{Image, ImageRawLE};
    let back: ImageRawLE<Rgb565> = ImageRawLE::new(
        crate::ui::display::icon_data::ICON_BACK,
        crate::ui::display::icon_data::ICON_BACK_W,
    );
    Image::new(&back, Point::new(0, 0)).draw(display).ok();
}

impl<'a> BootDisplay<'a> {
    /// Draw camera / QR scanner screen
    /// Shows status info and a viewfinder-style frame
    pub fn draw_camera_screen(&mut self) {
        self.draw_camera_screen_title("SCAN QR");
    }

    /// Guided anti-klepto reveal scanner. The protocol session is already fixed,
    /// so the title tells the user exactly which QR belongs here.
    pub fn draw_anti_klepto_reveal_camera_screen(&mut self) {
        self.draw_camera_screen_title("SCAN COMPANION QR");
    }

    fn draw_camera_screen_title(&mut self, title: &str) {
        clear_camera_surface(&mut self.display);
        draw_back_chrome(&mut self.display);
        let tw = measure_header(title);
        draw_oswald_header(&mut self.display, title, (320 - tw) / 2, 30, COLOR_TEXT);
        Line::new(Point::new(20, 40), Point::new(300, 40))
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display).ok();
    }

    /// Blit a grayscale camera frame into the viewfinder area.
    /// Renders at (40, 30) with 240x180 pixels, leaving top 30px for back button.
    /// Redraws back button after blit so it's always visible during streaming.
    pub fn blit_camera_frame(&mut self, frame: &[u8], width: usize, height: usize,
                             qr_guide_info: u8) {
        use embedded_graphics::primitives::Rectangle;
        use embedded_graphics::prelude::*;
        use embedded_graphics::pixelcolor::Rgb565;
        use embedded_graphics::draw_target::DrawTarget;

        // CoreS3 display area: centered below the 42px header chrome.
        let (vf_x, vf_y, vf_w, vf_h) = (40i32, 44i32, 240usize, 180usize);

        if width == 0 || height == 0 { return; }

        let dw = vf_w;
        let dh = vf_h;

        // Decode guide info: bit 7 = finders active
        let finders_active = (qr_guide_info & 0x80) != 0;

        // Frame border: 2px thick around entire viewfinder
        // Red/orange when idle, flashing green when finders detected
        let border_w = 2i32;
        let border_color = if finders_active {
            // Flash between bright and dim green using frame data parity
            let flash = (frame[0] as u16 + frame[width/2] as u16) & 1;
            if flash == 0 {
                Rgb565::new(0, 63, 0)
            } else {
                Rgb565::new(0, 42, 0)
            }
        } else {
            Rgb565::new(20, 8, 0) // dim red/amber — "scanning"
        };

        for vy in 0..dh {
            let src_y = if height > vf_h {
                vy * height / vf_h
            } else {
                vy * height / dh
            };
            if src_y >= height { break; }

            let area = Rectangle::new(
                Point::new(vf_x, vf_y + vy as i32),
                Size::new(dw as u32, 1),
            );

            let abs_y = vf_y + vy as i32;
            let on_top_border = abs_y < vf_y + border_w;
            let on_bot_border = abs_y >= vf_y + vf_h as i32 - border_w;

            let row_start = src_y * width;
            let _ = self.display.fill_contiguous(
                &area,
                (0..dw).map(move |vx| {
                    let abs_x = vf_x + vx as i32;
                    let on_left = abs_x < vf_x + border_w;
                    let on_right = abs_x >= vf_x + vf_w as i32 - border_w;
                    if on_top_border || on_bot_border || on_left || on_right {
                        return border_color;
                    }

                    let sx = if width >= vf_w {
                        (vx * width / vf_w).min(width - 1)
                    } else {
                        (vx * width / dw).min(width - 1)
                    };
                    let gray = frame[row_start + sx];
                    Rgb565::new(gray >> 3, gray >> 2, gray >> 3)
                }),
            );
        }

        // Icons persist outside blit rectangle — no per-frame redraw needed.
    }

}
