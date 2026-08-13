use vn_engine::galaxy::{Galaxy, StarId};
use vn_engine::sim::Simulation;
use vn_engine::time::SimTime;
use vn_engine::SimConfig;

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
fn running_in_chunks_matches_one_shot() {
    let mut a = Simulation::new(SimConfig::default());
    a.run_until(SimTime::from_years(400.0));

    let mut b = Simulation::new(SimConfig::default());
    for i in 1..=40 {
        b.run_until(SimTime::from_years(i as f64 * 10.0));
    }
    assert_eq!(a.digest(), b.digest());
}
