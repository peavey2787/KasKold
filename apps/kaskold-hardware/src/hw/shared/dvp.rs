//! Bounded ESP-HAL DVP DMA capture primitive shared by camera consumers.

use esp_hal::{delay::Delay, dma::DmaRxBuf, lcd_cam::cam::Camera};

use crate::hw::camera::CameraStatus;

const CAPTURE_POLL_US: u32 = 50;
const CAPTURE_MAX_POLLS: u32 = 10_000; // 500 ms: UI must never block for multi-second camera waits.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameCaptureStatus {
    Complete,
    TimedOut,
    ReceiveFailed,
}

/// The DVP camera, its DMA receive buffer, and the sensor status.
///
/// Every capture lends the camera and buffer to one transfer and always returns
/// both, so consumers hold them as one port instead of two loose options.
pub(crate) struct CameraPort<'cam> {
    camera: Option<Camera<'cam>>,
    buffer: Option<DmaRxBuf>,
    pub(crate) status: CameraStatus,
}

impl<'cam> CameraPort<'cam> {
    pub(crate) const fn new(camera: Option<Camera<'cam>>, buffer: Option<DmaRxBuf>, status: CameraStatus) -> Self {
        Self { camera, buffer, status }
    }

    /// Both the camera peripheral and its DMA buffer are available for a capture.
    pub(crate) const fn is_ready(&self) -> bool {
        self.camera.is_some() && self.buffer.is_some()
    }

    /// Receive one frame. `observe` sees the frame bytes only when the transfer
    /// completed, before the buffer returns to the port. `None` means the camera
    /// or its buffer is unavailable.
    pub(crate) fn capture(&mut self, delay: &mut Delay, observe: impl FnOnce(&[u8])) -> Option<FrameCaptureStatus> {
        let camera = self.camera.take()?;
        let Some(buffer) = self.buffer.take() else {
            self.camera = Some(camera);
            return None;
        };
        let (status, camera, buffer) = receive_full_frame(camera, buffer, delay);
        if status == FrameCaptureStatus::Complete {
            observe(buffer.as_slice());
        }
        self.camera = Some(camera);
        self.buffer = Some(buffer);
        Some(status)
    }
}

/// Receive one descriptor-bounded DVP buffer without an unbounded `wait()` spin.
///
/// `Camera::receive` enables CAM_STOP_EN. Once `is_done()` is true, ESP-HAL has
/// observed the camera stop after the DMA path stopped draining (normally because
/// the supplied descriptor-backed buffer is full). Only then is `wait()` used;
/// on timeout the partial transfer is stopped and must not be consumed as a frame.
fn receive_full_frame<'a>(
    camera: Camera<'a>,
    buffer: DmaRxBuf,
    delay: &mut Delay,
) -> (FrameCaptureStatus, Camera<'a>, DmaRxBuf) {
    let transfer = match camera.receive(buffer) {
        Ok(transfer) => transfer,
        Err((_error, camera, buffer)) => {
            return (FrameCaptureStatus::ReceiveFailed, camera, buffer);
        }
    };

    let mut polls = 0u32;
    loop {
        if transfer.is_done() {
            let (_dma_result, camera, buffer) = transfer.wait();
            return (FrameCaptureStatus::Complete, camera, buffer);
        }
        polls = polls.saturating_add(1);
        if polls > CAPTURE_MAX_POLLS {
            let (camera, buffer) = transfer.stop();
            return (FrameCaptureStatus::TimedOut, camera, buffer);
        }
        delay.delay_micros(CAPTURE_POLL_US);
    }
}
