//! QR codes as a person meets them: held up to the lens, or read off the panel.
use esp_hal::board::{camera, lcd};
use kaspa_portal::transaction::interchange::qr::frame::{
    encode_frame, session_id, FRAME_HEADER_LEN,
};
use qrcode::{Color, EcLevel, QrCode};

const QUIET_MODULES: usize = 4;
const WHITE: u8 = 255;
const BLACK: u8 = 0;
/// What a phone puts in each frame of an animated QR.
const FRAME_FRAGMENT: usize = 91;
const CAPTURES_PER_FRAME: usize = 3;

/// Holds one QR code in front of the camera, as large as the frame allows.
pub(crate) fn hold_up_qr(payload: &[u8]) {
    camera::show(vec![qr_picture(payload)]);
}

/// Plays a payload too big for one QR code as its looping frame sequence, as a phone would.
pub(crate) fn hold_up_frames(payload: &[u8]) {
    let total = payload.len().div_ceil(FRAME_FRAGMENT);
    let session = session_id(payload);
    let pictures: Vec<_> = payload
        .chunks(FRAME_FRAGMENT)
        .enumerate()
        .map(|(index, fragment)| {
            let mut wire = [0u8; FRAME_FRAGMENT + FRAME_HEADER_LEN];
            let length = encode_frame(
                &session,
                u8::try_from(index).unwrap(),
                u8::try_from(total).unwrap(),
                fragment,
                &mut wire,
            )
            .expect("a QR frame");
            qr_picture(&wire[..length])
        })
        .collect();
    camera::show(held_for_several_captures(pictures));
}

/// A phone holds each frame for many camera captures, and the firmware does not read every capture.
fn held_for_several_captures(pictures: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    pictures
        .into_iter()
        .flat_map(|picture| std::iter::repeat_n(picture, CAPTURES_PER_FRAME))
        .collect()
}

/// Takes the QR code away, leaving the sensor looking at nothing in particular.
pub(crate) fn lower_qr() {
    camera::show(Vec::new());
}

fn qr_picture(payload: &[u8]) -> Vec<u8> {
    let code =
        QrCode::with_error_correction_level(payload, EcLevel::L).expect("payload fits a QR code");
    let modules = code.width();
    let scale = camera::HEIGHT / (modules + 2 * QUIET_MODULES);
    assert!(
        scale >= 2,
        "a {modules}-module QR code is too dense for the frame"
    );
    let left = (camera::WIDTH - modules * scale) / 2;
    let top = (camera::HEIGHT - modules * scale) / 2;
    let colors = code.to_colors();
    let mut picture = vec![WHITE; camera::WIDTH * camera::HEIGHT];
    for (index, color) in colors.iter().enumerate() {
        if *color == Color::Dark {
            let (column, row) = (index % modules, index / modules);
            paint_module(
                &mut picture,
                left + column * scale,
                top + row * scale,
                scale,
            );
        }
    }
    picture
}

fn paint_module(picture: &mut [u8], x: usize, y: usize, scale: usize) {
    for line in y..y + scale {
        picture[line * camera::WIDTH + x..line * camera::WIDTH + x + scale].fill(BLACK);
    }
}

/// Everything readable from the QR code on the panel, as another device's camera sees it.
pub(crate) fn read_qr_on_screen() -> Option<Vec<u8>> {
    let pixels = lcd::snapshot();
    let mut image =
        rqrr::PreparedImage::prepare_from_greyscale(lcd::WIDTH, lcd::HEIGHT, |x, y| {
            luma(pixels[y * lcd::WIDTH + x])
        })?;
    image.detect_grids().into_iter().find_map(|grid| {
        let mut payload = Vec::new();
        grid.decode_to(&mut payload).ok().map(|_| payload)
    })
}

fn luma(rgb565: u16) -> u8 {
    let red = (rgb565 >> 11) & 0x1F;
    let green = (rgb565 >> 5) & 0x3F;
    let blue = rgb565 & 0x1F;
    u8::try_from((red * 2 + green * 2 + blue * 2) * 255 / (31 * 2 + 63 * 2 + 31 * 2)).unwrap()
}
