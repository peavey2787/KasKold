//! Reads the virtual panel: finds text by rendering it with the firmware's own fonts.
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{raw::RawU16, Rgb565, RgbColor},
    Pixel,
};
use esp_hal::board::lcd::{screens_held_for_reading, snapshot, HEIGHT, WIDTH};

use crate::ui::display as text;

type DrawText = fn(&mut MaskTarget, &str, i32, i32, Rgb565) -> i32;

const FONTS: [DrawText; 7] = [
    text::draw_lato_title::<MaskTarget>,
    text::draw_lato_body::<MaskTarget>,
    text::draw_lato_18::<MaskTarget>,
    text::draw_lato_22::<MaskTarget>,
    text::draw_lato_hint::<MaskTarget>,
    text::draw_oswald_header::<MaskTarget>,
    text::draw_rubik_big::<MaskTarget>,
];
const BASELINE: i32 = 40;
const BACK_CORNER: usize = 34;
const BIP39_WORDS: u16 = 2048;
const RECOVERY_WORD_BAND: std::ops::Range<usize> = 108..170;

/// The inked pixels of one string, relative to its top-left corner.
struct Glyphs {
    inked: Vec<(usize, usize)>,
    width: usize,
    height: usize,
    area: Vec<bool>,
}

#[derive(Default)]
struct MaskTarget {
    inked: Vec<(i32, i32)>,
}

impl OriginDimensions for MaskTarget {
    fn size(&self) -> Size {
        Size::new(4096, 256)
    }
}

impl DrawTarget for MaskTarget {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        self.inked
            .extend(pixels.into_iter().map(|Pixel(point, _)| (point.x, point.y)));
        Ok(())
    }
}

fn render(draw: DrawText, label: &str) -> Option<Glyphs> {
    let mut target = MaskTarget::default();
    let _ = draw(&mut target, label, 0, BASELINE, Rgb565::WHITE);
    let min_x = target.inked.iter().map(|point| point.0).min()?;
    let max_x = target.inked.iter().map(|point| point.0).max()?;
    let min_y = target.inked.iter().map(|point| point.1).min()?;
    let max_y = target.inked.iter().map(|point| point.1).max()?;
    let width = (max_x - min_x + 1) as usize;
    let height = (max_y - min_y + 1) as usize;
    let inked: Vec<(usize, usize)> = target
        .inked
        .iter()
        .map(|point| ((point.0 - min_x) as usize, (point.1 - min_y) as usize))
        .collect();
    let mut area = vec![false; width * height];
    for (x, y) in &inked {
        area[y * width + x] = true;
    }
    Some(Glyphs {
        inked,
        width,
        height,
        area,
    })
}

#[derive(PartialEq)]
pub(crate) struct Screen {
    pixels: Vec<u16>,
}

impl Screen {
    pub(crate) fn capture() -> Self {
        Self { pixels: snapshot() }
    }

    /// The most recent screens the firmware held long enough for a person to read.
    pub(crate) fn held_for_reading() -> Vec<Self> {
        screens_held_for_reading()
            .into_iter()
            .map(|pixels| Self { pixels })
            .collect()
    }

    /// The corner of the screen the back icon sits in.
    pub(crate) fn back_corner(&self) -> Vec<u16> {
        self.pixels
            .chunks(WIDTH)
            .take(BACK_CORNER)
            .flat_map(|row| row[..BACK_CORNER].iter().copied())
            .collect()
    }

    /// The centre of `label` when a person could read it, in any firmware font.
    pub(crate) fn find(&self, label: &str) -> Option<(u16, u16)> {
        FONTS.iter().find_map(|draw| self.find_in(*draw, label))
    }

    /// The recovery word a person reads off the word screen: the only word whose
    /// ink fills exactly the box of ink in the band the firmware draws it in.
    pub(crate) fn read_recovery_word(&self) -> Option<String> {
        let (left, top, width, height) = self.ink_box(RECOVERY_WORD_BAND)?;
        (0..BIP39_WORDS)
            .map(offline_signer::derivation::bip39::index_to_word)
            .find_map(|word| {
                let glyphs = render(text::draw_rubik_big::<MaskTarget>, word)?;
                let same_box = glyphs.width == width && glyphs.height == height;
                (same_box && self.matches_at(&glyphs, left, top)).then(|| word.to_owned())
            })
    }

    /// The box around everything that differs from the background within `band`.
    fn ink_box(&self, band: std::ops::Range<usize>) -> Option<(usize, usize, usize, usize)> {
        let background = self.pixels[band.start * WIDTH];
        let inked = |x: usize, y: usize| self.pixels[y * WIDTH + x] != background;
        let points: Vec<(usize, usize)> = band
            .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
            .filter(|(x, y)| inked(*x, *y))
            .collect();
        let left = points.iter().map(|point| point.0).min()?;
        let right = points.iter().map(|point| point.0).max()?;
        let top = points.iter().map(|point| point.1).min()?;
        let bottom = points.iter().map(|point| point.1).max()?;
        Some((left, top, right - left + 1, bottom - top + 1))
    }

    fn find_in(&self, draw: DrawText, label: &str) -> Option<(u16, u16)> {
        let glyphs = render(draw, label)?;
        for top in 0..=HEIGHT.checked_sub(glyphs.height)? {
            for left in 0..=WIDTH.checked_sub(glyphs.width)? {
                if self.matches_at(&glyphs, left, top) {
                    return Some((
                        (left + glyphs.width / 2) as u16,
                        (top + glyphs.height / 2) as u16,
                    ));
                }
            }
        }
        None
    }

    /// True when every glyph pixel shares one colour and no other pixel in the box has it.
    fn matches_at(&self, glyphs: &Glyphs, left: usize, top: usize) -> bool {
        let at = |x: usize, y: usize| self.pixels[(top + y) * WIDTH + left + x];
        let (first_x, first_y) = glyphs.inked[0];
        let ink = at(first_x, first_y);
        glyphs.inked.iter().all(|(x, y)| at(*x, *y) == ink)
            && (0..glyphs.height).all(|y| {
                (0..glyphs.width).all(|x| glyphs.area[y * glyphs.width + x] || at(x, y) != ink)
            })
    }

    pub(crate) fn save(&self, path: &str) {
        let file = std::fs::File::create(path).expect("screen file");
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), WIDTH as u32, HEIGHT as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        let bytes: Vec<u8> = self
            .pixels
            .iter()
            .flat_map(|pixel| {
                let color = Rgb565::from(RawU16::new(*pixel));
                [color.r() << 3, color.g() << 2, color.b() << 3]
            })
            .collect();
        writer.write_image_data(&bytes).expect("png data");
    }

    /// Saves what the device showed when a journey could not find `wanted`.
    pub(crate) fn save_for_failure(&self, wanted: &str) -> String {
        let name: String = wanted
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(24)
            .collect();
        let path = std::env::temp_dir().join(format!("kaskold-sim-missing-{name}.png"));
        let path = path.display().to_string();
        self.save(&path);
        path
    }
}
