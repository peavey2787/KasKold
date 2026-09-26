//! QR rendering and M5-compatible signed-response presentation profiles.

use qrcode::{types::Color, QrCode};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use super::hex_encode;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResponseFrame {
    pub(crate) index: u8,
    pub(crate) total: u8,
    pub(crate) payload_hex: String,
    pub(crate) svg: String,
}

pub(crate) fn render_response_frames(
    frames: &[vault_runtime::QrFrame],
) -> Result<Vec<ResponseFrame>, JsValue> {
    frames
        .iter()
        .map(|frame| {
            let svg = qr_svg(&frame.payload)?;
            Ok(ResponseFrame {
                index: frame.index,
                total: frame.total,
                payload_hex: hex_encode(&frame.payload),
                svg,
            })
        })
        .collect()
}

pub(crate) fn render_profiled_response_frames(
    payload: &[u8],
    payload_limit: usize,
) -> Result<Vec<ResponseFrame>, JsValue> {
    validate_profiled_payload(payload, payload_limit).map_err(JsValue::from_str)?;
    if uses_single_frame_profile(payload, payload_limit) {
        return render_single_response_frame(payload);
    }
    render_multi_response_frames(payload, payload_limit)
}

pub(crate) fn validate_profiled_payload(payload: &[u8], payload_limit: usize) -> Result<(), &'static str> {
    const ALLOWED_LIMITS: [usize; 5] = [91, 70, 40, 25, 12];
    if payload.is_empty() {
        return Err("Signed response is empty.");
    }
    if !ALLOWED_LIMITS.contains(&payload_limit) {
        return Err("Unsupported QR density.");
    }
    Ok(())
}

fn uses_single_frame_profile(payload: &[u8], payload_limit: usize) -> bool {
    const SINGLE_FRAME_LIMIT: usize = 134;
    payload_limit == 91 && payload.len() <= SINGLE_FRAME_LIMIT
}

fn render_single_response_frame(payload: &[u8]) -> Result<Vec<ResponseFrame>, JsValue> {
    Ok(vec![ResponseFrame {
        index: 0,
        total: 1,
        payload_hex: hex_encode(payload),
        svg: qr_svg(payload)?,
    }])
}

fn render_multi_response_frames(
    payload: &[u8],
    payload_limit: usize,
) -> Result<Vec<ResponseFrame>, JsValue> {
    let frame_count = payload.len().div_ceil(payload_limit);
    validate_frame_count(frame_count)?;
    let balanced_payload = payload.len().div_ceil(frame_count);
    let total = qr_frame_index(frame_count)?;
    let mut rendered = Vec::with_capacity(frame_count);
    for frame_index in 0..frame_count {
        rendered.push(render_response_fragment(
            payload,
            balanced_payload,
            frame_index,
            total,
        )?);
    }
    Ok(rendered)
}

fn validate_frame_count(frame_count: usize) -> Result<(), JsValue> {
    if !(2..=shared_signer::qr_frame::MAX_FRAMES).contains(&frame_count) {
        return Err(JsValue::from_str(
            "Signed response is too large for QR framing.",
        ));
    }
    Ok(())
}

fn qr_frame_index(value: usize) -> Result<u8, JsValue> {
    u8::try_from(value)
        .map_err(|_| JsValue::from_str("Signed response has too many QR frames."))
}

fn render_response_fragment(
    payload: &[u8],
    balanced_payload: usize,
    frame_index: usize,
    total: u8,
) -> Result<ResponseFrame, JsValue> {
    const FRAME_BUFFER_LEN: usize = 134;
    let offset = frame_index * balanced_payload;
    let fragment = payload_fragment(payload, offset, balanced_payload)?;
    let index = qr_frame_index(frame_index)?;
    let identifier = shared_signer::qr_frame::session_id(payload);
    let mut frame = [0u8; FRAME_BUFFER_LEN];
    let display_len = shared_signer::qr_frame::encode_frame(
        &identifier,
        index,
        total,
        fragment,
        &mut frame,
    )
    .map_err(|error| JsValue::from_str(&format!("QR frame failed: {error:?}")))?;
    response_frame(index, total, &frame[..display_len])
}

fn payload_fragment(
    payload: &[u8],
    offset: usize,
    balanced_payload: usize,
) -> Result<&[u8], JsValue> {
    let fragment_len = payload.len().saturating_sub(offset).min(balanced_payload);
    if fragment_len == 0 {
        return Err(JsValue::from_str("Signed response QR framing failed."));
    }
    Ok(&payload[offset..offset + fragment_len])
}

fn response_frame(index: u8, total: u8, bytes: &[u8]) -> Result<ResponseFrame, JsValue> {
    Ok(ResponseFrame {
        index,
        total,
        payload_hex: hex_encode(bytes),
        svg: qr_svg(bytes)?,
    })
}

pub(crate) fn qr_svg(data: &[u8]) -> Result<String, JsValue> {
    let code = QrCode::new(data)
        .map_err(|error| JsValue::from_str(&format!("QR failed: {error:?}")))?;
    Ok(qr_svg_document(&code))
}

fn qr_svg_document(code: &QrCode) -> String {
    let width = code.width();
    let quiet = 4usize;
    let view = width + quiet * 2;
    let modules = (0..width)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .filter(|&(x, y)| code[(x, y)] == Color::Dark)
        .map(|(x, y)| {
            format!(
                "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\" fill=\"black\"/>",
                x + quiet,
                y + quiet
            )
        })
        .collect::<String>();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {view} {view}\" shape-rendering=\"crispEdges\" role=\"img\" aria-label=\"QR code\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/>{modules}</svg>"
    )
}
