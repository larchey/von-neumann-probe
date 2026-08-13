//! Procedural, lazy, infinite starfield.
//!
//! Space is divided into square cells of `cell_size_ly`. Each cell's star
//! count, positions, and properties are pure functions of (seed, cell
//! coords) — nothing is stored until gameplay touches a system, so the
//! galaxy is unbounded at zero cost. Sol is pinned near the origin.

use crate::rng::{hash_n, unit_f64};
use serde::{Deserialize, Serialize};

/// Stable identity of a star: its cell plus index within the cell.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct StarId {
    pub cx: i32,
    pub cy: i32,
    pub idx: u8,
}

impl StarId {
    pub const SOL: StarId = StarId { cx: 0, cy: 0, idx: 0 };

    /// Fold into a u64 for hashing / RNG stream derivation.
    pub fn key(self) -> u64 {
        ((self.cx as u32 as u64) << 40) ^ ((self.cy as u32 as u64) << 8) ^ self.idx as u64
    }
}

/// Star data is fully derived and cheap to regenerate; names are *not*
/// stored here — they're only needed for display, so `Galaxy::name`
/// produces them on demand and target-search stays allocation-free.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Star {
    pub id: StarId,
    /// Position in light-years.
    pub x: f64,
    pub y: f64,
    /// Resource richness scalar. ~0.2–1.5; drives factory build time,
    /// replication rate, and launch budget.
    pub richness: f64,
}

impl Star {
    pub fn distance_ly(&self, other: &Star) -> f64 {
        let (dx, dy) = (self.x - other.x, self.y - other.y);
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Galaxy {
    seed: u64,
    pub cell_size_ly: f64,
}

const SALT_COUNT: u64 = 0xC0;
const SALT_POS: u64 = 0xF0;
const SALT_RICH: u64 = 0x51;
const SALT_ANOM: u64 = 0xA0;

/// What a survey turns up beyond the ore assay. Rare, hashed per star, so
/// the galaxy holds its secrets until someone physically arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anomaly {
    /// A living world. The reason the probes were built.
    GardenWorld,
    /// A dead ship or station: salvage improves the finder's design.
    Derelict,
    /// Someone else's automated foundry, still running. Big fabrication win.
    PrecursorCache,
    /// Pulsar sweep, radiation belt, unstable binary. Kills probes.
    Hazard,
}

/// Per-star anomaly probabilities. Everything else is empty sky.
const P_GARDEN: f64 = 0.012;
const P_DERELICT: f64 = 0.020;
const P_CACHE: f64 = 0.008;
const P_HAZARD: f64 = 0.022;

impl Galaxy {
    pub fn new(seed: u64, cell_size_ly: f64) -> Self {
        Self { seed, cell_size_ly }
    }

    /// Cell containing a point.
    pub fn cell_of(&self, x: f64, y: f64) -> (i32, i32) {
        ((x / self.cell_size_ly).floor() as i32, (y / self.cell_size_ly).floor() as i32)
    }

    /// Number of stars in a cell: 1–3, mean 2.
    pub fn star_count(&self, cx: i32, cy: i32) -> u8 {
        let h = hash_n(&[self.seed, cx as u32 as u64, cy as u32 as u64, SALT_COUNT]);
        1 + (h % 3) as u8
    }

    /// Generate a star by id. Panics if `idx` is out of range for the cell.
    pub fn star(&self, id: StarId) -> Star {
        if id == StarId::SOL {
            return Star { id, x: 0.0, y: 0.0, richness: 1.0 };
        }
        debug_assert!(id.idx < self.star_count(id.cx, id.cy));
        let hp = hash_n(&[self.seed, id.key(), SALT_POS]);
        let hr = hash_n(&[self.seed, id.key(), SALT_RICH]);
        let jx = unit_f64(hp);
        let jy = unit_f64(hash_n(&[hp, 1]));
        let x = (id.cx as f64 + jx) * self.cell_size_ly;
        let y = (id.cy as f64 + jy) * self.cell_size_ly;
        // Skew toward poor systems: richness = 0.2 + 1.3 * u^1.5
        let u = unit_f64(hr);
        let richness = 0.2 + 1.3 * (u * u.sqrt());
        Star { id, x, y, richness }
    }

    /// What waits at this star, if anything. Garden worlds skew toward
    /// richer systems — the same conditions that make a system worth
    /// settling make it worth living on.
    pub fn anomaly(&self, id: StarId) -> Option<Anomaly> {
        if id == StarId::SOL {
            return None;
        }
        let h = hash_n(&[self.seed, id.key(), SALT_ANOM]);
        let roll = unit_f64(h);
        let garden = P_GARDEN * (0.4 + self.star(id).richness);
        if roll < garden {
            Some(Anomaly::GardenWorld)
        } else if roll < garden + P_DERELICT {
            Some(Anomaly::Derelict)
        } else if roll < garden + P_DERELICT + P_CACHE {
            Some(Anomaly::PrecursorCache)
        } else if roll < garden + P_DERELICT + P_CACHE + P_HAZARD {
            Some(Anomaly::Hazard)
        } else {
            None
        }
    }

    /// Display name for a star. Allocates; call only for UI/reports.
    pub fn name(&self, id: StarId) -> String {
        if id == StarId::SOL {
            return "Sol".to_string();
        }
        let hr = hash_n(&[self.seed, id.key(), SALT_RICH]);
        format!("HIP-{:05}", hash_n(&[hr, 2]) % 100_000)
    }

    /// All stars in one cell, in index order.
    pub fn stars_in_cell(&self, cx: i32, cy: i32) -> Vec<Star> {
        (0..self.star_count(cx, cy))
            .map(|idx| self.star(StarId { cx, cy, idx }))
            .collect()
    }

    /// Best-scoring star near `from` (excluding it) within `max_dist_ly`
    /// for which `accept` returns true; lower score wins. Every cell that
    /// could hold an in-range star is scanned (a star within d ly lies at
    /// most ceil(d/cell)+1 rings out), so arbitrary scoring functions are
    /// safe. Fully deterministic: fixed ring/index order, ties broken by
    /// StarId. Returns None if nothing acceptable is in range.
    pub fn best_star<F, S>(
        &self,
        from: &Star,
        max_dist_ly: f64,
        max_rings: i32,
        accept: F,
        score: S,
    ) -> Option<Star>
    where
        F: Fn(&Star) -> bool,
        S: Fn(&Star, f64) -> f64,
    {
        let (ocx, ocy) = self.cell_of(from.x, from.y);
        let rings = max_rings.min((max_dist_ly / self.cell_size_ly).ceil() as i32 + 1);
        let mut best: Option<(f64, Star)> = None;
        for r in 0..=rings {
            for (cx, cy) in ring_cells(ocx, ocy, r) {
                for star in self.stars_in_cell(cx, cy) {
                    if star.id == from.id {
                        continue;
                    }
                    let d = from.distance_ly(&star);
                    if d > max_dist_ly || !accept(&star) {
                        continue;
                    }
                    let s = score(&star, d);
                    let better = match &best {
                        None => true,
                        Some((bs, bstar)) => s < *bs || (s == *bs && star.id < bstar.id),
                    };
                    if better {
                        best = Some((s, star));
                    }
                }
            }
        }
        best.map(|(_, s)| s)
    }

    /// Nearest acceptable star — `best_star` scored by distance.
    pub fn nearest_star<F: Fn(&Star) -> bool>(
        &self,
        from: &Star,
        max_dist_ly: f64,
        max_rings: i32,
        accept: F,
    ) -> Option<Star> {
        self.best_star(from, max_dist_ly, max_rings, accept, |_, d| d)
    }
}

/// Cells at Chebyshev distance exactly `r` from (ocx, ocy), fixed order.
fn ring_cells(ocx: i32, ocy: i32, r: i32) -> Vec<(i32, i32)> {
    if r == 0 {
        return vec![(ocx, ocy)];
    }
    let mut cells = Vec::with_capacity((8 * r) as usize);
    for dx in -r..=r {
        cells.push((ocx + dx, ocy - r));
        cells.push((ocx + dx, ocy + r));
    }
    for dy in (-r + 1)..r {
        cells.push((ocx - r, ocy + dy));
        cells.push((ocx + r, ocy + dy));
    }
    cells
}
