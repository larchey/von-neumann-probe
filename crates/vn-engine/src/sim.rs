//! The simulation: sparse mutable state + the event pump.
//!
//! `run_until` pops events in time order and handles each one; between
//! events, no work happens at all. Every decision a probe or colony makes
//! is local and autonomous — the probe IS the AI, which is both the fiction
//! (a von Neumann probe must operate beyond command lag) and the engine's
//! scaling model.

use crate::civs::{CivField, CivKey, Disposition};
use crate::events::{Event, EventQueue};
use crate::galaxy::{Galaxy, Star, StarId};
use crate::probe::{Probe, ProbeId, ProbeSpec, ProbeState};
use crate::report::{Report, ReportKind};
use crate::rng::SplitMix64;
use crate::time::SimTime;
use crate::{SimConfig, TargetPolicy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Mutable state of a system gameplay has touched. Everything else about
/// the system (position, richness) is regenerated from the galaxy seed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Colony {
    pub star: StarId,
    pub founded_at: SimTime,
    pub founder: ProbeId,
    /// Replicas this colony may still launch before local accessible
    /// material is exhausted (richness-scaled).
    pub launches_remaining: u32,
    pub probes_built: u32,
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
            time: SimTime::ZERO,
            queue: EventQueue::default(),
            rng,
            next_probe_id: 0,
            probes: BTreeMap::new(),
            colonies: BTreeMap::new(),
            claimed: BTreeSet::new(),
            reports: Vec::new(),
            stats: SimStats::default(),
            cfg,
        };
        let seed_id = sim.alloc_probe_id();
        sim.probes.insert(
            seed_id,
            Probe {
                id: seed_id,
                generation: 0,
                spec: ProbeSpec::baseline(sim.cfg.cruise_speed_c),
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

    /// Doctrine-adjusted settlement bar: Richest doctrine refuses to
    /// settle mediocre systems at all — fewer colonies, each faster.
    fn effective_min_richness(&self) -> f64 {
        match self.cfg.policy {
            TargetPolicy::Richest => self.cfg.min_richness.max(0.7),
            _ => self.cfg.min_richness,
        }
    }

    fn on_survey_complete(&mut self, probe_id: ProbeId, star_id: StarId) {
        let star = self.galaxy.star(star_id);
        if self.civ_encounter(probe_id, &star) {
            return; // probe was destroyed or expelled; encounter handled it
        }
        if star.richness >= self.effective_min_richness() {
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
                    launches_remaining: 0,
                    probes_built: 0,
                },
            );
            self.queue
                .schedule(self.time.plus_years(build_years), Event::FactoryOnline { star: star_id });
        } else {
            // Barren system: refuel from volatiles and move on.
            self.stats.systems_rejected += 1;
            self.emit(&star, ReportKind::SystemRejected, format!(
                "{} surveyed: richness {:.2}, below viability. Moving on.",
                self.galaxy.name(star.id), star.richness
            ));
            if let Some(probe) = self.probes.get_mut(&probe_id) {
                probe.rejected.push(star_id);
            }
            self.claimed.remove(&star_id);
            self.launch_from(star_id, probe_id);
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
            self.emit(star, ReportKind::FirstContact, format!(
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
                    self.emit(star, ReportKind::XenoSalvage, format!(
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
                    self.emit(star, ReportKind::ProbeKilled, format!(
                        "Probe {} destroyed by pickets of {} at {}. No survivors of the encounter.",
                        probe_id.0, civ.name(), self.galaxy.name(star.id)
                    ));
                } else {
                    self.emit(star, ReportKind::CivWarning, format!(
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
                        self.emit(&star, ReportKind::CivWarning, format!(
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
        self.emit(&star, ReportKind::ColonyLost, format!(
            "Colony at {} destroyed by {}. {} probes were built there. The system falls silent.",
            self.galaxy.name(star_id), civ_name, colony.probes_built
        ));
    }

    /// Shared by bootstrap (Sol) and FactoryOnline: mark the colony live
    /// and start its replication line.
    fn found_colony(&mut self, star_id: StarId, founder: ProbeId, now: SimTime) {
        let star = self.galaxy.star(star_id);
        let launches = (self.cfg.launches_per_colony * star.richness).round().max(2.0) as u32;
        let colony = self.colonies.entry(star_id).or_insert(Colony {
            star: star_id,
            founded_at: now,
            founder,
            launches_remaining: 0,
            probes_built: 0,
        });
        colony.founded_at = now;
        colony.launches_remaining = launches;
        if let Some(probe) = self.probes.get_mut(&founder) {
            probe.state = ProbeState::Settled { star: star_id };
        }
        let fabrication = self.probes.get(&founder).map(|p| p.spec.fabrication).unwrap_or(1.0);
        let interval = self.cfg.replication_years / (star.richness * fabrication);
        self.emit(&star, ReportKind::ColonyFounded, format!(
            "Colony established at {} (richness {:.2}). Autofactory online; {} replicas budgeted.",
            self.galaxy.name(star.id), star.richness, launches
        ));
        self.queue
            .schedule(now.plus_years(interval), Event::ReplicaComplete { star: star_id });
    }

    fn on_replica_complete(&mut self, star_id: StarId) {
        let star = self.galaxy.star(star_id);
        let colony = match self.colonies.get_mut(&star_id) {
            Some(c) if c.launches_remaining > 0 => c,
            _ => return,
        };
        colony.launches_remaining -= 1;
        colony.probes_built += 1;
        let founder = colony.founder;
        let remaining = colony.launches_remaining;
        self.stats.probes_built += 1;

        // Child inherits the founder's spec with replication drift.
        let (parent_spec, parent_gen) = self
            .probes
            .get(&founder)
            .map(|p| (p.spec, p.generation))
            .unwrap_or((ProbeSpec::baseline(self.cfg.cruise_speed_c), 0));
        let mut mrng = self.rng.fork(crate::rng::hash_n(&[star_id.key(), self.stats.probes_built as u64]));
        let child_spec = parent_spec.mutate(&mut mrng, self.cfg.drift);
        let child_id = self.alloc_probe_id();
        self.probes.insert(
            child_id,
            Probe {
                id: child_id,
                generation: parent_gen + 1,
                spec: child_spec,
                state: ProbeState::Settled { star: star_id }, // placeholder until launch
                rejected: Vec::new(),
            },
        );
        self.launch_from(star_id, child_id);

        if remaining > 0 {
            let fabrication = child_spec.fabrication; // latest line off the fab
            let interval = self.cfg.replication_years / (star.richness * fabrication);
            self.queue
                .schedule(self.time.plus_years(interval), Event::ReplicaComplete { star: star_id });
        } else {
            self.emit(&star, ReportKind::SaturationReached, format!(
                "{} has exhausted accessible material; replication line shut down after {} probes.",
                self.galaxy.name(star.id), colony_probes(&self.colonies, star_id)
            ));
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
                        let threshold = if self.cfg.respect_warnings {
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
        let policy = self.cfg.policy;
        let min_rich = self.effective_min_richness();
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
            },
        );
        let Some(target) = target else {
            // Frontier dead end: stay dormant at the current system.
            if let Some(p) = self.probes.get_mut(&probe_id) {
                p.state = ProbeState::Settled { star: from_id };
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
        let p_loss = (self.cfg.loss_per_ly * dist / spec.reliability).clamp(0.0, 0.95);
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
        self.emit(&from, ReportKind::ProbeLaunched, format!(
            "Gen-{} probe departs {} for {} ({:.1} ly, ETA {:.1} yr at {:.2}c).",
            self.probes[&probe_id].generation, self.galaxy.name(from.id), self.galaxy.name(target.id),
            dist, travel_years, spec.cruise_speed_c
        ));
    }

    // ---- observation ----------------------------------------------------

    fn emit(&mut self, origin: &Star, kind: ReportKind, text: String) {
        let dist = (origin.x * origin.x + origin.y * origin.y).sqrt();
        self.reports.push(Report {
            kind,
            occurred_at: self.time,
            received_at: self.time.plus_years(dist), // signal travels at c
            distance_ly: dist,
            text,
        });
    }

    /// Reports that have physically reached Sol by `now`, in receive order.
    pub fn reports_received_by(&self, now: SimTime) -> Vec<&Report> {
        let mut out: Vec<&Report> =
            self.reports.iter().filter(|r| r.received_at <= now).collect();
        out.sort_by_key(|r| (r.received_at, r.occurred_at));
        out
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
        self.probes.values().map(|p| p.generation).max().unwrap_or(0)
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
            acc = hash_n(&[acc, id.key(), c.founded_at.0, c.launches_remaining as u64]);
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
