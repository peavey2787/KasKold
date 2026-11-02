//! Shipment-escrow transaction and address façade.

mod address;
mod terms;

#[cfg(test)]
pub(crate) use address::build_shipping_escrow_json;
pub use address::covenant_ship_escrow;
pub use terms::ship_escrow_terms;
#[cfg(test)]
pub(crate) use terms::ship_escrow_terms_json;

#[cfg(test)]
mod unit_tests;
