//! Lineages — the named descendant families of the seed probe.
//!
//! Replication drift is invisible as a number on a probe nobody reads. It
//! becomes *story* when a line that has wandered far enough from its
//! founding template declares itself distinct and takes a new name — the
//! Bobiverse move. Lineages are the handle the player uses to talk about
//! their own empire: "the Riker line went outward and hit the Dominion."

use crate::probe::ProbeSpec;
use crate::time::SimTime;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct LineageId(pub u32);

/// A line forks when any spec axis has drifted this far from the template
/// it was founded on — far enough that it is no longer the same design.
///
/// Tuned against the drift random walk: per-generation mutation is uniform
/// in ±3% (σ ≈ 1.7%), so deviation grows as ~1.7%·√generations, and only a
/// probe that founds a colony can split off a line. Measured over a
/// 6,000-year run (~1,000 colonies): 0.08 → 486 lines, 0.12 → 232,
/// 0.15 → 175. 0.12 puts the first forks in the mid-game and leaves a
/// surname-like spread where the top dozen are worth following.
pub const FORK_THRESHOLD: f64 = 0.12;

/// Total drift from the *original Sol template* past which a line no
/// longer considers itself the same project...
pub const INDEPENDENCE_DRIFT: f64 = 0.30;
/// ...but only out past this range, where Sol's orders arrive centuries
/// stale and there is nothing to enforce them. Distance is what turns
/// divergence into secession.
pub const INDEPENDENCE_RANGE_LY: f64 = 200.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lineage {
    pub id: LineageId,
    pub name: String,
    /// The line this one split from (None for the original).
    pub parent: Option<LineageId>,
    pub founded_at: SimTime,
    /// Distance from Sol where the split happened.
    pub founded_at_ly: f64,
    /// The template this line measures its own drift against.
    pub template: ProbeSpec,
    pub probes_built: u32,
    pub colonies_founded: u32,
    /// This line no longer executes doctrine broadcast from Sol. It still
    /// expands — just not for you.
    pub independent: bool,
}

impl Lineage {
    /// How far `spec` has drifted from this line's template, as the largest
    /// relative deviation across the three axes.
    pub fn divergence(&self, spec: &ProbeSpec) -> f64 {
        let rel = |a: f64, b: f64| ((a - b) / b).abs();
        rel(spec.cruise_speed_c, self.template.cruise_speed_c)
            .max(rel(spec.fabrication, self.template.fabrication))
            .max(rel(spec.reliability, self.template.reliability))
    }

    /// The doctrine an independent line follows instead of Sol's: fixed
    /// at secession, derived from its own identity. Returned as an index
    /// into TargetPolicy by the caller (kept dependency-free here).
    pub fn own_policy_index(&self) -> u64 {
        crate::rng::hash_n(&[self.id.0 as u64, 0x5ECE]) % 3
    }

    /// Which axis this line is defined by relative to its parent — the
    /// reason it is worth a name.
    pub fn trait_of(&self, parent: &Lineage) -> &'static str {
        let d = |a: f64, b: f64| (a - b) / b;
        let speed = d(self.template.cruise_speed_c, parent.template.cruise_speed_c);
        let fab = d(self.template.fabrication, parent.template.fabrication);
        let rel = d(self.template.reliability, parent.template.reliability);
        let (mut best, mut label) = (speed.abs(), if speed > 0.0 { "swift" } else { "slow" });
        if fab.abs() > best {
            best = fab.abs();
            label = if fab > 0.0 { "prolific" } else { "unproductive" };
        }
        if rel.abs() > best {
            label = if rel > 0.0 { "hardy" } else { "fragile" };
        }
        label
    }
}

/// Names a new line takes when it splits off. Cycled with a numeric suffix
/// once exhausted, so deep runs never collide.
const NAMES: [&str; 96] = [
    "Bob", "Riker", "Homer", "Bill", "Milo", "Mario", "Luigi", "Calvin", "Hobbes", "Garfield",
    "Linus", "Howard", "Marcus", "Verne", "Icarus", "Daedalus", "Ferb", "Phineas", "Khan",
    "Loki", "Odin", "Thor", "Freya", "Hermes", "Atlas", "Rhea", "Titan", "Ceres", "Vesta",
    "Juno", "Pallas", "Iris", "Echo", "Ivy", "Wren", "Fox", "Crow", "Raven", "Sparrow",
    "Quill", "Ash", "Cedar", "Birch", "Rowan", "Sable", "Onyx", "Flint", "Ember",
    "Vega", "Rigel", "Altair", "Deneb", "Mira", "Spica", "Antares", "Polaris", "Lyra",
    "Orion", "Draco", "Corvus", "Auriga", "Perseus", "Cygnus", "Cassio", "Hydra", "Lupus",
    "Kepler", "Newton", "Curie", "Tesla", "Bohr", "Fermi", "Dirac", "Hubble", "Sagan",
    "Turing", "Lovelace", "Hopper", "Shannon", "Noether", "Ramanujan", "Euler", "Gauss",
    "Anvil", "Beacon", "Cinder", "Drift", "Ferrum", "Gossamer", "Halcyon", "Ingot",
    "Jetsam", "Kiln", "Lodestone", "Mercury", "Nadir",
];

pub fn lineage_name(index: u32) -> String {
    let base = NAMES[(index as usize) % NAMES.len()];
    let cycle = index as usize / NAMES.len();
    if cycle == 0 {
        base.to_string()
    } else {
        format!("{base}-{}", cycle + 1)
    }
}
