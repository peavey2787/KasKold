use crate::board;

/// A field is named by its position in the vendor table; the model keeps a value per name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfuseField(pub &'static str);

macro_rules! fields {
    ($($name:ident),* $(,)?) => {
        $(pub const $name: EfuseField = EfuseField(stringify!($name));)*
    };
}

fields!(
    DIS_DOWNLOAD_MANUAL_ENCRYPT,
    DIS_PAD_JTAG,
    DIS_USB_JTAG,
    KEY_PURPOSE_0,
    KEY_PURPOSE_1,
    KEY_PURPOSE_2,
    KEY_PURPOSE_3,
    KEY_PURPOSE_4,
    KEY_PURPOSE_5,
    OPTIONAL_UNIQUE_ID,
    RD_DIS,
    SECURE_BOOT_EN,
    SECURE_BOOT_KEY_REVOKE0,
    SECURE_BOOT_KEY_REVOKE1,
    SECURE_BOOT_KEY_REVOKE2,
    SECURE_VERSION,
    SPI_BOOT_CRYPT_CNT,
    WR_DIS,
);

/// Little-endian widening from the field's stored bytes.
pub trait FieldValue: Sized {
    fn from_bytes(bytes: [u8; 16]) -> Self;
}

macro_rules! scalar_values {
    ($($ty:ty),*) => {
        $(impl FieldValue for $ty {
            fn from_bytes(bytes: [u8; 16]) -> Self {
                let mut raw = [0u8; core::mem::size_of::<$ty>()];
                raw.copy_from_slice(&bytes[..core::mem::size_of::<$ty>()]);
                <$ty>::from_le_bytes(raw)
            }
        })*
    };
}
scalar_values!(u8, u16, u32);

impl FieldValue for [u32; 4] {
    fn from_bytes(bytes: [u8; 16]) -> Self {
        core::array::from_fn(|i| {
            u32::from_le_bytes([
                bytes[4 * i],
                bytes[4 * i + 1],
                bytes[4 * i + 2],
                bytes[4 * i + 3],
            ])
        })
    }
}

pub struct Efuse;

impl Efuse {
    pub fn read_field_le<T: FieldValue>(field: EfuseField) -> T {
        T::from_bytes(board::efuse::field(field.0))
    }

    pub fn read_base_mac_address() -> [u8; 6] {
        board::efuse::base_mac()
    }

    pub fn flash_encryption() -> bool {
        board::efuse::field("SPI_BOOT_CRYPT_CNT")[0].count_ones() % 2 == 1
    }
}
