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

// Manual dice entropy collection.

/// Dice roll entropy collector.
///
/// Each dice roll (1-6) contributes log2(6) ≈ 2.585 bits of entropy.
/// For 128 bits: need 50 rolls minimum, we use 99 for safety margin.
/// For 256 bits: need 100 rolls minimum, we use 198.
///
/// Method: Hash all rolls with SHA256 to extract uniform entropy.
/// This is the same approach used by SeedSigner and ColdCard.
pub struct DiceCollector {
    /// Raw dice values (1-6)
    pub rolls: [u8; 200],
    /// Number of rolls collected
    pub count: usize,
    /// Target number of rolls
    pub target: usize,
}

impl DiceCollector {
    /// Create a dice collector targeting 12-word mnemonic (128 bits).
    pub fn new_12_word() -> Self {
        Self { rolls: [0; 200], count: 0, target: 99 }
    }

    /// Reconfigure this collector for optional additive dice entropy.
    /// Returns false rather than silently clamping an invalid target.
    pub fn configure_target(&mut self, target: usize) -> bool {
        if target == 0 || target > self.rolls.len() {
            return false;
        }
        self.zeroize();
        self.target = target;
        true
    }

    /// Add a dice roll (value 1-6)
    pub fn add_roll(&mut self, value: u8) -> bool {
        if !(1..=6).contains(&value) || self.count >= self.target {
            return false;
        }
        self.rolls[self.count] = value;
        self.count += 1;
        true
    }

    /// Remove last roll
    pub fn undo(&mut self) {
        if self.count > 0 {
            self.count -= 1;
            self.rolls[self.count] = 0;
        }
    }

    /// Check if we have enough rolls
    pub fn is_complete(&self) -> bool {
        self.count >= self.target
    }

    /// Zeroize every collected roll and reset the collector.
    pub fn zeroize(&mut self) {
        shared_signer::bytes::zeroize_bytes(&mut self.rolls);
        shared_signer::bytes::volatile_clear(core::slice::from_mut(&mut self.count), 0usize);
    }
}

impl Drop for DiceCollector {
    fn drop(&mut self) {
        self.zeroize();
    }
}
