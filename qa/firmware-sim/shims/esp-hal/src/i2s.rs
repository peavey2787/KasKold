pub mod master {
    use core::marker::PhantomData;

    use crate::{
        dma::{DmaDescriptor, DmaTransferTxCircular},
        gpio::Pin,
        peripherals::{DMA_CH1, I2S1},
        time::Rate,
        Blocking,
    };

    #[derive(Clone, Copy, Debug)]
    pub enum DataFormat {
        Data16Channel16,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Channels;

    impl Channels {
        pub const STEREO: Self = Self;
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Config;

    impl Config {
        pub fn new_tdm_philips() -> Self {
            Self
        }
        pub fn with_sample_rate(self, _rate: Rate) -> Self {
            self
        }
        pub fn with_data_format(self, _format: DataFormat) -> Self {
            self
        }
        pub fn with_channels(self, _channels: Channels) -> Self {
            self
        }
    }

    #[derive(Debug)]
    pub struct ConfigError;

    #[derive(Clone, Copy, Debug)]
    pub struct I2sError;

    pub struct TxCreator<'d>(PhantomData<&'d ()>);

    impl<'d> TxCreator<'d> {
        pub fn with_bclk(self, _pin: Pin) -> Self {
            self
        }
        pub fn with_ws(self, _pin: Pin) -> Self {
            self
        }
        pub fn with_dout(self, _pin: Pin) -> Self {
            self
        }
        pub fn build(self, _descriptors: &'static mut [DmaDescriptor]) -> I2sTx<'d, Blocking> {
            I2sTx(PhantomData)
        }
    }

    pub struct I2s<'d> {
        pub i2s_tx: TxCreator<'d>,
    }

    impl<'d> I2s<'d> {
        pub fn new(_i2s: I2S1<'d>, _channel: DMA_CH1<'d>, _config: Config) -> Result<Self, ConfigError> {
            Ok(Self { i2s_tx: TxCreator(PhantomData) })
        }
    }

    pub struct I2sTx<'d, Dm>(PhantomData<(&'d (), Dm)>);

    impl<'d, Dm> I2sTx<'d, Dm> {
        pub fn write_dma_circular<'t>(
            &'t mut self,
            _words: &'t [u8],
        ) -> Result<DmaTransferTxCircular<'t, Self>, I2sError> {
            Ok(DmaTransferTxCircular { _transmitter: PhantomData })
        }
    }
}
