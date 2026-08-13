//! Probes and their hereditary specs.
//!
//! A probe is an individual (Bobiverse-style): it travels, surveys, founds a
//! colony, and its colony manufactures children whose specs *drift* slightly
//! from the founder's — replication is imperfect at interstellar remove.

use crate::galaxy::StarId;
use crate::rng::SplitMix64;
use crate::time::SimTime;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ProbeId(pub u64);

/// Hereditary performance characteristics. Mutates per generation.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProbeSpec {
    /// Cruise speed as a fraction of c.
    pub cruise_speed_c: f64,
    /// Multiplier on replication speed (higher = faster copies).
    pub fabrication: f64,
    /// Multiplier reducing transit attrition (higher = safer).
    pub reliability: f64,
}

impl ProbeSpec {
    pub fn baseline(cruise_speed_c: f64) -> Self {
        Self { cruise_speed_c, fabrication: 1.0, reliability: 1.0 }
    }

    /// Imperfect self-replication: each stat drifts by ~±`drift`,
    /// multiplicatively, clamped to sane bounds.
    pub fn mutate(&self, rng: &mut SplitMix64, drift: f64) -> Self {
        let mut jig = |v: f64, lo: f64, hi: f64| {
            (v * (1.0 + rng.range_f64(-drift, drift))).clamp(lo, hi)
        };
        Self {
            cruise_speed_c: jig(self.cruise_speed_c, 0.01, 0.5),
            fabrication: jig(self.fabrication, 0.25, 4.0),
            reliability: jig(self.reliability, 0.25, 4.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProbeState {
    /// En route between stars.
    InTransit { from: StarId, to: StarId, departed: SimTime, arrives: SimTime },
    /// Arrived; assaying the system.
    Surveying { star: StarId },
    /// Building the autofactory that turns a system into a colony.
    Colonizing { star: StarId },
    /// Settled as the seed intelligence of a colony.
    Settled { star: StarId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Probe {
    pub id: ProbeId,
    /// Replication generation; Sol's seed probe is generation 0.
    pub generation: u32,
    pub spec: ProbeSpec,
    pub state: ProbeState,
    /// Systems this probe surveyed and rejected (avoid revisiting).
    pub rejected: Vec<StarId>,
}
