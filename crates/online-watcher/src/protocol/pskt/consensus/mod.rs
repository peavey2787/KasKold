// KasKold Companion Web — organized PSKT subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

mod finalizer;
#[cfg(test)]
mod input;
#[cfg(test)]
mod legacy_finalizer;
#[cfg(test)]
mod output;

pub(crate) use finalizer::finalize_to_consensus;
#[cfg(test)]
pub(crate) use input::build_consensus_input;
#[cfg(test)]
pub(crate) use output::build_consensus_output;

#[cfg(test)]
pub(super) use legacy_finalizer::finalize_to_consensus_unverified_for_test;
