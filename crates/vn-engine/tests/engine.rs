use vn_engine::civs::{CivField, SOL_EXCLUSION_LY};
use vn_engine::galaxy::{Galaxy, StarId};
use vn_engine::sim::Simulation;
use vn_engine::time::SimTime;
use vn_engine::{SimConfig, TargetPolicy};

#[test]
fn galaxy_generation_is_stable() {
    let g = Galaxy::new(7, 16.0);
    let a = g.star(StarId { cx: 3, cy: -2, idx: 0 });
    let b = g.star(StarId { cx: 3, cy: -2, idx: 0 });
    assert_eq!(a.x.to_bits(), b.x.to_bits());
    assert_eq!(a.richness.to_bits(), b.richness.to_bits());
    assert_eq!(g.name(a.id), g.name(b.id));

    let sol = g.star(StarId::SOL);
    assert_eq!(g.name(sol.id), "Sol");
    assert_eq!(sol.x, 0.0);
}

#[test]
fn simulation_is_deterministic() {
    let run = || {
        let mut sim = Simulation::new(SimConfig::default());
        sim.run_until(SimTime::from_years(400.0));
        sim.digest()
    };
    assert_eq!(run(), run());
}

#[test]
fn different_seeds_diverge() {
    let run = |seed| {
        let mut sim = Simulation::new(SimConfig { seed, ..SimConfig::default() });
        sim.run_until(SimTime::from_years(300.0));
        sim.digest()
    };
    assert_ne!(run(1), run(2));
}

#[test]
fn expansion_actually_happens() {
    let mut sim = Simulation::new(SimConfig::default());
    sim.run_until(SimTime::from_years(500.0));
    assert!(sim.colonies.len() >= 5, "expected ≥5 colonies, got {}", sim.colonies.len());
    assert!(sim.probes.len() > sim.colonies.len());
    assert!(sim.frontier_radius_ly() > 10.0);
    assert!(sim.max_generation() >= 2);
}

#[test]
fn light_lag_is_respected() {
    let mut sim = Simulation::new(SimConfig::default());
    sim.run_until(SimTime::from_years(300.0));
    for r in &sim.reports {
        assert!(r.received_at >= r.occurred_at);
        let lag_years = r.received_at.as_years() - r.occurred_at.as_years();
        assert!((lag_years - r.distance_ly).abs() < 0.01, "lag must equal distance/c");
    }
    // And the filtered view never leaks unreceived signals.
    let now = SimTime::from_years(150.0);
    for r in sim.reports_received_by(now) {
        assert!(r.received_at <= now);
    }
}

#[test]
fn policies_change_history() {
    let run = |policy| {
        let mut sim = Simulation::new(SimConfig { policy, ..SimConfig::default() });
        sim.run_until(SimTime::from_years(600.0));
        (sim.digest(), sim.frontier_radius_ly(), sim.colonies.len())
    };
    let (dn, _fn_, cn) = run(TargetPolicy::Nearest);
    let (dr, _fr, cr) = run(TargetPolicy::Richest);
    let (do_, fo, co) = run(TargetPolicy::Outward);
    assert_ne!(dn, dr);
    assert_ne!(dn, do_);
    assert_ne!(dr, do_);
    // Every doctrine must still actually expand.
    assert!(cn >= 5 && cr >= 5 && co >= 5);
    assert!(fo > 10.0);
}

#[test]
fn civs_are_deterministic_and_respect_sol_exclusion() {
    let f = CivField::new(42, 16.0);
    let mut found = 0;
    for rx in -12..=12 {
        for ry in -12..=12 {
            if let Some(civ) = f.civ_in_region(rx, ry) {
                found += 1;
                let again = f.civ_in_region(rx, ry).unwrap();
                assert_eq!(civ.x.to_bits(), again.x.to_bits());
                assert_eq!(civ.disposition, again.disposition);
                let dist = (civ.x * civ.x + civ.y * civ.y).sqrt();
                assert!(dist >= SOL_EXCLUSION_LY, "civ too close to Sol: {dist:.0} ly");
                // Growing borders must be monotonic in time.
                assert!(civ.radius_at(1000.0) >= civ.radius_at(0.0));
            }
        }
    }
    assert!(found > 10, "expected a populated galaxy, found {found} civs in 625 regions");
}

#[test]
fn first_contact_eventually_happens() {
    // Expansion reaches well past the exclusion zone by year 4000; with
    // ~18% of regions inhabited we must have met someone by then.
    let mut sim = Simulation::new(SimConfig::default());
    sim.run_until(SimTime::from_years(4000.0));
    assert!(
        !sim.relations.is_empty(),
        "no civilizations met after 4000 years and {:.0} ly of frontier",
        sim.frontier_radius_ly()
    );
    // Contact must not predate physical reach of the exclusion zone:
    // nothing can be met before a probe could have flown there.
    for rel in sim.relations.values() {
        let earliest = SOL_EXCLUSION_LY / 0.5; // generous: 0.5c bound
        assert!(rel.met_at.as_years() > earliest * 0.2);
    }
}

#[test]
fn save_load_roundtrip_continues_bit_identically() {
    let mut a = Simulation::new(SimConfig::default());
    a.run_until(SimTime::from_years(300.0));

    let json = a.to_json();
    let mut b = Simulation::from_json(&json).expect("save should parse");
    assert_eq!(a.digest(), b.digest(), "loaded state must match saved state");

    // The true test: both timelines must stay identical *after* the load,
    // which requires RNG state and the event queue to round-trip exactly.
    a.run_until(SimTime::from_years(800.0));
    b.run_until(SimTime::from_years(800.0));
    assert_eq!(a.digest(), b.digest(), "post-load divergence — save is lossy");
}

#[test]
fn running_in_chunks_matches_one_shot() {
    let mut a = Simulation::new(SimConfig::default());
    a.run_until(SimTime::from_years(400.0));

    let mut b = Simulation::new(SimConfig::default());
    for i in 1..=40 {
        b.run_until(SimTime::from_years(i as f64 * 10.0));
    }
    assert_eq!(a.digest(), b.digest());
}
