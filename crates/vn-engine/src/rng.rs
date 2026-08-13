//! Deterministic randomness. Two flavors:
//! - `mix`/`hash_n`: stateless hashing for procedural generation — the same
//!   (seed, coords, salt) always yields the same star, with no generation
//!   order dependence.
//! - `SplitMix64`: a tiny stateful stream for event-time rolls (attrition,
//!   mutation), forked per-purpose so adding a new consumer never perturbs
//!   existing streams.
//!
//! No external RNG crates: the whole point is that these bits are stable
//! across platforms and dependency upgrades, forever.

use serde::{Deserialize, Serialize};

/// splitmix64 finalizer — good avalanche, trivially portable.
#[inline]
pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash an arbitrary list of u64 words into one value.
pub fn hash_n(words: &[u64]) -> u64 {
    let mut acc = 0x51_7C_C1_B7_27_22_0A_95u64;
    for &w in words {
        acc = mix(acc ^ w);
    }
    acc
}

/// Map a u64 to a uniform f64 in [0, 1) using the top 53 bits.
#[inline]
pub fn unit_f64(bits: u64) -> f64 {
    (bits >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Derive an independent stream; `tag` names the purpose.
    pub fn fork(&self, tag: u64) -> Self {
        Self { state: mix(self.state ^ mix(tag)) }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    pub fn next_f64(&mut self) -> f64 {
        unit_f64(self.next_u64())
    }

    /// Uniform in [lo, hi).
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_f64() * (hi - lo)
    }
}
