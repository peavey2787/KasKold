// KasKold screen façade.

use super::super::{
    draw_lato_18, draw_lato_body, draw_lato_hint, draw_lato_title, draw_oswald_header, measure_18, measure_body,
    measure_header, measure_hint, measure_title, truncate_chars, BootDisplay, CornerRadii, DrawTarget, Drawable, Line,
    Point, Primitive, PrimitiveStyle, Rectangle, Rgb565, RoundedRectangle, Size, COLOR_BG, COLOR_CARD,
    COLOR_CARD_BORDER, COLOR_HINT, COLOR_ORANGE, COLOR_TEXT, COLOR_TEXT_DIM, KASPA_ACCENT, KASPA_TEAL,
};

mod jpeg;
mod prompts;
mod text;
