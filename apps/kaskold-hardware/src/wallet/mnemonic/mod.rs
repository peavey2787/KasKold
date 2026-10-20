// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// Stable mnemonic-domain facade. Focused modules own dice entropy,
// mnemonic generation, validation, and word input.

mod dice;
mod generation;
mod touch;
mod validation;
mod word_input;

pub use dice::DiceCollector;
pub use generation::{generate_from_dice, generate_from_entropy};
pub use touch::TouchEntropyCollector;
pub use validation::validate;
pub use word_input::WordInput;

#[cfg(any(test, feature = "verbose-boot"))]
#[path = "../unit_tests/mnemonic_tests.rs"]
pub mod unit_tests;
