//! vnp — headless runner for the von Neumann probe engine.
//!
//! Usage: vnp [--seed N] [--years N] [--step N] [--reports N]
//!
//! Runs the simulation and prints a decade-by-decade expansion table plus
//! the tail of mission control's message log. The log honors light lag:
//! you only see what a signal could physically have delivered to Sol.

use vn_engine::sim::Simulation;
use vn_engine::time::SimTime;
use vn_engine::{SimConfig, TargetPolicy};

struct Args {
    seed: u64,
    years: f64,
    step: f64,
    reports: usize,
    policy: TargetPolicy,
    bold: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        seed: 42,
        years: 500.0,
        step: 25.0,
        reports: 20,
        policy: TargetPolicy::Nearest,
        bold: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut grab = || it.next().unwrap_or_default();
        match flag.as_str() {
            "--seed" => args.seed = grab().parse().unwrap_or(args.seed),
            "--years" => args.years = grab().parse().unwrap_or(args.years),
            "--step" => args.step = grab().parse().unwrap_or(args.step),
            "--reports" => args.reports = grab().parse().unwrap_or(args.reports),
            "--policy" => {
                args.policy = match grab().as_str() {
                    "nearest" => TargetPolicy::Nearest,
                    "richest" => TargetPolicy::Richest,
                    "outward" => TargetPolicy::Outward,
                    other => {
                        eprintln!("unknown policy: {other} (nearest|richest|outward)");
                        std::process::exit(2);
                    }
                }
            }
            "--bold" => args.bold = true,
            "--help" | "-h" => {
                println!(
                    "vnp [--seed N] [--years N] [--step N] [--reports N] \
                     [--policy nearest|richest|outward] [--bold]"
                );
                println!("  --bold  ignore Watcher warnings (colonize until they shoot)");
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown flag: {other}");
                std::process::exit(2);
            }
        }
    }
    args
}

fn main() {
    let args = parse_args();
    let cfg = SimConfig {
        seed: args.seed,
        policy: args.policy,
        respect_warnings: !args.bold,
        ..SimConfig::default()
    };
    println!("von Neumann probe expansion — seed {}, {} years", cfg.seed, args.years);
    println!(
        "cruise {:.2}c | replication {:.1} yr | max hop {:.0} ly | drift ±{:.0}% | doctrine {:?}{}\n",
        cfg.cruise_speed_c,
        cfg.replication_years,
        cfg.max_hop_ly,
        cfg.drift * 100.0,
        cfg.policy,
        if cfg.respect_warnings { "" } else { " (bold)" }
    );

    let mut sim = Simulation::new(cfg);

    println!(
        "{:>6}  {:>7}  {:>8}  {:>9}  {:>9}  {:>7}  {:>6}  {:>6}  {:>7}",
        "year", "probes", "transit", "colonies", "frontier", "max gen", "lost", "killed", "civs"
    );
    let mut year = 0.0;
    while year < args.years {
        year = (year + args.step).min(args.years);
        sim.run_until(SimTime::from_years(year));
        println!(
            "{:>6.0}  {:>7}  {:>8}  {:>9}  {:>7.1}ly  {:>7}  {:>6}  {:>6}  {:>7}",
            year,
            sim.probes.len(),
            sim.probes_in_transit(),
            sim.colonies.len(),
            sim.frontier_radius_ly(),
            sim.max_generation(),
            sim.stats.probes_lost,
            sim.stats.probes_killed,
            sim.relations.len(),
        );
    }

    let now = SimTime::from_years(args.years);
    let received = sim.reports_received_by(now);
    println!(
        "\n─── mission control log (light-lagged; {} of {} signals received) ───",
        received.len(),
        sim.reports.len()
    );
    for r in received.iter().rev().take(args.reports).rev() {
        println!(
            "[recv Y{:>6.1} | sent Y{:>6.1} | {:>5.1} ly] {}",
            r.received_at.as_years(),
            r.occurred_at.as_years(),
            r.distance_ly,
            r.text
        );
    }
    if !sim.relations.is_empty() {
        println!("\n─── known civilizations ───");
        for (key, rel) in &sim.relations {
            if let Some(civ) = sim.civ_field.civ_by_key(*key) {
                let dist = (civ.x * civ.x + civ.y * civ.y).sqrt();
                println!(
                    "{:<32} {:>6.0} ly out | met Y{:<7.1} | irritation {} | colonies lost to them: {}",
                    civ.name(),
                    dist,
                    rel.met_at.as_years(),
                    rel.irritation,
                    rel.colonies_lost_to
                );
            }
        }
    }

    println!("\nmean colony richness: {:.3}", sim.mean_colony_richness());
    println!(
        "{} events | {} probes built | {} lost in transit | {} killed by civs | {} colonies destroyed | digest {:016x}",
        sim.stats.events_handled,
        sim.stats.probes_built,
        sim.stats.probes_lost,
        sim.stats.probes_killed,
        sim.stats.colonies_lost,
        sim.digest()
    );
}
