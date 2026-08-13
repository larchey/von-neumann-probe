//! Simulation time. Integer seconds since epoch — integers keep event
//! ordering and serialization exact, and u64 seconds covers ~584 billion
//! years, comfortably past the stelliferous era.

use serde::{Deserialize, Serialize};
use std::fmt;

pub const SECONDS_PER_YEAR: u64 = 31_557_600; // Julian year
pub const SECONDS_PER_DAY: u64 = 86_400;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct SimTime(pub u64);

impl SimTime {
    pub const ZERO: SimTime = SimTime(0);

    pub fn from_years(years: f64) -> Self {
        SimTime((years * SECONDS_PER_YEAR as f64) as u64)
    }

    pub fn as_years(self) -> f64 {
        self.0 as f64 / SECONDS_PER_YEAR as f64
    }

    /// Saturating add of a duration expressed in years.
    pub fn plus_years(self, years: f64) -> Self {
        SimTime(self.0.saturating_add((years * SECONDS_PER_YEAR as f64) as u64))
    }
}

impl fmt::Display for SimTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Y{:.1}", self.as_years())
    }
}
