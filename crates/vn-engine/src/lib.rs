//! vn-engine — deterministic discrete-event simulation core for a
//! von Neumann probe game.
//!
//! Design principles:
//! - **Event-driven, not tick-driven.** Interstellar timescales mean almost
//!   nothing changes in any given second. Cost scales with the number of
//!   events (arrivals, replications, surveys), never with entity-count ×
//!   frames. There is no tick loop anywhere in this crate.
//! - **Lazy, procedural space.** Stars are generated deterministically from
//!   (seed, cell coords) on demand. Only systems a probe has touched hold
//!   mutable state. Untouched galaxy costs zero memory and zero CPU.
//! - **Deterministic.** Same seed + same config ⇒ bit-identical outcomes,
//!   verified by `Simulation::digest()`. All randomness flows from seeded
//!   splitmix64 streams; no HashMap iteration order ever affects logic.
//! - **Physical constraints are gameplay.** No FTL: probes cruise at a
//!   fraction of c and news propagates home at light speed (see `report`).
//!   Replication is gated by local resource richness, hops are range-limited
//!   (delta-v), transit carries attrition risk, and each generation's spec
//!   drifts slightly from its parent's.

pub mod civs;
pub mod events;
pub mod galaxy;
pub mod lineage;
pub mod probe;
pub mod report;
pub mod rng;
pub mod sim;
pub mod time;

use serde::{Deserialize, Serialize};

/// Expansion doctrine: how a colony's children choose their targets.
/// This is *policy set in advance* — the probes execute it autonomously,
/// because by the time you could veto a launch, it happened decades ago.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetPolicy {
    /// Minimize hop distance; dense, consolidated growth.
    Nearest,
    /// Spend extra light-years to reach spectroscopically rich systems.
    Richest,
    /// Prefer hops that gain radial distance from Sol; race outward.
    Outward,
    /// Prospecting for life. Garden-world odds scale with system richness,
    /// so this aims hard at rich systems (harder than `Richest`) while
    /// keeping a normal settlement bar, trading colony quality for the
    /// volume of *arrivals at promising systems* the search depends on.
    Survey,
}

/// Which axis a colony spends extra fabrication time improving. Directed
/// investment biases replication drift instead of leaving it a pure random
/// walk — the player's one way to steer evolution rather than watch it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecAxis {
    Speed,
    Fabrication,
    Reliability,
}

/// Directed replication costs this multiple of the normal build time.
///
/// Time alone can't be the price: transit takes centuries while a replica
/// takes ~3 years, so any build-time penalty is noise against the decades
/// a faster drive saves — engineering came out strictly better. The real
/// cost is material (see MATERIAL_PER_PROBE); this is flavor on top.
pub const INVESTMENT_TIME_COST: f64 = 1.25;

/// A colony's budget is tracked in abstract material units rather than
/// whole probes, so the cost of engineering can be a real 1.5× instead of
/// a brutal 2× — at 2× the halved output swamped the slow (~3%/generation)
/// gain from directed drift, and engineering was never worth it.
pub const MATERIAL_PER_PROBE: u32 = 2;
/// Material for one engineered replica: better tooling, more waste.
pub const MATERIAL_PER_ENGINEERED_PROBE: u32 = 3;

/// Tunable parameters for a simulation run. All time values are in years,
/// all distances in light-years, speeds in fractions of c.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimConfig {
    pub seed: u64,
    /// Edge length of one procedural galaxy cell.
    pub cell_size_ly: f64,
    /// Baseline cruise speed of a freshly-built probe.
    pub cruise_speed_c: f64,
    /// Time to survey a system after arrival.
    pub survey_years: f64,
    /// Time to bootstrap an autofactory in a system of richness 1.0.
    pub factory_build_years: f64,
    /// Interval between replicas from one colony at richness 1.0.
    pub replication_years: f64,
    /// Probes a richness-1.0, fabrication-1.0 colony can build before its
    /// accessible material runs out.
    pub launches_per_colony: f64,
    /// Systems below this richness are not worth colonizing; the probe
    /// refuels and moves on.
    pub min_richness: f64,
    /// Probability of losing a probe per light-year of transit, for a
    /// reliability-1.0 spec.
    pub loss_per_ly: f64,
    /// Maximum single-hop range (delta-v / propellant constraint).
    pub max_hop_ly: f64,
    /// How many cell rings outward a colony searches for targets.
    pub search_rings: i32,
    /// Std-dev-ish magnitude of per-generation spec mutation.
    pub drift: f64,
    /// Expansion doctrine for target selection.
    pub policy: TargetPolicy,
    /// If true, stop colonizing a Watcher's space once they issue a formal
    /// warning; if false, push on until they start shooting.
    pub respect_warnings: bool,
    /// Axis to engineer into replicas, at a cost in material per probe.
    pub invest: Option<SpecAxis>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            cell_size_ly: 16.0,
            cruise_speed_c: 0.10,
            survey_years: 0.5,
            factory_build_years: 4.0,
            replication_years: 3.0,
            launches_per_colony: 8.0,
            min_richness: 0.35,
            loss_per_ly: 0.0015,
            max_hop_ly: 25.0,
            search_rings: 12,
            drift: 0.03,
            policy: TargetPolicy::Nearest,
            respect_warnings: true,
            invest: None,
        }
    }
}
