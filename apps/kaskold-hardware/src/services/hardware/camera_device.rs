//! Controller-facing CoreS3 camera transport facade.

pub(crate) use crate::hw::shared::dvp::FrameCaptureStatus;

pub(crate) fn receive_full_frame<'a>(
    camera: esp_hal::lcd_cam::cam::Camera<'a>,
    buffer: esp_hal::dma::DmaRxBuf,
    delay: &mut esp_hal::delay::Delay,
) -> (FrameCaptureStatus, esp_hal::lcd_cam::cam::Camera<'a>, esp_hal::dma::DmaRxBuf) {
    crate::hw::shared::dvp::receive_full_frame(camera, buffer, delay)
}
