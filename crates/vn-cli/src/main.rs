//! vnp — headless runner for the von Neumann probe engine.
//!
//! Usage: vnp [--seed N] [--years N] [--step N] [--reports N]
//!
//! Runs the simulation and prints a decade-by-decade expansion table plus
//! the tail of mission control's message log. The log honors light lag:
//! you only see what a signal could physically have delivered to Sol.

use vn_engine::sim::Simulation;
use vn_engine::time::SimTime;
use vn_engine::SimConfig;

struct Args {
    seed: u64,
    years: f64,
    step: f64,
    reports: usize,
}

fn parse_args() -> Args {
    let mut args = Args { seed: 42, years: 500.0, step: 25.0, reports: 20 };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut grab = || it.next().unwrap_or_default();
        match flag.as_str() {
            "--seed" => args.seed = grab().parse().unwrap_or(args.seed),
            "--years" => args.years = grab().parse().unwrap_or(args.years),
            "--step" => args.step = grab().parse().unwrap_or(args.step),
            "--reports" => args.reports = grab().parse().unwrap_or(args.reports),
            "--help" | "-h" => {
                println!("vnp [--seed N] [--years N] [--step N] [--reports N]");
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
    let cfg = SimConfig { seed: args.seed, ..SimConfig::default() };
    println!("von Neumann probe expansion — seed {}, {} years", cfg.seed, args.years);
    println!(
        "cruise {:.2}c | replication {:.1} yr | max hop {:.0} ly | drift ±{:.0}%\n",
        cfg.cruise_speed_c,
        cfg.replication_years,
        cfg.max_hop_ly,
        cfg.drift * 100.0
    );

    let mut sim = Simulation::new(cfg);

    println!(
        "{:>6}  {:>7}  {:>8}  {:>9}  {:>9}  {:>7}  {:>8}",
        "year", "probes", "transit", "colonies", "frontier", "max gen", "lost"
    );
    let mut year = 0.0;
    while year < args.years {
        year = (year + args.step).min(args.years);
        sim.run_until(SimTime::from_years(year));
        println!(
            "{:>6.0}  {:>7}  {:>8}  {:>9}  {:>7.1}ly  {:>7}  {:>8}",
            year,
            sim.probes.len(),
            sim.probes_in_transit(),
            sim.colonies.len(),
            sim.frontier_radius_ly(),
            sim.max_generation(),
            sim.stats.probes_lost,
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
    println!(
        "\n{} events handled | {} probes built | {} lost | digest {:016x}",
        sim.stats.events_handled, sim.stats.probes_built, sim.stats.probes_lost, sim.digest()
    );
}
