//! The simulation: sparse mutable state + the event pump.
//!
//! `run_until` pops events in time order and handles each one; between
//! events, no work happens at all. Every decision a probe or colony makes
//! is local and autonomous — the probe IS the AI, which is both the fiction
//! (a von Neumann probe must operate beyond command lag) and the engine's
//! scaling model.

use crate::events::{Event, EventQueue};
use crate::galaxy::{Galaxy, Star, StarId};
use crate::probe::{Probe, ProbeId, ProbeSpec, ProbeState};
use crate::report::{Report, ReportKind};
use crate::rng::SplitMix64;
use crate::time::SimTime;
use crate::SimConfig;
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
    pub reports: Vec<Report>,
    pub stats: SimStats,
}

impl Simulation {
    /// Bootstrap: Sol hosts the generation-0 seed probe with a pre-built
    /// factory (the Earth-launched mission succeeded before play begins).
    pub fn new(cfg: SimConfig) -> Self {
        let galaxy = Galaxy::new(cfg.seed, cfg.cell_size_ly);
        let rng = SplitMix64::new(cfg.seed).fork(0x5157);
        let mut sim = Self {
            galaxy,
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

    fn on_survey_complete(&mut self, probe_id: ProbeId, star_id: StarId) {
        let star = self.galaxy.star(star_id);
        if star.richness >= self.cfg.min_richness {
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
        let (claimed, probes) = (&self.claimed, &self.probes);
        let rejected: &[StarId] = probes
            .get(&probe_id)
            .map(|p| p.rejected.as_slice())
            .unwrap_or(&[]);
        let target = self.galaxy.nearest_star(
            &from,
            self.cfg.max_hop_ly,
            self.cfg.search_rings,
            |s| !claimed.contains(&s.id) && !rejected.contains(&s.id),
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

    pub fn max_generation(&self) -> u32 {
        self.probes.values().map(|p| p.generation).max().unwrap_or(0)
    }

    pub fn probes_in_transit(&self) -> usize {
        self.probes
            .values()
            .filter(|p| matches!(p.state, ProbeState::InTransit { .. }))
            .count()
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
        acc
    }
}

fn colony_probes(colonies: &BTreeMap<StarId, Colony>, star: StarId) -> u32 {
    colonies.get(&star).map(|c| c.probes_built).unwrap_or(0)
}
