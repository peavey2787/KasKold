pub mod master {
    use core::marker::PhantomData;

    use crate::{board, gpio::Pin, peripherals::I2C0, time::Rate, Blocking};

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Config;

    impl Config {
        pub fn with_frequency(self, _frequency: Rate) -> Self {
            self
        }
        pub fn with_software_timeout(self, _timeout: SoftwareTimeout) -> Self {
            self
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub enum SoftwareTimeout {
        Transaction(crate::time::Duration),
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Error {
        AcknowledgeCheckFailed,
    }

    impl embedded_hal::i2c::Error for Error {
        fn kind(&self) -> embedded_hal::i2c::ErrorKind {
            embedded_hal::i2c::ErrorKind::Other
        }
    }

    #[derive(Debug)]
    pub struct ConfigError;

    pub struct I2c<'d, Dm> {
        _mode: PhantomData<(&'d (), Dm)>,
    }

    impl<'d> I2c<'d, Blocking> {
        pub fn new(_peripheral: I2C0<'d>, _config: Config) -> Result<Self, ConfigError> {
            Ok(Self { _mode: PhantomData })
        }

        pub fn with_sda(self, _pin: crate::gpio::Pin) -> Self {
            self
        }

        pub fn with_scl(self, _pin: crate::gpio::Pin) -> Self {
            self
        }

        pub fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Error> {
            board::i2c::write(address, bytes)
        }

        pub fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Error> {
            board::i2c::read(address, buffer)
        }

        pub fn write_read(
            &mut self,
            address: u8,
            bytes: &[u8],
            buffer: &mut [u8],
        ) -> Result<(), Error> {
            board::i2c::write(address, bytes)?;
            board::i2c::read(address, buffer)
        }

        pub fn apply_config(&mut self, _config: &Config) -> Result<(), ConfigError> {
            Ok(())
        }
    }

    impl<Dm> embedded_hal::i2c::ErrorType for I2c<'_, Dm> {
        type Error = Error;
    }

    impl embedded_hal::i2c::I2c for I2c<'_, Blocking> {
        fn transaction(
            &mut self,
            address: u8,
            operations: &mut [embedded_hal::i2c::Operation<'_>],
        ) -> Result<(), Error> {
            for operation in operations {
                match operation {
                    embedded_hal::i2c::Operation::Read(buffer) => {
                        board::i2c::read(address, buffer)?
                    }
                    embedded_hal::i2c::Operation::Write(bytes) => {
                        board::i2c::write(address, bytes)?
                    }
                }
            }
            Ok(())
        }
    }
}
