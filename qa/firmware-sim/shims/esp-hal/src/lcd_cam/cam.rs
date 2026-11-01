use core::marker::PhantomData;

use crate::{board, dma::DmaRxBuf, gpio::Pin, peripherals::DMA_CH0, time::Rate};

pub struct Cam<'d>(PhantomData<&'d ()>);

impl Cam<'_> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Config;

impl Config {
    pub fn with_frequency(self, _frequency: Rate) -> Self {
        self
    }
}

#[derive(Debug)]
pub struct ConfigError;

#[derive(Debug)]
pub struct DmaError;

pub struct Camera<'d>(PhantomData<&'d ()>);

macro_rules! pin_builders {
    ($($name:ident),* $(,)?) => {
        $(pub fn $name(self, _pin: Pin) -> Self { self })*
    };
}

impl<'d> Camera<'d> {
    pub fn new(_cam: Cam<'d>, _channel: DMA_CH0<'d>, _config: Config) -> Result<Self, ConfigError> {
        Ok(Self(PhantomData))
    }

    pin_builders!(
        with_master_clock, with_pixel_clock, with_vsync, with_h_enable, with_data0, with_data1,
        with_data2, with_data3, with_data4, with_data5, with_data6, with_data7,
    );

    pub fn receive(self, mut buffer: DmaRxBuf) -> Result<CameraTransfer<'d>, (DmaError, Self, DmaRxBuf)> {
        board::camera::capture(buffer.as_mut_slice());
        Ok(CameraTransfer { camera: self, buffer })
    }
}

pub struct CameraTransfer<'d> {
    camera: Camera<'d>,
    buffer: DmaRxBuf,
}

impl<'d> CameraTransfer<'d> {
    pub fn is_done(&self) -> bool {
        true
    }

    pub fn wait(self) -> (Result<(), DmaError>, Camera<'d>, DmaRxBuf) {
        (Ok(()), self.camera, self.buffer)
    }

    pub fn stop(self) -> (Camera<'d>, DmaRxBuf) {
        (self.camera, self.buffer)
    }
}
