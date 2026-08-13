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
    save: Option<String>,
    load: Option<String>,
    map: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        seed: 42,
        years: 500.0,
        step: 25.0,
        reports: 20,
        policy: TargetPolicy::Nearest,
        bold: false,
        save: None,
        load: None,
        map: false,
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
            "--save" => args.save = Some(grab()),
            "--load" => args.load = Some(grab()),
            "--map" => args.map = true,
            "--help" | "-h" => {
                println!(
                    "vnp [--seed N] [--years N] [--step N] [--reports N] \
                     [--policy nearest|richest|outward] [--bold] [--save FILE] [--load FILE] [--map]"
                );
                println!("  --bold   ignore Watcher warnings (colonize until they shoot)");
                println!("  --years  with --load: additional years to simulate");
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
    let mut sim = match &args.load {
        Some(path) => {
            let json = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("cannot read save {path}: {e}");
                std::process::exit(1);
            });
            let sim = Simulation::from_json(&json).unwrap_or_else(|e| {
                eprintln!("corrupt save {path}: {e}");
                std::process::exit(1);
            });
            println!(
                "resumed from {path} at Y{:.1} — {} probes, {} colonies",
                sim.time.as_years(),
                sim.probes.len(),
                sim.colonies.len()
            );
            sim
        }
        None => Simulation::new(SimConfig {
            seed: args.seed,
            policy: args.policy,
            respect_warnings: !args.bold,
            ..SimConfig::default()
        }),
    };
    let cfg = sim.cfg.clone();
    println!(
        "von Neumann probe expansion — seed {}, {} years",
        cfg.seed, args.years
    );
    println!(
        "cruise {:.2}c | replication {:.1} yr | max hop {:.0} ly | drift ±{:.0}% | doctrine {:?}{}\n",
        cfg.cruise_speed_c,
        cfg.replication_years,
        cfg.max_hop_ly,
        cfg.drift * 100.0,
        cfg.policy,
        if cfg.respect_warnings { "" } else { " (bold)" }
    );

    println!(
        "{:>6}  {:>7}  {:>8}  {:>9}  {:>9}  {:>7}  {:>6}  {:>6}  {:>7}",
        "year", "probes", "transit", "colonies", "frontier", "max gen", "lost", "killed", "civs"
    );
    let start_year = sim.time.as_years();
    let end_year = start_year + args.years;
    let mut year = start_year;
    while year < end_year {
        year = (year + args.step).min(end_year);
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

    let now = SimTime::from_years(end_year);
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
    if args.map {
        render_map(&sim);
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

    if let Some(path) = &args.save {
        match std::fs::write(path, sim.to_json()) {
            Ok(()) => println!("saved to {path}"),
            Err(e) => eprintln!("failed to save {path}: {e}"),
        }
    }
}

/// Top-down chart of the expansion sphere (omniscient debug view, not the
/// light-lagged player view). One character ≈ (2R/width) light-years.
fn render_map(sim: &Simulation) {
    use vn_engine::civs::Disposition;

    const W: i32 = 71;
    const H: i32 = 35;
    let radius = (sim.frontier_radius_ly() * 1.15 + 25.0).max(60.0);
    let years = sim.time.as_years();
    // Chars are ~2× taller than wide; scale y by 2 to keep circles round.
    let sx = 2.0 * radius / W as f64;
    let sy = 2.0 * radius / H as f64;
    let to_cell = |x: f64, y: f64| -> Option<(i32, i32)> {
        let cx = (x / sx).round() as i32 + W / 2;
        let cy = (y / sy).round() as i32 + H / 2;
        (cx >= 0 && cx < W && cy >= 0 && cy < H).then_some((cx, cy))
    };
    let mut grid = vec![b' '; (W * H) as usize];
    let mut put = |cell: Option<(i32, i32)>, ch: u8, grid: &mut Vec<u8>| {
        if let Some((cx, cy)) = cell {
            grid[(cy * W + cx) as usize] = ch;
        }
    };

    // Civ territories as background fill.
    for cy in 0..H {
        for cx in 0..W {
            let x = (cx - W / 2) as f64 * sx;
            let y = (cy - H / 2) as f64 * sy;
            if let Some(civ) = sim.civ_field.territory_at(x, y, years) {
                let ch = match civ.disposition {
                    Disposition::Extinct => b'.',
                    Disposition::Watcher => b'w',
                    Disposition::Territorial => b't',
                    Disposition::Expansionist => b'x',
                };
                grid[(cy * W + cx) as usize] = ch;
            }
        }
    }
    // Claimed (in-flight targets): the fringe of the wave.
    for id in sim.claimed_stars() {
        let s = sim.galaxy.star(*id);
        put(to_cell(s.x, s.y), b'\'', &mut grid);
    }
    // Colonies.
    for id in sim.colonies.keys() {
        let s = sim.galaxy.star(*id);
        put(to_cell(s.x, s.y), b'o', &mut grid);
    }
    // Civ homeworlds over their fill.
    for civ in sim.civ_field.civs_near(0.0, 0.0, radius * 1.5, years) {
        let ch = match civ.disposition {
            Disposition::Extinct => b'E',
            Disposition::Watcher => b'W',
            Disposition::Territorial => b'T',
            Disposition::Expansionist => b'X',
        };
        put(to_cell(civ.x, civ.y), ch, &mut grid);
    }
    put(to_cell(0.0, 0.0), b'@', &mut grid);

    println!(
        "\n─── galaxy chart Y{:.0} — {:.0} ly across (omniscient view) ───",
        years,
        2.0 * radius
    );
    for cy in 0..H {
        let row = &grid[(cy * W) as usize..((cy + 1) * W) as usize];
        println!("{}", std::str::from_utf8(row).unwrap());
    }
    println!(
        "@ Sol   o colony   ' target   E/W/T/X civ home   . ruins  w watcher  t territorial  x swarm"
    );
}
