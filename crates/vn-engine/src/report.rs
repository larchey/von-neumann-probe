//! Light-lag reporting. The simulation is ground truth, but the *player's*
//! knowledge lives at Sol: every event emits a report that propagates home
//! at c. A colony founded 60 ly out is 60-year-old news by the time you
//! read it. Frontends should present `received_at`-ordered reports and
//! never leak ground truth ahead of light.

use crate::civs::CivKey;
use crate::time::SimTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportKind {
    ColonyFounded,
    SystemRejected,
    ProbeLaunched,
    ProbeLost,
    SaturationReached,
    /// First physical evidence of another civilization.
    FirstContact,
    /// Salvaged technology from a dead civilization's ruins.
    XenoSalvage,
    /// A living civilization warns us off its space.
    CivWarning,
    /// A probe destroyed by a civilization's defenses.
    ProbeKilled,
    /// A colony destroyed by a civilization.
    ColonyLost,
    /// A new standing order broadcast from Sol.
    DoctrineChange,
    /// A descendant line has drifted far enough to name itself.
    LineageFork,
    /// A living world — the reason the probes were built.
    GardenWorld,
    /// A derelict or precursor cache found and salvaged.
    AnomalyFound,
    /// A probe destroyed by a natural hazard.
    HazardLoss,
    /// A descendant line has stopped taking orders from Sol.
    Secession,
    /// A dead civilization's archives, recording how they ended.
    Archive,
    /// A living civilization has answered.
    Transmission,
}

impl ReportKind {
    /// Is this worth interrupting the player for? Routine expansion
    /// traffic is not; contact, loss, discovery, and defection are.
    pub fn is_significant(self) -> bool {
        matches!(
            self,
            ReportKind::FirstContact
                | ReportKind::GardenWorld
                | ReportKind::Secession
                | ReportKind::ColonyLost
                | ReportKind::CivWarning
                | ReportKind::ProbeKilled
                | ReportKind::Archive
                | ReportKind::Transmission
        )
    }
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
    /// Where the event happened — this is how the player's knowledge map
    /// is built purely from received signals.
    pub x: f64,
    pub y: f64,
    /// The civilization involved, when the event concerns one.
    pub civ: Option<CivKey>,
    /// Human-readable summary.
    pub text: String,
}
