//! Host stand-ins for the `esp_hal` entry and placement attributes.
use proc_macro::TokenStream;

/// RAM placement has no meaning on the host, so the item passes through.
#[proc_macro_attribute]
pub fn ram(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// The harness starts `firmware_main` itself, so the entry point stays an ordinary function.
#[proc_macro_attribute]
pub fn main(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}
