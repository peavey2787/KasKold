//! The real firmware crate, compiled for the host against a virtual CoreS3.
//!
//! Host-unused firmware items are expected here. The firmware's own build keeps
//! its strict lint policy; this crate only re-hosts that source.
#![allow(dead_code, unused, non_snake_case, unexpected_cfgs, static_mut_refs)]

include!(concat!(env!("OUT_DIR"), "/firmware_root.rs"));

#[cfg(test)]
mod harness;
#[cfg(test)]
mod scenarios;
