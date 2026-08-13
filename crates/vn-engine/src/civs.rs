//! Advanced civilizations — procedural, lazy, and deterministic.
//!
//! Like stars, civs are pure functions of (seed, region): they cost nothing
//! until a probe enters their space. They are *not* simulated agents — a
//! civ's territory is a closed-form function of time (expansionists grow
//! linearly), and its reactions are events scheduled when you provoke it,
//! propagating at sublight interceptor speeds. Same physics as you.
//!
//! Region grid is 10× the star-cell grid (~160 ly). Civs near Sol are
//! suppressed so the early game is yours alone; the deep galaxy is not.

use crate::rng::{hash_n, unit_f64};
use serde::{Deserialize, Serialize};

/// How many star-cells per civ-region edge.
pub const REGION_CELLS: i32 = 10;
/// No civs spawn with a homeworld closer to Sol than this.
pub const SOL_EXCLUSION_LY: f64 = 120.0;
/// Fraction of regions hosting a civilization.
const CIV_DENSITY: f64 = 0.18;

const SALT_CIV: u64 = 0xA11E;

pub type CivKey = (i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    /// Long dead. Their ruins hold salvageable technology.
    Extinct,
    /// Ancient and patient. Tolerates trespass — to a point.
    Watcher,
    /// Defends a fixed border with overwhelming force.
    Territorial,
    /// A rival replicator wave, growing outward at a steady rate.
    Expansionist,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Civilization {
    pub key: CivKey,
    pub disposition: Disposition,
    /// Homeworld position, light-years.
    pub x: f64,
    pub y: f64,
    /// Territory radius at t = 0.
    pub radius0_ly: f64,
    /// Radial growth in ly/year (nonzero only for Expansionists).
    pub growth_ly_per_year: f64,
    /// Interceptor cruise speed as fraction of c (their response lag).
    pub response_speed_c: f64,
}

impl Civilization {
    pub fn radius_at(&self, years: f64) -> f64 {
        self.radius0_ly + self.growth_ly_per_year * years
    }

    pub fn contains(&self, x: f64, y: f64, years: f64) -> bool {
        let (dx, dy) = (x - self.x, y - self.y);
        let r = self.radius_at(years);
        dx * dx + dy * dy <= r * r
    }

    pub fn dist_to(&self, x: f64, y: f64) -> f64 {
        let (dx, dy) = (x - self.x, y - self.y);
        (dx * dx + dy * dy).sqrt()
    }

    /// Procedural name, deterministic per civ.
    pub fn name(&self) -> String {
        const SYL: [&str; 16] = [
            "ka", "thar", "xi", "ur", "vel", "dra", "om", "ish", "tau", "ren", "qol", "az",
            "myr", "esk", "no", "vau",
        ];
        let h = hash_n(&[self.key.0 as u32 as u64, self.key.1 as u32 as u64, SALT_CIV, 7]);
        let a = SYL[(h % 16) as usize];
        let b = SYL[((h >> 8) % 16) as usize];
        let c = SYL[((h >> 16) % 16) as usize];
        let stem = format!("{}{}{}", a, b, c);
        let mut chars = stem.chars();
        let cap: String = chars.next().unwrap().to_uppercase().chain(chars).collect();
        match self.disposition {
            Disposition::Extinct => format!("the ruins of the {}", cap),
            Disposition::Watcher => format!("the {} Watchers", cap),
            Disposition::Territorial => format!("the {} Dominion", cap),
            Disposition::Expansionist => format!("the {} Swarm", cap),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CivField {
    seed: u64,
    pub region_size_ly: f64,
}

impl CivField {
    pub fn new(seed: u64, cell_size_ly: f64) -> Self {
        Self { seed, region_size_ly: cell_size_ly * REGION_CELLS as f64 }
    }

    pub fn region_of(&self, x: f64, y: f64) -> CivKey {
        (
            (x / self.region_size_ly).floor() as i32,
            (y / self.region_size_ly).floor() as i32,
        )
    }

    /// The civilization seated in a region, if any.
    pub fn civ_in_region(&self, rx: i32, ry: i32) -> Option<Civilization> {
        let h = hash_n(&[self.seed, rx as u32 as u64, ry as u32 as u64, SALT_CIV]);
        if unit_f64(h) >= CIV_DENSITY {
            return None;
        }
        let hx = hash_n(&[h, 1]);
        let hy = hash_n(&[h, 2]);
        let x = (rx as f64 + 0.15 + 0.7 * unit_f64(hx)) * self.region_size_ly;
        let y = (ry as f64 + 0.15 + 0.7 * unit_f64(hy)) * self.region_size_ly;
        if (x * x + y * y).sqrt() < SOL_EXCLUSION_LY {
            return None;
        }
        let hd = hash_n(&[h, 3]);
        let roll = unit_f64(hd);
        let disposition = if roll < 0.35 {
            Disposition::Extinct
        } else if roll < 0.60 {
            Disposition::Watcher
        } else if roll < 0.85 {
            Disposition::Territorial
        } else {
            Disposition::Expansionist
        };
        let radius0_ly = 18.0 + 30.0 * unit_f64(hash_n(&[h, 4]));
        let growth_ly_per_year = match disposition {
            Disposition::Expansionist => 0.008 + 0.012 * unit_f64(hash_n(&[h, 5])),
            _ => 0.0,
        };
        Some(Civilization {
            key: (rx, ry),
            disposition,
            x,
            y,
            radius0_ly,
            growth_ly_per_year,
            response_speed_c: 0.30,
        })
    }

    pub fn civ_by_key(&self, key: CivKey) -> Option<Civilization> {
        self.civ_in_region(key.0, key.1)
    }

    /// Largest territory radius any civ can have at time `years` — bounds
    /// how far a region scan must reach.
    fn max_radius_at(years: f64) -> f64 {
        48.0 + 0.02 * years
    }

    /// All civs whose territory (at time `years`) could intersect a disc of
    /// `range_ly` around (x, y). One region scan; callers test candidates
    /// against the returned (small, deterministic-ordered) list.
    pub fn civs_near(&self, x: f64, y: f64, range_ly: f64, years: f64) -> Vec<Civilization> {
        let (rx, ry) = self.region_of(x, y);
        let reach = range_ly + Self::max_radius_at(years);
        let scan = 1 + (reach / self.region_size_ly).ceil() as i32;
        let mut out = Vec::new();
        for dy in -scan..=scan {
            for dx in -scan..=scan {
                if let Some(civ) = self.civ_in_region(rx + dx, ry + dy) {
                    if civ.dist_to(x, y) <= range_ly + civ.radius_at(years) {
                        out.push(civ);
                    }
                }
            }
        }
        out
    }

    /// Whose territory (if anyone's) contains this point at time `years`?
    pub fn territory_at(&self, x: f64, y: f64, years: f64) -> Option<Civilization> {
        let mut best: Option<Civilization> = None;
        for civ in self.civs_near(x, y, 0.0, years) {
            if civ.contains(x, y, years) {
                let better = match &best {
                    None => true,
                    // Deterministic tie-break: closest homeworld, then key.
                    Some(b) => {
                        let (da, db) = (civ.dist_to(x, y), b.dist_to(x, y));
                        da < db || (da == db && civ.key < b.key)
                    }
                };
                if better {
                    best = Some(civ);
                }
            }
        }
        best
    }
}
