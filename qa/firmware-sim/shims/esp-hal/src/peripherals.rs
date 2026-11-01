use core::marker::PhantomData;

macro_rules! singletons {
    ($($name:ident),* $(,)?) => {
        $(
            #[allow(non_camel_case_types)]
            pub struct $name<'d>(PhantomData<&'d ()>);
            impl $name<'_> {
                /// # Safety
                /// Matches the vendor signature; these handles carry no state.
                pub unsafe fn steal() -> Self {
                    Self(PhantomData)
                }
            }
        )*
    };
}

macro_rules! peripherals {
    ($($name:ident),* $(,)?; $($pin:ident = $number:literal),* $(,)?) => {
        singletons!($($name,)*);

        // Upper-case fields mirror the esp-hal API the firmware is written against.
        #[allow(non_snake_case)]
        pub struct Peripherals {
            $(pub $name: $name<'static>,)*
            $(pub $pin: crate::gpio::Pin,)*
        }

        impl Peripherals {
            pub(crate) fn take() -> Self {
                Self {
                    $($name: $name(PhantomData),)*
                    $($pin: crate::gpio::Pin($number),)*
                }
            }
        }
    };
}

peripherals!(
    CPU_CTRL, DMA_CH0, DMA_CH1, FLASH, HMAC, I2C0, I2S1, LCD_CAM, PSRAM, SPI2, TIMG0, TIMG1, USB_DEVICE;
    GPIO2 = 2, GPIO3 = 3, GPIO4 = 4, GPIO11 = 11, GPIO12 = 12, GPIO13 = 13, GPIO14 = 14, GPIO15 = 15,
    GPIO16 = 16, GPIO33 = 33, GPIO34 = 34, GPIO35 = 35, GPIO36 = 36, GPIO37 = 37, GPIO38 = 38,
    GPIO39 = 39, GPIO40 = 40, GPIO41 = 41, GPIO42 = 42, GPIO45 = 45, GPIO46 = 46, GPIO47 = 47, GPIO48 = 48,
);
