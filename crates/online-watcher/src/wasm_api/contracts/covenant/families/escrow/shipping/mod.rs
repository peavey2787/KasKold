//! Shipment-escrow transaction and address façade.

mod address;
mod deposit;
mod terms;
mod withdraw;

#[cfg(test)]
pub(crate) use address::build_shipping_escrow_json;
pub use address::covenant_ship_escrow;
pub use deposit::create_covenant_borrower_spend;
pub use terms::ship_escrow_terms;
#[cfg(test)]
pub(crate) use terms::ship_escrow_terms_json;
pub use withdraw::create_covenant_borrower_withdraw;

#[cfg(test)]
mod unit_tests;
