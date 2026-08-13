//! The simulation: sparse mutable state + the event pump.
//!
//! `run_until` pops events in time order and handles each one; between
//! events, no work happens at all. Every decision a probe or colony makes
//! is local and autonomous — the probe IS the AI, which is both the fiction
//! (a von Neumann probe must operate beyond command lag) and the engine's
//! scaling model.

use crate::civs::{CivField, CivKey, Disposition};
use crate::events::{Event, EventQueue};
use crate::galaxy::{Anomaly, Galaxy, Star, StarId};
use crate::lineage::{
    lineage_name, Lineage, LineageId, FORK_THRESHOLD, INDEPENDENCE_DRIFT, INDEPENDENCE_RANGE_LY,
};
use crate::probe::{Probe, ProbeId, ProbeSpec, ProbeState};
use crate::report::{Report, ReportKind};
use crate::rng::SplitMix64;
use crate::time::SimTime;
use crate::{
    SimConfig, SpecAxis, TargetPolicy, INVESTMENT_TIME_COST, MATERIAL_PER_ENGINEERED_PROBE,
    MATERIAL_PER_PROBE,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Mutable state of a system gameplay has touched. Everything else about
/// the system (position, richness) is regenerated from the galaxy seed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Colony {
    pub star: StarId,
    pub founded_at: SimTime,
    pub founder: ProbeId,
    /// Accessible material left, in abstract units (richness- and
    /// fabrication-scaled). A probe costs MATERIAL_PER_PROBE, an
    /// engineered one more — see the constants in lib.rs.
    pub material_remaining: u32,
    pub probes_built: u32,
    /// Probes powered down here forever (saturated founders, dead-end
    /// arrivals). Archived as a count — they are record, not simulation,
    /// which is what keeps the hot probe map frontier-sized. They die with
    /// the colony if a civ strikes it.
    pub dormant: u32,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SimStats {
    pub probes_built: u32,
    pub probes_lost: u32,
    pub systems_rejected: u32,
    pub events_handled: u64,
    /// Probes destroyed by civilizations' defenses.
    pub probes_killed: u32,
    /// Colonies destroyed by civilizations.
    pub colonies_lost: u32,
    /// Highest replication generation ever built (tracked here because
    /// dormant probes are archived out of the map).
    pub max_generation: u32,
    /// Probes powered down at systems that never became colonies
    /// (barren dead-ends); archived out of the hot map.
    pub drifters: u32,
    /// Living worlds found — the mission's actual scoreboard.
    pub garden_worlds: u32,
    /// Derelicts and precursor caches salvaged.
    pub anomalies_salvaged: u32,
    /// Probes lost to natural hazards at survey.
    pub hazard_losses: u32,
    /// Lines that have declared independence from Sol.
    pub independent_lines: u32,
    /// Replicas built under a directed-investment doctrine.
    pub directed_replicas: u32,
    /// Garden worlds found by lines that no longer report to Sol. They
    /// exist; you will never hear about them. This is what secession
    /// actually costs.
    pub garden_worlds_unreported: u32,
}

/// Our accumulated relationship with a civilization we've met.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CivRelation {
    pub met_at: SimTime,
    /// Watcher patience burned by our colonies in their space.
    pub irritation: u32,
    pub colonies_lost_to: u32,
}

/// Watcher civs issue a warning at this irritation level...
const WATCHER_WARN_AT: u32 = 3;
/// ...and begin destroying new colonies at this one.
pub const WATCHER_STRIKE_AT: u32 = 5;

/// A standing order broadcast from Sol. Doctrine is the only thing the
/// player controls — and it propagates at c, so a change made today
/// governs a colony 80 ly out only 80 years from now. The empire is a set
/// of nested light-cones of obedience.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doctrine {
    pub policy: TargetPolicy,
    pub respect_warnings: bool,
    /// Axis colonies spend extra build time engineering into their
    /// children, or None to replicate as fast as possible.
    pub invest: Option<SpecAxis>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Simulation {
    pub cfg: SimConfig,
    pub galaxy: Galaxy,
    pub time: SimTime,
    queue: EventQueue,
    rng: SplitMix64,
    next_probe_id: u64,
    /// BTreeMaps keep iteration deterministic for digests and UIs.
    pub probes: BTreeMap<ProbeId, Probe>,
    pub colonies: BTreeMap<StarId, Colony>,
    /// Stars already targeted or settled — prevents two colonies racing
    /// for the same system. Released if the inbound probe is lost.
    claimed: BTreeSet<StarId>,
    pub civ_field: CivField,
    /// Civilizations we've physically encountered.
    pub relations: BTreeMap<CivKey, CivRelation>,
    /// Doctrine broadcasts, in send order. Each entry governs a location
    /// only once its light-front has arrived there.
    doctrine_history: Vec<(SimTime, Doctrine)>,
    /// Named descendant families, in creation order.
    pub lineages: BTreeMap<LineageId, Lineage>,
    next_lineage_id: u32,
    pub reports: Vec<Report>,
    pub stats: SimStats,
}

impl Simulation {
    /// Bootstrap: Sol hosts the generation-0 seed probe with a pre-built
    /// factory (the Earth-launched mission succeeded before play begins).
    pub fn new(cfg: SimConfig) -> Self {
        let galaxy = Galaxy::new(cfg.seed, cfg.cell_size_ly);
        let civ_field = CivField::new(cfg.seed, cfg.cell_size_ly);
        let rng = SplitMix64::new(cfg.seed).fork(0x5157);
        let mut sim = Self {
            galaxy,
            civ_field,
            relations: BTreeMap::new(),
            doctrine_history: vec![(
                SimTime::ZERO,
                Doctrine {
                    policy: cfg.policy,
                    respect_warnings: cfg.respect_warnings,
                    invest: cfg.invest,
                },
            )],
            time: SimTime::ZERO,
            queue: EventQueue::default(),
            rng,
            next_probe_id: 0,
            probes: BTreeMap::new(),
            colonies: BTreeMap::new(),
            claimed: BTreeSet::new(),
            lineages: BTreeMap::new(),
            next_lineage_id: 0,
            reports: Vec::new(),
            stats: SimStats::default(),
            cfg,
        };
        let root_spec = ProbeSpec::baseline(sim.cfg.cruise_speed_c);
        let root_line = sim.new_lineage(None, root_spec, 0.0);
        let seed_id = sim.alloc_probe_id();
        sim.probes.insert(
            seed_id,
            Probe {
                id: seed_id,
                generation: 0,
                lineage: root_line,
                spec: root_spec,
                state: ProbeState::Settled { star: StarId::SOL },
                rejected: Vec::new(),
            },
        );
        sim.claimed.insert(StarId::SOL);
        sim.found_colony(StarId::SOL, seed_id, SimTime::ZERO);
        sim
    }

    fn alloc_probe_id(&mut self) -> ProbeId {
        let id = ProbeId(self.next_probe_id);
        self.next_probe_id += 1;
        id
    }

    fn new_lineage(
        &mut self,
        parent: Option<LineageId>,
        template: ProbeSpec,
        founded_at_ly: f64,
    ) -> LineageId {
        let id = LineageId(self.next_lineage_id);
        let name = lineage_name(self.next_lineage_id);
        self.next_lineage_id += 1;
        self.lineages.insert(
            id,
            Lineage {
                id,
                name,
                parent,
                founded_at: self.time,
                founded_at_ly,
                template,
                probes_built: 0,
                colonies_founded: 0,
                independent: false,
            },
        );
        id
    }

    /// Does this probe's line still send its telemetry home? Seceded lines
    /// go dark: they keep exploring and building, but Sol stops hearing
    /// about any of it, so their space becomes a hole in the known map.
    fn reports_home(&self, probe: ProbeId) -> bool {
        match self.probes.get(&probe).and_then(|p| self.lineages.get(&p.lineage)) {
            Some(l) => !l.independent,
            None => true,
        }
    }

    /// The doctrine actually governing a probe: Sol's orders if the line
    /// still answers to Sol, its own if it has seceded.
    fn governing_doctrine(&self, line: LineageId, x: f64, y: f64) -> Doctrine {
        match self.lineages.get(&line) {
            Some(l) if l.independent => Doctrine {
                policy: match l.own_policy_index() {
                    0 => TargetPolicy::Nearest,
                    1 => TargetPolicy::Richest,
                    _ => TargetPolicy::Outward,
                },
                // Nobody's warnings bind a line that answers to nobody.
                respect_warnings: false,
                // Seceded lines optimize for spread, not for your program.
                invest: None,
            },
            _ => self.doctrine_at(x, y),
        }
    }

    /// Advance until `until`, handling every event scheduled before it.
    /// Cost is proportional to events handled — zero if nothing happens.
    pub fn run_until(&mut self, until: SimTime) {
        while let Some(at) = self.queue.peek_time() {
            if at > until {
                break;
            }
            let (at, event) = self.queue.pop().unwrap();
            self.time = at;
            self.stats.events_handled += 1;
            self.handle(event);
        }
        self.time = until.max(self.time);
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::ProbeArrival { probe, star } => self.on_arrival(probe, star),
            Event::SurveyComplete { probe, star } => self.on_survey_complete(probe, star),
            Event::FactoryOnline { star } => self.on_factory_online(star),
            Event::ReplicaComplete { star } => self.on_replica_complete(star),
            Event::ProbeLost { probe, target } => self.on_probe_lost(probe, target),
            Event::CivStrike { star, civ } => self.on_civ_strike(star, civ),
        }
    }

    // ---- event handlers -------------------------------------------------

    fn on_arrival(&mut self, probe_id: ProbeId, star: StarId) {
        let survey_years = self.cfg.survey_years;
        if let Some(probe) = self.probes.get_mut(&probe_id) {
            probe.state = ProbeState::Surveying { star };
            self.queue.schedule(
                self.time.plus_years(survey_years),
                Event::SurveyComplete { probe: probe_id, star },
            );
        }
    }

    /// Broadcast a new standing order from Sol. It takes effect at each
    /// location only when its light-front arrives there.
    pub fn broadcast_doctrine(&mut self, doctrine: Doctrine) {
        self.doctrine_history.push((self.time, doctrine));
        let sol = self.galaxy.star(StarId::SOL);
        self.emit(&sol, ReportKind::DoctrineChange, format!(
            "Doctrine broadcast from Sol: {:?}{}{}. Propagating at c.",
            doctrine.policy,
            if doctrine.respect_warnings { "" } else { " (ignore warnings)" },
            match doctrine.invest {
                Some(a) => format!(", engineer {a:?}"),
                None => String::new(),
            }
        ));
    }

    /// The doctrine in force at a location: the newest broadcast whose
    /// light-front has reached it. Distant colonies obey old orders.
    pub fn doctrine_at(&self, x: f64, y: f64) -> Doctrine {
        let dist_ly = (x * x + y * y).sqrt();
        let mut current = self.doctrine_history[0].1;
        for (sent, d) in &self.doctrine_history {
            if sent.plus_years(dist_ly) <= self.time {
                current = *d;
            }
        }
        current
    }

    /// Doctrine-adjusted settlement bar: Richest doctrine refuses to
    /// settle mediocre systems at all — fewer colonies, each faster.
    fn effective_min_richness(&self, policy: TargetPolicy) -> f64 {
        match policy {
            TargetPolicy::Richest => self.cfg.min_richness.max(0.7),
            // Prospectors keep the normal bar: settling barren systems is
            // a trap, since a 0.25-richness colony spends 16 years building
            // a factory that yields two probes.
            TargetPolicy::Survey => self.cfg.min_richness,
            _ => self.cfg.min_richness,
        }
    }

    fn on_survey_complete(&mut self, probe_id: ProbeId, star_id: StarId) {
        let star = self.galaxy.star(star_id);
        if self.civ_encounter(probe_id, &star) {
            return; // probe was destroyed or expelled; encounter handled it
        }
        if self.survey_anomaly(probe_id, &star) {
            return; // hazard destroyed the probe
        }
        let local_policy = match self.probes.get(&probe_id).map(|p| p.lineage) {
            Some(l) => self.governing_doctrine(l, star.x, star.y).policy,
            None => self.doctrine_at(star.x, star.y).policy,
        };
        if star.richness >= self.effective_min_richness(local_policy) {
            // Worth settling: build the autofactory. Richer systems
            // bootstrap faster.
            let build_years = self.cfg.factory_build_years / star.richness;
            if let Some(probe) = self.probes.get_mut(&probe_id) {
                probe.state = ProbeState::Colonizing { star: star_id };
            }
            self.colonies.insert(
                star_id,
                Colony {
                    star: star_id,
                    founded_at: SimTime::ZERO, // set on FactoryOnline
                    founder: probe_id,
                    material_remaining: 0,
                    dormant: 0,
                    probes_built: 0,
                },
            );
            self.queue
                .schedule(self.time.plus_years(build_years), Event::FactoryOnline { star: star_id });
        } else {
            // Barren system: refuel from volatiles and move on.
            self.stats.systems_rejected += 1;
            if self.reports_home(probe_id) {
                self.emit(&star, ReportKind::SystemRejected, format!(
                    "{} surveyed: richness {:.2}, below viability. Moving on.",
                    self.galaxy.name(star.id), star.richness
                ));
            }
            if let Some(probe) = self.probes.get_mut(&probe_id) {
                probe.rejected.push(star_id);
            }
            self.claimed.remove(&star_id);
            self.launch_from(star_id, probe_id);
        }
    }

    /// Resolve whatever the survey turned up beyond the ore assay.
    /// Returns true if the probe did not survive it.
    fn survey_anomaly(&mut self, probe_id: ProbeId, star: &Star) -> bool {
        let Some(anomaly) = self.galaxy.anomaly(star.id) else {
            return false;
        };
        let name = self.galaxy.name(star.id);
        match anomaly {
            Anomaly::GardenWorld => {
                // A world found by a line that stopped reporting is a world
                // Sol never learns about.
                if !self.reports_home(probe_id) {
                    self.stats.garden_worlds_unreported += 1;
                    return false;
                }
                self.stats.garden_worlds += 1;
                let line = self
                    .probes
                    .get(&probe_id)
                    .and_then(|p| self.lineages.get(&p.lineage))
                    .map(|l| l.name.clone())
                    .unwrap_or_default();
                self.emit(star, ReportKind::GardenWorld, format!(
                    "GARDEN WORLD at {name}. Oxygen, liquid water, a biosphere. \
                     Found by the {line} line, {:.0} ly from Sol. This is what we were built for.",
                    (star.x * star.x + star.y * star.y).sqrt()
                ));
                false
            }
            Anomaly::Derelict | Anomaly::PrecursorCache => {
                self.stats.anomalies_salvaged += 1;
                let mut arng = self
                    .rng
                    .fork(crate::rng::hash_n(&[star.id.key(), probe_id.0, 0xA7]));
                let (gain, what) = match anomaly {
                    Anomaly::PrecursorCache => (1.0 + arng.range_f64(0.10, 0.25), "a precursor foundry, still running"),
                    _ => (1.0 + arng.range_f64(0.04, 0.12), "a derelict hulk"),
                };
                let which = arng.next_u64() % 3;
                let desc = if let Some(probe) = self.probes.get_mut(&probe_id) {
                    let s = &mut probe.spec;
                    match which {
                        0 => {
                            s.cruise_speed_c = (s.cruise_speed_c * gain).min(0.5);
                            "drive"
                        }
                        1 => {
                            s.fabrication = (s.fabrication * gain).min(4.0);
                            "fabrication"
                        }
                        _ => {
                            s.reliability = (s.reliability * gain).min(4.0);
                            "structural"
                        }
                    }
                } else {
                    return false;
                };
                self.emit(star, ReportKind::AnomalyFound, format!(
                    "Survey of {name} found {what}. Reverse-engineered {desc} gains \
                     (+{:.0}%) into the local template.",
                    (gain - 1.0) * 100.0
                ));
                false
            }
            Anomaly::Hazard => {
                // Reliability is exactly what buys survival here.
                let reliability = self
                    .probes
                    .get(&probe_id)
                    .map(|p| p.spec.reliability)
                    .unwrap_or(1.0);
                let p_death = (0.55 / reliability).clamp(0.05, 0.95);
                let mut hrng = self
                    .rng
                    .fork(crate::rng::hash_n(&[star.id.key(), probe_id.0, 0x4D]));
                if hrng.next_f64() < p_death {
                    self.stats.hazard_losses += 1;
                    self.claimed.remove(&star.id);
                    self.probes.remove(&probe_id);
                    self.emit(star, ReportKind::HazardLoss, format!(
                        "Probe {} lost at {name}: the system is a radiation trap. \
                         Telemetry ended mid-survey.",
                        probe_id.0
                    ));
                    true
                } else {
                    self.emit(star, ReportKind::AnomalyFound, format!(
                        "Probe {} survived the radiation environment at {name} \
                         (reliability {reliability:.2}). Hull degraded; survey continues.",
                        probe_id.0
                    ));
                    false
                }
            }
        }
    }

    fn on_factory_online(&mut self, star_id: StarId) {
        let founder = match self.colonies.get(&star_id) {
            Some(c) => c.founder,
            None => return,
        };
        self.found_colony(star_id, founder, self.time);
        self.civ_reaction_to_colony(star_id);
    }

    /// A probe has finished surveying a system inside (or salvageable from)
    /// another civilization's space. Returns true if the encounter ended
    /// the probe's business here (destroyed or expelled).
    fn civ_encounter(&mut self, probe_id: ProbeId, star: &Star) -> bool {
        let years = self.time.as_years();
        let Some(civ) = self.civ_field.territory_at(star.x, star.y, years) else {
            return false;
        };
        if !self.relations.contains_key(&civ.key) {
            self.relations.insert(
                civ.key,
                CivRelation { met_at: self.time, irritation: 0, colonies_lost_to: 0 },
            );
            self.emit_civ(star, ReportKind::FirstContact, Some(civ.key), format!(
                "FIRST CONTACT: probe {} has entered the space of {} at {}. \
                 Territory radius ~{:.0} ly.",
                probe_id.0, civ.name(), self.galaxy.name(star.id), civ.radius_at(years)
            ));
        }
        match civ.disposition {
            Disposition::Extinct => {
                // Dead worlds are safe — and their ruins improve us.
                let mut srng = self
                    .rng
                    .fork(crate::rng::hash_n(&[star.id.key(), probe_id.0, 0x5A]));
                let mut salvaged: Option<(&str, f64)> = None;
                if let Some(probe) = self.probes.get_mut(&probe_id) {
                    let gain = 1.0 + srng.range_f64(0.03, 0.10);
                    let which = srng.next_u64() % 3;
                    let s = &mut probe.spec;
                    let desc = match which {
                        0 => {
                            s.cruise_speed_c = (s.cruise_speed_c * gain).min(0.5);
                            "propulsion"
                        }
                        1 => {
                            s.fabrication = (s.fabrication * gain).min(4.0);
                            "fabrication"
                        }
                        _ => {
                            s.reliability = (s.reliability * gain).min(4.0);
                            "hull shielding"
                        }
                    };
                    salvaged = Some((desc, gain));
                }
                if let Some((desc, gain)) = salvaged {
                    self.emit_civ(star, ReportKind::XenoSalvage, Some(civ.key), format!(
                        "Ruins of {} catalogued at {}. Salvaged {} improvements (+{:.0}%) \
                         folded into the lineage template.",
                        civ.name(), self.galaxy.name(star.id), desc, (gain - 1.0) * 100.0
                    ));
                }
                false // proceed with normal colonize/reject flow
            }
            Disposition::Watcher => false, // tolerated... for now (see colonize)
            Disposition::Territorial | Disposition::Expansionist => {
                let kill_p = match civ.disposition {
                    Disposition::Territorial => 0.7,
                    _ => 0.3,
                };
                let mut erng = self
                    .rng
                    .fork(crate::rng::hash_n(&[star.id.key(), probe_id.0, 0xC1]));
                if erng.next_f64() < kill_p {
                    self.stats.probes_killed += 1;
                    self.claimed.remove(&star.id);
                    self.probes.remove(&probe_id);
                    self.emit_civ(star, ReportKind::ProbeKilled, Some(civ.key), format!(
                        "Probe {} destroyed by pickets of {} at {}. No survivors of the encounter.",
                        probe_id.0, civ.name(), self.galaxy.name(star.id)
                    ));
                } else {
                    self.emit_civ(star, ReportKind::CivWarning, Some(civ.key), format!(
                        "Probe {} expelled from {} by {}. Withdrawing.",
                        probe_id.0, self.galaxy.name(star.id), civ.name()
                    ));
                    if let Some(p) = self.probes.get_mut(&probe_id) {
                        p.rejected.push(star.id);
                    }
                    self.claimed.remove(&star.id);
                    self.launch_from(star.id, probe_id);
                }
                true
            }
        }
    }

    /// Living civilizations react to a new colony: Watchers lose patience;
    /// Expansionists will eventually overrun anything in their growth path.
    fn civ_reaction_to_colony(&mut self, star_id: StarId) {
        let star = self.galaxy.star(star_id);
        let years = self.time.as_years();
        if let Some(civ) = self.civ_field.territory_at(star.x, star.y, years) {
            if civ.disposition == Disposition::Watcher {
                if let Some(rel) = self.relations.get_mut(&civ.key) {
                    rel.irritation += 1;
                    let irritation = rel.irritation;
                    if irritation == WATCHER_WARN_AT {
                        self.emit_civ(&star, ReportKind::CivWarning, Some(civ.key), format!(
                            "{} have issued a formal warning: cease expansion into their space.",
                            civ.name()
                        ));
                    } else if irritation >= WATCHER_STRIKE_AT {
                        let eta_years = civ.dist_to(star.x, star.y) / civ.response_speed_c;
                        self.queue.schedule(
                            self.time.plus_years(eta_years),
                            Event::CivStrike { star: star_id, civ: civ.key },
                        );
                    }
                }
            }
        }
        // Any expansionist nearby will eventually swallow this system;
        // schedule the overrun for when its border arrives (closed-form).
        for civ in self.civ_field.civs_near(star.x, star.y, 400.0, years) {
            if civ.growth_ly_per_year <= 0.0 {
                continue;
            }
            let dist = civ.dist_to(star.x, star.y);
            let gap = dist - civ.radius_at(years);
            if gap > 0.0 {
                let arrival_years = years + gap / civ.growth_ly_per_year;
                self.queue.schedule(
                    SimTime::from_years(arrival_years),
                    Event::CivStrike { star: star_id, civ: civ.key },
                );
            }
        }
    }

    fn on_civ_strike(&mut self, star_id: StarId, civ_key: CivKey) {
        let Some(colony) = self.colonies.remove(&star_id) else {
            return; // already gone (e.g. double-scheduled overrun)
        };
        let star = self.galaxy.star(star_id);
        self.stats.colonies_lost += 1;
        if let Some(rel) = self.relations.get_mut(&civ_key) {
            rel.colonies_lost_to += 1;
        }
        // The settled founder dies with the colony; dormant descendants in
        // the system go silent (kept as record, no longer productive).
        if let Some(p) = self.probes.get(&colony.founder) {
            if p.state == (ProbeState::Settled { star: star_id }) {
                self.probes.remove(&colony.founder);
            }
        }
        let civ_name = self
            .civ_field
            .civ_by_key(civ_key)
            .map(|c| c.name())
            .unwrap_or_else(|| "an unknown power".to_string());
        self.emit_civ(&star, ReportKind::ColonyLost, Some(civ_key), format!(
            "Colony at {} destroyed by {}. {} probes were built there. The system falls silent.",
            self.galaxy.name(star_id), civ_name, colony.probes_built
        ));
    }

    /// Shared by bootstrap (Sol) and FactoryOnline: mark the colony live
    /// and start its replication line.
    fn found_colony(&mut self, star_id: StarId, founder: ProbeId, now: SimTime) {
        let star = self.galaxy.star(star_id);
        // Fabrication scales how many probes a colony gets out of the same
        // material, not just how fast it stamps them: replication interval
        // is ~3 years against transits of centuries, so build *rate* is
        // nearly irrelevant. Yield is what makes the stat worth having.
        let founder_fab = self.probes.get(&founder).map(|p| p.spec.fabrication).unwrap_or(1.0);
        let launches = (self.cfg.launches_per_colony * star.richness * founder_fab)
            .round()
            .max(2.0) as u32;
        let material = launches * MATERIAL_PER_PROBE;
        let colony = self.colonies.entry(star_id).or_insert(Colony {
            star: star_id,
            founded_at: now,
            founder,
            material_remaining: 0,
            dormant: 0,
            probes_built: 0,
        });
        colony.founded_at = now;
        colony.material_remaining = material;
        if let Some(probe) = self.probes.get_mut(&founder) {
            probe.state = ProbeState::Settled { star: star_id };
        }
        let fabrication = self.probes.get(&founder).map(|p| p.spec.fabrication).unwrap_or(1.0);
        let interval = self.cfg.replication_years / (star.richness * fabrication);

        // A founding probe that no longer matches the design its line was
        // founded on establishes a line of its own here — drift becomes a
        // named family exactly when it starts reproducing.
        let founder_info = self.probes.get(&founder).map(|p| (p.lineage, p.spec, p.generation));
        let mut line = founder_info.map(|(l, _, _)| l);
        if let Some((parent_line, spec, generation)) = founder_info {
            let diverged = self
                .lineages
                .get(&parent_line)
                .map(|l| l.divergence(&spec) >= FORK_THRESHOLD)
                .unwrap_or(false);
            if diverged {
                let dist = (star.x * star.x + star.y * star.y).sqrt();
                let new_line = self.new_lineage(Some(parent_line), spec, dist);
                if let Some(p) = self.probes.get_mut(&founder) {
                    p.lineage = new_line;
                }
                line = Some(new_line);

                // Far from the original design and far beyond any oversight:
                // the line stops being yours. It keeps expanding regardless.
                let root_drift = self.lineages[&LineageId(0)].divergence(&spec);
                let secedes = root_drift >= INDEPENDENCE_DRIFT
                    && dist >= INDEPENDENCE_RANGE_LY
                    && !self.lineages[&parent_line].independent;
                let inherits_independence = self.lineages[&parent_line].independent;
                if secedes || inherits_independence {
                    if let Some(l) = self.lineages.get_mut(&new_line) {
                        l.independent = true;
                    }
                }

                let (name, trait_word, parent_name) = {
                    let parent = &self.lineages[&parent_line];
                    let child = &self.lineages[&new_line];
                    (child.name.clone(), child.trait_of(parent), parent.name.clone())
                };
                if secedes {
                    self.stats.independent_lines += 1;
                    self.emit(&star, ReportKind::Secession, format!(
                        "The {name} line has stopped acknowledging directives from Sol. \
                         Founded at {} ({:.0} ly out), {:.0}% divergent from the original \
                         template, it continues to replicate on its own terms.",
                        self.galaxy.name(star.id), dist, root_drift * 100.0
                    ));
                } else {
                    self.emit(&star, ReportKind::LineageFork, format!(
                        "A gen-{generation} probe of the {parent_name} line has founded its own \
                         at {}: the {name} line, {trait_word} — {:.2}c, fab {:.2}, rel {:.2}.",
                        self.galaxy.name(star.id),
                        spec.cruise_speed_c,
                        spec.fabrication,
                        spec.reliability
                    ));
                }
            }
        }
        let line_name = line
            .and_then(|l| self.lineages.get(&l))
            .map(|l| l.name.clone())
            .unwrap_or_default();
        if let Some(l) = line.and_then(|l| self.lineages.get_mut(&l)) {
            l.colonies_founded += 1;
        }
        if self.reports_home(founder) {
            self.emit(&star, ReportKind::ColonyFounded, format!(
                "Colony established at {} (richness {:.2}) by the {} line. \
                 Autofactory online; {} replicas budgeted.",
                self.galaxy.name(star.id), star.richness, line_name, launches
            ));
        }
        self.queue
            .schedule(now.plus_years(interval), Event::ReplicaComplete { star: star_id });
    }

    fn on_replica_complete(&mut self, star_id: StarId) {
        let star = self.galaxy.star(star_id);
        let colony = match self.colonies.get_mut(&star_id) {
            Some(c) if c.material_remaining >= MATERIAL_PER_PROBE => c,
            _ => return,
        };
        let founder = colony.founder;
        colony.probes_built += 1;
        self.stats.probes_built += 1;

        // Child inherits the founder's spec with replication drift.
        let (parent_spec, parent_gen, parent_line) = self
            .probes
            .get(&founder)
            .map(|p| (p.spec, p.generation, p.lineage))
            .unwrap_or((
                ProbeSpec::baseline(self.cfg.cruise_speed_c),
                0,
                LineageId(0),
            ));
        let invest = self.governing_doctrine(parent_line, star.x, star.y).invest;
        let mut mrng = self.rng.fork(crate::rng::hash_n(&[star_id.key(), self.stats.probes_built as u64]));
        let child_spec = parent_spec.mutate(&mut mrng, self.cfg.drift, invest);

        // An engineered probe eats more of the colony's accessible
        // material than a mass-produced one.
        let cost = if invest.is_some() {
            self.stats.directed_replicas += 1;
            MATERIAL_PER_ENGINEERED_PROBE
        } else {
            MATERIAL_PER_PROBE
        };
        let remaining = match self.colonies.get_mut(&star_id) {
            Some(c) => {
                c.material_remaining = c.material_remaining.saturating_sub(cost);
                c.material_remaining
            }
            None => 0,
        };
        let child_id = self.alloc_probe_id();
        self.stats.max_generation = self.stats.max_generation.max(parent_gen + 1);

        // Children carry their founder's line; the split happens when one
        // of them settles somewhere and starts a factory of its own (see
        // found_colony), not per-unit off the assembly line.
        let child_line = parent_line;
        if let Some(l) = self.lineages.get_mut(&child_line) {
            l.probes_built += 1;
        }

        self.probes.insert(
            child_id,
            Probe {
                id: child_id,
                generation: parent_gen + 1,
                lineage: child_line,
                spec: child_spec,
                state: ProbeState::Settled { star: star_id }, // placeholder until launch
                rejected: Vec::new(),
            },
        );
        self.launch_from(star_id, child_id);

        if remaining >= MATERIAL_PER_PROBE {
            let fabrication = child_spec.fabrication; // latest line off the fab
            let mut interval = self.cfg.replication_years / (star.richness * fabrication);
            if invest.is_some() {
                interval *= INVESTMENT_TIME_COST; // engineered, not stamped out
            }
            self.queue
                .schedule(self.time.plus_years(interval), Event::ReplicaComplete { star: star_id });
        } else {
            self.emit(&star, ReportKind::SaturationReached, format!(
                "{} has exhausted accessible material; replication line shut down after {} probes.",
                self.galaxy.name(star.id), colony_probes(&self.colonies, star_id)
            ));
            // The founder's work is done; archive it out of the hot map.
            if self.probes.remove(&founder).is_some() {
                if let Some(c) = self.colonies.get_mut(&star_id) {
                    c.dormant += 1;
                }
            }
        }
    }

    fn on_probe_lost(&mut self, probe_id: ProbeId, target: StarId) {
        self.stats.probes_lost += 1;
        // Release the claim so another colony can retry the system.
        self.claimed.remove(&target);
        let target_star = self.galaxy.star(target);
        self.emit(&target_star, ReportKind::ProbeLost, format!(
            "Probe {} lost in transit to {}. Signal ceased.",
            probe_id.0, self.galaxy.name(target_star.id)
        ));
        self.probes.remove(&probe_id);
    }

    // ---- actions --------------------------------------------------------

    /// Pick the nearest unclaimed star in hop range and send `probe_id`
    /// there. If nothing is reachable, the probe goes dormant where it is.
    fn launch_from(&mut self, from_id: StarId, probe_id: ProbeId) {
        let from = self.galaxy.star(from_id);
        let years = self.time.as_years();
        // The launching colony obeys the doctrine whose light-front has
        // reached *it* — a fresh broadcast doesn't govern the frontier yet
        // — unless its line has seceded, in which case it obeys itself.
        let line = self.probes.get(&probe_id).map(|p| p.lineage);
        let doctrine = match line {
            Some(l) => self.governing_doctrine(l, from.x, from.y),
            None => self.doctrine_at(from.x, from.y),
        };
        // Route around space we've *learned* is hostile. Unknown civs can't
        // be avoided — first contact is paid for in probes.
        let hostile: Vec<_> = self
            .civ_field
            .civs_near(from.x, from.y, self.cfg.max_hop_ly, years)
            .into_iter()
            .filter(|c| match self.relations.get(&c.key) {
                None => false,
                Some(rel) => match c.disposition {
                    Disposition::Territorial | Disposition::Expansionist => true,
                    Disposition::Watcher => {
                        let threshold = if doctrine.respect_warnings {
                            WATCHER_WARN_AT
                        } else {
                            WATCHER_STRIKE_AT
                        };
                        rel.irritation >= threshold
                    }
                    Disposition::Extinct => false,
                },
            })
            .collect();
        let (claimed, probes) = (&self.claimed, &self.probes);
        let rejected: &[StarId] = probes
            .get(&probe_id)
            .map(|p| p.rejected.as_slice())
            .unwrap_or(&[]);
        let policy = doctrine.policy;
        let min_rich = self.effective_min_richness(policy);
        let from_radial = (from.x * from.x + from.y * from.y).sqrt();
        let target = self.galaxy.best_star(
            &from,
            self.cfg.max_hop_ly,
            self.cfg.search_rings,
            |s| {
                // Long-range spectroscopy screens out clearly-barren
                // systems, but its estimate carries error: borderline
                // systems still get visited and sometimes rejected on
                // arrival by the ground-truth survey.
                s.richness >= min_rich - 0.12
                    && !claimed.contains(&s.id)
                    && !rejected.contains(&s.id)
                    && !hostile.iter().any(|c| c.contains(s.x, s.y, years))
            },
            |s, d| match policy {
                // Score is "effective light-years"; lower is better.
                TargetPolicy::Nearest => d,
                // A rich system is worth a detour: ~15 ly per point of
                // spectroscopic richness estimate.
                TargetPolicy::Richest => d - 15.0 * s.richness,
                // Radial gain from Sol is the prize; a hop that flies
                // outward is nearly free.
                TargetPolicy::Outward => {
                    let radial = (s.x * s.x + s.y * s.y).sqrt();
                    d - 1.2 * (radial - from_radial)
                }
                // Aim at life-bearing odds: garden probability scales with
                // richness, so weight it harder than `Richest` does.
                TargetPolicy::Survey => d - 25.0 * s.richness,
            },
        );
        let Some(target) = target else {
            // Frontier dead end: power down here forever. Archived as a
            // count, not an entity — the hot map stays frontier-sized.
            if self.probes.remove(&probe_id).is_some() {
                match self.colonies.get_mut(&from_id) {
                    Some(c) => c.dormant += 1,
                    None => self.stats.drifters += 1,
                }
            }
            return;
        };

        let Some(probe) = self.probes.get_mut(&probe_id) else { return };
        let dist = from.distance_ly(&target);
        let travel_years = dist / probe.spec.cruise_speed_c;
        let arrives = self.time.plus_years(travel_years);
        probe.state = ProbeState::InTransit {
            from: from_id,
            to: target.id,
            departed: self.time,
            arrives,
        };
        let spec = probe.spec;
        self.claimed.insert(target.id);

        // Attrition roll, made now (deterministically) for the whole trip.
        //
        // Impact energy against the interstellar medium scales with v², so
        // a faster probe is a more fragile one. This is what stops "just
        // build faster drives" from being a free win: speed compounds
        // expansion, but it also compounds the odds of not arriving, and
        // only reliability buys that back.
        let speed_ratio = spec.cruise_speed_c / self.cfg.cruise_speed_c;
        let p_loss = (self.cfg.loss_per_ly * dist * speed_ratio * speed_ratio
            / spec.reliability)
            .clamp(0.0, 0.95);
        let mut trng = self
            .rng
            .fork(crate::rng::hash_n(&[probe_id.0, target.id.key(), 0x10]));
        if trng.next_f64() < p_loss {
            let frac = trng.range_f64(0.1, 0.95);
            let lost_at = self.time.plus_years(travel_years * frac);
            self.queue
                .schedule(lost_at, Event::ProbeLost { probe: probe_id, target: target.id });
        } else {
            self.queue
                .schedule(arrives, Event::ProbeArrival { probe: probe_id, star: target.id });
        }
        if self.reports_home(probe_id) {
            self.emit(&from, ReportKind::ProbeLaunched, format!(
                "Gen-{} probe departs {} for {} ({:.1} ly, ETA {:.1} yr at {:.2}c).",
                self.probes[&probe_id].generation, self.galaxy.name(from.id), self.galaxy.name(target.id),
                dist, travel_years, spec.cruise_speed_c
            ));
        }
    }

    // ---- observation ----------------------------------------------------

    fn emit(&mut self, origin: &Star, kind: ReportKind, text: String) {
        self.emit_civ(origin, kind, None, text);
    }

    fn emit_civ(&mut self, origin: &Star, kind: ReportKind, civ: Option<CivKey>, text: String) {
        let dist = (origin.x * origin.x + origin.y * origin.y).sqrt();
        self.reports.push(Report {
            kind,
            occurred_at: self.time,
            received_at: self.time.plus_years(dist), // signal travels at c
            distance_ly: dist,
            x: origin.x,
            y: origin.y,
            civ,
            text,
        });
        // Bound log memory over deep time: drop the oldest routine
        // traffic, keep every historically significant signal.
        if self.reports.len() >= 400_000 {
            let cutoff = self.reports.len() - 200_000;
            let old: Vec<Report> = self.reports.drain(..cutoff).collect();
            let mut kept: Vec<Report> = old
                .into_iter()
                .filter(|r| {
                    matches!(
                        r.kind,
                        ReportKind::FirstContact
                            | ReportKind::ColonyLost
                            | ReportKind::CivWarning
                            | ReportKind::XenoSalvage
                            | ReportKind::DoctrineChange
                    )
                })
                .collect();
            kept.append(&mut self.reports);
            self.reports = kept;
        }
    }

    /// Reports that have physically reached Sol by `now`, in receive order.
    pub fn reports_received_by(&self, now: SimTime) -> Vec<&Report> {
        let mut out: Vec<&Report> =
            self.reports.iter().filter(|r| r.received_at <= now).collect();
        out.sort_by_key(|r| (r.received_at, r.occurred_at));
        out
    }

    /// Systems physically visited and assayed — colonies plus the ones
    /// found wanting. The denominator of the mission's search.
    pub fn systems_surveyed(&self) -> u64 {
        self.colonies.len() as u64
            + self.stats.colonies_lost as u64
            + self.stats.systems_rejected as u64
    }

    /// Furthest colony from Sol, in light-years.
    pub fn frontier_radius_ly(&self) -> f64 {
        self.colonies
            .keys()
            .map(|id| {
                let s = self.galaxy.star(*id);
                (s.x * s.x + s.y * s.y).sqrt()
            })
            .fold(0.0, f64::max)
    }

    /// Mean richness across current colonies — the visible fingerprint of
    /// a Richest-doctrine run.
    pub fn mean_colony_richness(&self) -> f64 {
        if self.colonies.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .colonies
            .keys()
            .map(|id| self.galaxy.star(*id).richness)
            .sum();
        sum / self.colonies.len() as f64
    }

    pub fn max_generation(&self) -> u32 {
        self.stats.max_generation
    }

    /// Fraction of colonies founded by lines that still take orders from
    /// Sol. Falls as the frontier outgrows its own obedience.
    pub fn obedient_fraction(&self) -> f64 {
        let (mut loyal, mut total) = (0u64, 0u64);
        for l in self.lineages.values() {
            total += l.colonies_founded as u64;
            if !l.independent {
                loyal += l.colonies_founded as u64;
            }
        }
        if total == 0 {
            1.0
        } else {
            loyal as f64 / total as f64
        }
    }

    /// Total living probes: active (hot map) + dormant (archived counts).
    pub fn population(&self) -> u64 {
        self.probes.len() as u64
            + self.colonies.values().map(|c| c.dormant as u64).sum::<u64>()
            + self.stats.drifters as u64
    }

    pub fn probes_in_transit(&self) -> usize {
        self.probes
            .values()
            .filter(|p| matches!(p.state, ProbeState::InTransit { .. }))
            .count()
    }

    /// Stars currently claimed (settled or being flown to) — the fringe of
    /// the wave, for map rendering.
    pub fn claimed_stars(&self) -> impl Iterator<Item = &StarId> {
        self.claimed.iter()
    }

    /// Order-independent state fingerprint for determinism tests.
    pub fn digest(&self) -> u64 {
        use crate::rng::hash_n;
        let mut acc = hash_n(&[self.time.0, self.next_probe_id, self.queue.len() as u64]);
        for (id, p) in &self.probes {
            acc = hash_n(&[
                acc,
                id.0,
                p.generation as u64,
                p.spec.cruise_speed_c.to_bits(),
                p.spec.fabrication.to_bits(),
                p.spec.reliability.to_bits(),
            ]);
        }
        for (id, c) in &self.colonies {
            acc = hash_n(&[
                acc,
                id.key(),
                c.founded_at.0,
                c.material_remaining as u64,
                c.dormant as u64,
                c.probes_built as u64,
            ]);
        }
        for id in &self.claimed {
            acc = hash_n(&[acc, id.key()]);
        }
        for (key, rel) in &self.relations {
            acc = hash_n(&[
                acc,
                key.0 as u32 as u64,
                key.1 as u32 as u64,
                rel.met_at.0,
                rel.irritation as u64,
                rel.colonies_lost_to as u64,
            ]);
        }
        for (sent, d) in &self.doctrine_history {
            acc = hash_n(&[acc, sent.0, d.policy as u64, d.respect_warnings as u64]);
        }
        for (id, l) in &self.lineages {
            acc = hash_n(&[
                acc,
                id.0 as u64,
                l.founded_at.0,
                l.probes_built as u64,
                l.colonies_founded as u64,
                l.independent as u64,
            ]);
        }
        acc
    }
}

fn colony_probes(colonies: &BTreeMap<StarId, Colony>, star: StarId) -> u32 {
    colonies.get(&star).map(|c| c.probes_built).unwrap_or(0)
}

/// Serializable snapshot of a `Simulation`. Maps are flattened to vectors
/// because their keys are structs (JSON allows only string keys); every
/// other field round-trips exactly — RNG state, event queue, reports — so
/// a loaded game continues bit-identically (see the save/load test).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveGame {
    cfg: SimConfig,
    time: SimTime,
    queue: EventQueue,
    rng: SplitMix64,
    next_probe_id: u64,
    probes: Vec<Probe>,
    colonies: Vec<Colony>,
    claimed: Vec<StarId>,
    relations: Vec<(CivKey, CivRelation)>,
    doctrine_history: Vec<(SimTime, Doctrine)>,
    lineages: Vec<Lineage>,
    next_lineage_id: u32,
    reports: Vec<Report>,
    stats: SimStats,
}

impl Simulation {
    pub fn to_save(&self) -> SaveGame {
        SaveGame {
            cfg: self.cfg.clone(),
            time: self.time,
            queue: self.queue.clone(),
            rng: self.rng.clone(),
            next_probe_id: self.next_probe_id,
            probes: self.probes.values().cloned().collect(),
            colonies: self.colonies.values().cloned().collect(),
            claimed: self.claimed.iter().copied().collect(),
            relations: self.relations.iter().map(|(k, v)| (*k, *v)).collect(),
            doctrine_history: self.doctrine_history.clone(),
            lineages: self.lineages.values().cloned().collect(),
            next_lineage_id: self.next_lineage_id,
            reports: self.reports.clone(),
            stats: self.stats,
        }
    }

    pub fn from_save(save: SaveGame) -> Self {
        let galaxy = Galaxy::new(save.cfg.seed, save.cfg.cell_size_ly);
        let civ_field = CivField::new(save.cfg.seed, save.cfg.cell_size_ly);
        Self {
            galaxy,
            civ_field,
            time: save.time,
            queue: save.queue,
            rng: save.rng,
            next_probe_id: save.next_probe_id,
            probes: save.probes.into_iter().map(|p| (p.id, p)).collect(),
            colonies: save.colonies.into_iter().map(|c| (c.star, c)).collect(),
            claimed: save.claimed.into_iter().collect(),
            relations: save.relations.into_iter().collect(),
            doctrine_history: save.doctrine_history,
            lineages: save.lineages.into_iter().map(|l| (l.id, l)).collect(),
            next_lineage_id: save.next_lineage_id,
            reports: save.reports,
            stats: save.stats,
            cfg: save.cfg,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_save()).expect("save serialization cannot fail")
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        Ok(Self::from_save(serde_json::from_str(json)?))
    }
}
