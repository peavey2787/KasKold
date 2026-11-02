use super::super::{
    draw_lato_title, draw_oswald_header, measure_header, size24px, BootDisplay, Circle, CornerRadii, Drawable, Image,
    Line, Point, Primitive, PrimitiveStyle, Rectangle, Rgb565, RoundedRectangle, Size, Triangle, COLOR_CARD,
    COLOR_CARD_BORDER, COLOR_TEXT, KASPA_ACCENT, KASPA_TEAL,
};
use embedded_iconoir::prelude::IconoirNewIcon;

const NAME_HEAD: usize = 6;
const NAME_TAIL: usize = 8;

/// A name too long for its row keeps both ends: photos from one camera differ at the end.
fn shorten_name<'text>(name: &'text str, shortened: &'text mut [u8; NAME_HEAD + 2 + NAME_TAIL]) -> &'text str {
    let bytes = name.as_bytes();
    if bytes.len() <= shortened.len() {
        return name;
    }
    shortened[..NAME_HEAD].copy_from_slice(&bytes[..NAME_HEAD]);
    shortened[NAME_HEAD..NAME_HEAD + 2].copy_from_slice(b"..");
    shortened[NAME_HEAD + 2..].copy_from_slice(&bytes[bytes.len() - NAME_TAIL..]);
    core::str::from_utf8(shortened).unwrap_or("?")
}

impl<'a> BootDisplay<'a> {
    pub(crate) fn draw_stego_file_picker(
        &mut self,
        header: &str,
        display_names: &[[u8; 32]; 8],
        display_lens: &[u8; 8],
        count: u8,
        selected: Option<u8>,
        jpeg: bool,
    ) {
        self.clear_keep_nav();
        let title_width = measure_header(header);
        draw_oswald_header(&mut self.display, header, (320 - title_width) / 2, 30, COLOR_TEXT);
        Line::new(Point::new(20, 40), Point::new(300, 40))
            .into_styled(PrimitiveStyle::with_stroke(KASPA_TEAL, 1))
            .draw(&mut self.display)
            .ok();

        let page_size = 4u8;
        let scroll = selected.map_or(0, |index| (index / page_size) * page_size);
        let dark_teal = Rgb565::new(0b00001, 0b000100, 0b00010);
        for visible in 0..page_size {
            let index = scroll + visible;
            let y = 46 + i32::from(visible) * 46;
            let rect = Rectangle::new(Point::new(44, y), Size::new(232, 42));
            let corner = CornerRadii::new(Size::new(6, 6));
            let is_selected = selected == Some(index);
            let fill = if is_selected { COLOR_CARD_BORDER } else { COLOR_CARD };
            let stroke = if is_selected { KASPA_TEAL } else { COLOR_CARD_BORDER };
            RoundedRectangle::new(rect, corner)
                .into_styled(PrimitiveStyle::with_fill(fill))
                .draw(&mut self.display)
                .ok();
            RoundedRectangle::new(rect, corner)
                .into_styled(PrimitiveStyle::with_stroke(stroke, 1))
                .draw(&mut self.display)
                .ok();
            if index >= count {
                continue;
            }

            let icon_color = if is_selected { KASPA_TEAL } else { COLOR_TEXT };
            if jpeg {
                let icon = size24px::photos_and_videos::MediaImage::new(icon_color);
                Image::new(&icon, Point::new(50, y + 9)).draw(&mut self.display).ok();
            } else {
                let icon = size24px::docs::Page::new(icon_color);
                Image::new(&icon, Point::new(50, y + 9)).draw(&mut self.display).ok();
            }
            let length = usize::from(display_lens[index as usize]);
            let name = core::str::from_utf8(&display_names[index as usize][..length]).unwrap_or("?");
            let mut shortened = [0u8; NAME_HEAD + 2 + NAME_TAIL];
            draw_lato_title(&mut self.display, shorten_name(name, &mut shortened), 80, y + 28, icon_color);
        }

        let arrow_y = 136;
        let can_up = scroll > 0;
        let can_down = scroll + page_size < count;
        for (points, enabled) in [
            ((Point::new(5, arrow_y), Point::new(30, arrow_y - 17), Point::new(30, arrow_y + 17)), can_up),
            ((Point::new(315, arrow_y), Point::new(290, arrow_y - 17), Point::new(290, arrow_y + 17)), can_down),
        ] {
            Triangle::new(points.0, points.1, points.2)
                .into_styled(PrimitiveStyle::with_fill(if enabled { KASPA_TEAL } else { dark_teal }))
                .draw(&mut self.display)
                .ok();
        }

        if jpeg && count > page_size {
            let page_count = count.div_ceil(page_size);
            let current = scroll / page_size;
            let total_width = i32::from(page_count) * 7 + (i32::from(page_count) - 1) * 8;
            for page in 0..page_count {
                let x = (320 - total_width) / 2 + i32::from(page) * 15;
                Circle::new(Point::new(x, 232), 7)
                    .into_styled(PrimitiveStyle::with_fill(if page == current { KASPA_ACCENT } else { dark_teal }))
                    .draw(&mut self.display)
                    .ok();
            }
        }
    }
}
