//! Light-lag reporting. The simulation is ground truth, but the *player's*
//! knowledge lives at Sol: every event emits a report that propagates home
//! at c. A colony founded 60 ly out is 60-year-old news by the time you
//! read it. Frontends should present `received_at`-ordered reports and
//! never leak ground truth ahead of light.

use crate::time::SimTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportKind {
    ColonyFounded,
    SystemRejected,
    ProbeLaunched,
    ProbeLost,
    SaturationReached,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub kind: ReportKind,
    /// When the event actually happened (ground truth).
    pub occurred_at: SimTime,
    /// When the signal reaches Sol: occurred_at + distance / c.
    pub received_at: SimTime,
    /// Distance from Sol of the originating system, in light-years.
    pub distance_ly: f64,
    /// Human-readable summary.
    pub text: String,
}
