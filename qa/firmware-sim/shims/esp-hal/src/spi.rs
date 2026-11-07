#[derive(Clone, Copy, Debug, Default)]
pub enum Mode {
    #[default]
    _0,
}

pub mod master {
    use core::marker::PhantomData;

    use super::Mode;
    use crate::{board, gpio::Pin, peripherals::SPI2, time::Rate, Blocking};

    /// Opaque like esp-hal's, whose configuration has private fields.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Config {
        _private: (),
    }

    impl Config {
        pub fn with_frequency(self, _frequency: Rate) -> Self {
            self
        }
        pub fn with_mode(self, _mode: Mode) -> Self {
            self
        }
    }

    #[derive(Debug)]
    pub struct ConfigError;

    #[derive(Clone, Copy, Debug)]
    pub struct Error;

    impl embedded_hal::spi::Error for Error {
        fn kind(&self) -> embedded_hal::spi::ErrorKind {
            embedded_hal::spi::ErrorKind::Other
        }
    }

    pub struct Spi<'d, Dm> {
        _mode: PhantomData<(&'d (), Dm)>,
    }

    impl<'d> Spi<'d, Blocking> {
        pub fn new(_peripheral: SPI2<'d>, _config: Config) -> Result<Self, ConfigError> {
            Ok(Self { _mode: PhantomData })
        }
        pub fn with_sck(self, _pin: Pin) -> Self {
            self
        }
        pub fn with_mosi(self, _pin: Pin) -> Self {
            self
        }
        pub fn with_miso(self, _pin: Pin) -> Self {
            self
        }
        pub fn apply_config(&mut self, _config: &Config) -> Result<(), ConfigError> {
            Ok(())
        }
        pub fn write(&mut self, words: &[u8]) -> Result<(), Error> {
            board::spi::exchange(words, &mut []);
            Ok(())
        }
        pub fn read(&mut self, words: &mut [u8]) -> Result<(), Error> {
            board::spi::exchange(&[], words);
            Ok(())
        }
        pub fn transfer(&mut self, words: &mut [u8]) -> Result<(), Error> {
            let out = words.to_vec();
            board::spi::exchange(&out, words);
            Ok(())
        }
    }

    impl<Dm> embedded_hal::spi::ErrorType for Spi<'_, Dm> {
        type Error = Error;
    }

    impl embedded_hal::spi::SpiBus<u8> for Spi<'_, Blocking> {
        fn read(&mut self, words: &mut [u8]) -> Result<(), Error> {
            Spi::read(self, words)
        }
        fn write(&mut self, words: &[u8]) -> Result<(), Error> {
            Spi::write(self, words)
        }
        fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Error> {
            board::spi::exchange(write, read);
            Ok(())
        }
        fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Error> {
            Spi::transfer(self, words)
        }
        fn flush(&mut self) -> Result<(), Error> {
            Ok(())
        }
    }
}
