use core::marker::PhantomData;

use crate::board;

/// A numbered pad, which is everything the virtual board needs to route it.
#[derive(Clone, Copy, Debug)]
pub struct Pin(pub u8);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Level {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OutputConfig;

pub struct Output<'d> {
    pin: u8,
    _lifetime: PhantomData<&'d ()>,
}

impl<'d> Output<'d> {
    pub fn new(pin: Pin, level: Level, _config: OutputConfig) -> Self {
        board::pins::set(pin.0, level == Level::High);
        Self { pin: pin.0, _lifetime: PhantomData }
    }

    pub fn set_high(&mut self) {
        board::pins::set(self.pin, true);
    }

    pub fn set_low(&mut self) {
        board::pins::set(self.pin, false);
    }
}

impl embedded_hal::digital::ErrorType for Output<'_> {
    type Error = core::convert::Infallible;
}

impl embedded_hal::digital::OutputPin for Output<'_> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Output::set_low(self);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Output::set_high(self);
        Ok(())
    }
}
