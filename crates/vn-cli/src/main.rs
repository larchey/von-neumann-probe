//! vnp — headless runner for the von Neumann probe engine.
//!
//! Usage: vnp [--seed N] [--years N] [--step N] [--reports N]
//!
//! Runs the simulation and prints a decade-by-decade expansion table plus
//! the tail of mission control's message log. The log honors light lag:
//! you only see what a signal could physically have delivered to Sol.

use vn_engine::sim::{Doctrine, Simulation};
use vn_engine::time::SimTime;
use vn_engine::{SimConfig, SpecAxis, TargetPolicy};
use std::io::{BufRead, Write};

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
    interactive: bool,
    invest: Option<SpecAxis>,
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
        interactive: false,
        invest: None,
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
                    "survey" => TargetPolicy::Survey,
                    other => {
                        eprintln!("unknown policy: {other} (nearest|richest|outward|survey)");
                        std::process::exit(2);
                    }
                }
            }
            "--bold" => args.bold = true,
            "--save" => args.save = Some(grab()),
            "--load" => args.load = Some(grab()),
            "--invest" => {
                args.invest = match grab().as_str() {
                    "speed" => Some(SpecAxis::Speed),
                    "fab" | "fabrication" => Some(SpecAxis::Fabrication),
                    "rel" | "reliability" => Some(SpecAxis::Reliability),
                    "none" => None,
                    other => {
                        eprintln!("unknown investment axis: {other} (speed|fab|rel|none)");
                        std::process::exit(2);
                    }
                }
            }
            "--map" => args.map = true,
            "--interactive" | "-i" => args.interactive = true,
            "--help" | "-h" => {
                println!(
                    "vnp [--seed N] [--years N] [--step N] [--reports N] \
                     [--policy nearest|richest|outward] [--invest speed|fab|rel] \
                     [--bold] [--save FILE] [--load FILE] [--map]"
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
            invest: args.invest,
            ..SimConfig::default()
        }),
    };
    if args.interactive {
        interactive(&mut sim);
        return;
    }

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
    if let Some(axis) = cfg.invest {
        println!("engineering {axis:?} into every replica (slower builds, directed drift)\n");
    }

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
            sim.population(),
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

    println!();
    show_lineages(&sim, 10);

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

    scorecard(&sim);

    println!(
        "\nanomalies salvaged: {} | mean colony richness: {:.3}",
        sim.stats.anomalies_salvaged,
        sim.mean_colony_richness()
    );
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

/// Mission-control REPL. The one lever you have is doctrine — and your
/// broadcasts crawl outward at c, so the frontier keeps obeying old orders
/// for decades after you change your mind.
fn interactive(sim: &mut Simulation) {
    println!("mission control online — Y{:.1}. type 'help' for commands.", sim.time.as_years());
    let stdin = std::io::stdin();
    let mut last_recv = sim.time;
    loop {
        print!("Y{:.1}> ", sim.time.as_years());
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break; // EOF
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["run", years] => {
                let years: f64 = match years.parse() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("usage: run <years>");
                        continue;
                    }
                };
                sim.run_until(sim.time.plus_years(years));
                status(sim);
                // Only what light has delivered since the last check-in.
                let fresh: Vec<_> = sim
                    .reports_received_by(sim.time)
                    .into_iter()
                    .filter(|r| r.received_at > last_recv)
                    .collect();
                let total = fresh.len();
                for r in fresh.into_iter().rev().take(12).rev() {
                    println!(
                        "[recv Y{:>7.1} | {:>5.1} ly] {}",
                        r.received_at.as_years(),
                        r.distance_ly,
                        r.text
                    );
                }
                if total > 12 {
                    println!("(…and {} more signals; 'log <n>' to see more)", total - 12);
                }
                last_recv = sim.time;
            }
            // Advance until something worth hearing about arrives. This is
            // how you actually play across century-scale time: you don't
            // watch, you wait for the mail.
            ["next"] | ["next", _] => {
                let limit: f64 = parts
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(3000.0);
                let deadline = sim.time.plus_years(limit);
                let mut reported = false;
                while sim.time < deadline && !sim.is_finished() {
                    let before = sim.time;
                    sim.run_until(sim.time.plus_years(25.0));
                    let fresh = sim.significant_reports_between(before, sim.time);
                    if !fresh.is_empty() {
                        for r in fresh.iter().take(10) {
                            println!(
                                "[recv Y{:>7.1} | {:>5.1} ly] {}",
                                r.received_at.as_years(),
                                r.distance_ly,
                                r.text
                            );
                        }
                        if fresh.len() > 10 {
                            println!("(+{} more this decade)", fresh.len() - 10);
                        }
                        reported = true;
                        break;
                    }
                }
                last_recv = sim.time;
                if !reported {
                    println!(
                        "Y{:.0}: {} years pass. Nothing of note reaches Sol.",
                        sim.time.as_years(),
                        limit
                    );
                }
                if sim.is_finished() {
                    println!("The expansion has ended: no probes remain in flight or in production.");
                }
                status(sim);
            }
            ["status"] => status(sim),
            ["map"] => render_known_map(sim),
            ["map", "all"] => render_map(sim),
            ["lines"] => show_lineages(sim, 15),
            ["score"] => scorecard(sim),
            ["history"] => {
                if sim.fates_learned.is_empty() {
                    println!("No dead civilizations' archives recovered yet.");
                }
                for key in &sim.fates_learned {
                    if let Some(civ) = sim.civ_field.civ_by_key(*key) {
                        if let Some(fate) = civ.fate() {
                            println!(
                                "{:<34} {:>5.0} ly — ended by {}.",
                                civ.name(),
                                (civ.x * civ.x + civ.y * civ.y).sqrt(),
                                fate.describe()
                            );
                        }
                    }
                }
            }
            ["civs"] => {
                if sim.relations.is_empty() {
                    println!("no contact with other civilizations yet.");
                }
                for (key, rel) in &sim.relations {
                    if let Some(civ) = sim.civ_field.civ_by_key(*key) {
                        println!(
                            "{:<32} met Y{:<8.1} irritation {:<3} colonies lost: {}",
                            civ.name(),
                            rel.met_at.as_years(),
                            rel.irritation,
                            rel.colonies_lost_to
                        );
                    }
                }
            }
            ["log", n] => {
                let n: usize = n.parse().unwrap_or(20);
                for r in sim.reports_received_by(sim.time).into_iter().rev().take(n).rev() {
                    println!(
                        "[recv Y{:>7.1} | {:>5.1} ly] {}",
                        r.received_at.as_years(),
                        r.distance_ly,
                        r.text
                    );
                }
            }
            ["policy", p] => {
                let policy = match *p {
                    "nearest" => TargetPolicy::Nearest,
                    "richest" => TargetPolicy::Richest,
                    "outward" => TargetPolicy::Outward,
                    "survey" => TargetPolicy::Survey,
                    _ => {
                        println!("usage: policy nearest|richest|outward|survey");
                        continue;
                    }
                };
                let current = sim.doctrine_at(0.0, 0.0);
                sim.broadcast_doctrine(Doctrine { policy, ..current });
                println!(
                    "broadcast sent. the frontier ({:.0} ly out) will hear this in ~{:.0} years.",
                    sim.frontier_radius_ly(),
                    sim.frontier_radius_ly()
                );
            }
            ["invest", axis] => {
                let invest = match *axis {
                    "speed" => Some(SpecAxis::Speed),
                    "fab" | "fabrication" => Some(SpecAxis::Fabrication),
                    "rel" | "reliability" => Some(SpecAxis::Reliability),
                    "none" => None,
                    _ => {
                        println!("usage: invest speed|fab|rel|none");
                        continue;
                    }
                };
                let current = sim.doctrine_at(0.0, 0.0);
                sim.broadcast_doctrine(Doctrine { invest, ..current });
                match invest {
                    Some(a) => println!(
                        "broadcast sent: engineer {a:?}. Builds slow by {:.0}%, but drift on \
                         that axis only goes up — and directed lines diverge (and secede) faster.",
                        (vn_engine::INVESTMENT_TIME_COST - 1.0) * 100.0
                    ),
                    None => println!("broadcast sent: replicate as fast as possible."),
                }
            }
            ["bold", v @ ("on" | "off")] => {
                let current = sim.doctrine_at(0.0, 0.0);
                sim.broadcast_doctrine(Doctrine {
                    respect_warnings: *v == "off",
                    ..current
                });
                println!("broadcast sent (bold {}).", v);
            }
            ["save", path] => match std::fs::write(path, sim.to_json()) {
                Ok(()) => println!("saved to {path}"),
                Err(e) => println!("save failed: {e}"),
            },
            ["quit"] | ["exit"] => break,
            ["help"] => {
                println!("next [max]       advance until news worth hearing reaches Sol");
                println!("run <years>      advance the simulation by a fixed span");
                println!("status           one-line empire summary");
                println!("map              chart of what signals have reached Sol (your actual knowledge)");
                println!("map all          omniscient ground-truth chart (debug)");
                println!("civs             known civilizations");
                println!("history          how the dead ones died, from their archives");
                println!("lines            your descendant lineages and how they've drifted");
                println!("log <n>          last n received signals");
                println!("policy <p>       broadcast doctrine: nearest|richest|outward|survey (travels at c!)");
                println!("score            mission scorecard");
                println!("invest <axis>    engineer speed|fab|rel into replicas, or none (travels at c!)");
                println!("bold on|off      ignore/respect Watcher warnings (travels at c!)");
                println!("save <file>      write save");
                println!("quit             exit");
            }
            [] => {}
            _ => println!("unknown command; 'help' lists commands."),
        }
    }
}

/// End-of-mission summary. Garden worlds are the headline because they
/// are what the probes were launched to find; everything else is means.
fn scorecard(sim: &Simulation) {
    let years = sim.time.as_years().max(1.0);
    let surveyed = sim.systems_surveyed().max(1);
    println!("\n╔══ mission scorecard — Y{:.0} ══", years);
    println!(
        "║ GARDEN WORLDS FOUND      {:>10}   ({:.2} per century, 1 per {} systems surveyed)",
        sim.stats.garden_worlds,
        sim.stats.garden_worlds as f64 / years * 100.0,
        surveyed / sim.stats.garden_worlds.max(1) as u64
    );
    if sim.stats.garden_worlds_unreported > 0 {
        println!(
            "║   ...and {} more found by lines that no longer report to Sol",
            sim.stats.garden_worlds_unreported
        );
    }
    println!("║ systems surveyed         {:>10}", surveyed);
    println!(
        "║ colonies / population    {:>10} / {}",
        sim.colonies.len(),
        sim.population()
    );
    println!(
        "║ frontier reached         {:>10.0} ly   ({:.2} ly/century)",
        sim.frontier_radius_ly(),
        sim.frontier_radius_ly() / years * 100.0
    );
    println!(
        "║ still answering to Sol   {:>9.0}%   ({} lines, {} independent)",
        sim.obedient_fraction() * 100.0,
        sim.lineages.len(),
        sim.stats.independent_lines
    );
    println!(
        "║ civilizations met        {:>10}   ({} colonies lost to them)",
        sim.relations.len(),
        sim.stats.colonies_lost
    );
    println!(
        "║ probes lost              {:>10}   ({} transit, {} hazards, {} hostile)",
        sim.stats.probes_lost + sim.stats.hazard_losses + sim.stats.probes_killed,
        sim.stats.probes_lost,
        sim.stats.hazard_losses,
        sim.stats.probes_killed
    );
    println!("╚══");
}

/// The most productive lineages, with how far each has drifted from the
/// original Sol template — the empire's family tree.
fn show_lineages(sim: &Simulation, n: usize) {
    let root = sim.lineages.values().next();
    let root_spec = match root {
        Some(r) => r.template,
        None => return,
    };
    let mut lines: Vec<_> = sim.lineages.values().collect();
    lines.sort_by(|a, b| {
        b.probes_built
            .cmp(&a.probes_built)
            .then(a.id.0.cmp(&b.id.0))
    });
    println!(
        "{} lineages descended from the seed probe; top {}:",
        sim.lineages.len(),
        n.min(lines.len())
    );
    println!(
        "{:<12} {:<12} {:>6} {:>8} {:>9} {:>7} {:>6} {:>6}",
        "line", "from", "born", "probes", "colonies", "speed", "fab", "rel"
    );
    for l in lines.into_iter().take(n) {
        let parent = l
            .parent
            .and_then(|p| sim.lineages.get(&p))
            .map(|p| p.name.as_str())
            .unwrap_or("—");
        let pct = |v: f64, base: f64| (v / base - 1.0) * 100.0;
        let name = if l.independent {
            format!("{}*", l.name)
        } else {
            l.name.clone()
        };
        println!(
            "{:<12} {:<12} {:>6.0} {:>8} {:>9} {:>+6.0}% {:>+5.0}% {:>+5.0}%",
            name,
            parent,
            l.founded_at.as_years(),
            l.probes_built,
            l.colonies_founded,
            pct(l.template.cruise_speed_c, root_spec.cruise_speed_c),
            pct(l.template.fabrication, root_spec.fabrication),
            pct(l.template.reliability, root_spec.reliability),
        );
    }
    println!(
        "(percentages are drift from the original Sol template; * = independent, no longer takes orders)"
    );
}

fn status(sim: &Simulation) {
    println!(
        "Y{:>7.1} | {} probes ({} in transit) | {} colonies | frontier {:.1} ly | gen {} | {} lost, {} killed | {} civs known",
        sim.time.as_years(),
        sim.population(),
        sim.probes_in_transit(),
        sim.colonies.len(),
        sim.frontier_radius_ly(),
        sim.max_generation(),
        sim.stats.probes_lost,
        sim.stats.probes_killed,
        sim.relations.len()
    );
    println!(
        "          garden worlds found: {} | anomalies salvaged: {} | {} lineages ({} independent, {:.0}% of colonies still answer to Sol)",
        sim.stats.garden_worlds,
        sim.stats.anomalies_salvaged,
        sim.lineages.len(),
        sim.stats.independent_lines,
        sim.obedient_fraction() * 100.0
    );
}

/// The chart mission control can actually draw: built *only* from signals
/// that have physically reached Sol. Colonies appear when their founding
/// report arrives (decades late), disappear when their loss report does —
/// the far frontier on this map is always old news, and the wave's true
/// edge is invisible.
fn render_known_map(sim: &Simulation) {
    use vn_engine::civs::Disposition;
    use vn_engine::report::ReportKind;

    const W: i32 = 71;
    const H: i32 = 35;
    let received = sim.reports_received_by(sim.time);

    // Reconstruct knowledge in receive order; quantize positions so a
    // ColonyLost report cancels its ColonyFounded predecessor.
    let quant = |v: f64| (v * 10.0).round() as i64;
    let mut known_colonies: std::collections::BTreeMap<(i64, i64), (f64, f64)> =
        std::collections::BTreeMap::new();
    let mut known_civs: std::collections::BTreeMap<(i32, i32), f64> =
        std::collections::BTreeMap::new();
    let mut radius: f64 = 40.0;
    for r in &received {
        match r.kind {
            ReportKind::ColonyFounded => {
                known_colonies.insert((quant(r.x), quant(r.y)), (r.x, r.y));
                radius = radius.max(r.distance_ly * 1.2 + 15.0);
            }
            ReportKind::ColonyLost => {
                known_colonies.remove(&(quant(r.x), quant(r.y)));
            }
            _ => {}
        }
        if let Some(key) = r.civ {
            known_civs.entry(key).or_insert(r.occurred_at.as_years());
        }
    }

    let sx = 2.0 * radius / W as f64;
    let sy = 2.0 * radius / H as f64;
    let mut grid = vec![b' '; (W * H) as usize];
    let to_cell = |x: f64, y: f64| -> Option<(i32, i32)> {
        let cx = (x / sx).round() as i32 + W / 2;
        let cy = (y / sy).round() as i32 + H / 2;
        (cx >= 0 && cx < W && cy >= 0 && cy < H).then_some((cx, cy))
    };

    // Known civ territories, drawn at the radius we *observed* (the border
    // may have moved since — we wouldn't know yet).
    for (key, seen_years) in &known_civs {
        if let Some(civ) = sim.civ_field.civ_by_key(*key) {
            for cy in 0..H {
                for cx in 0..W {
                    let x = (cx - W / 2) as f64 * sx;
                    let y = (cy - H / 2) as f64 * sy;
                    if civ.contains(x, y, *seen_years) {
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
        }
    }
    for (_, (x, y)) in &known_colonies {
        if let Some((cx, cy)) = to_cell(*x, *y) {
            grid[(cy * W + cx) as usize] = b'o';
        }
    }
    if let Some((cx, cy)) = to_cell(0.0, 0.0) {
        grid[(cy * W + cx) as usize] = b'@';
    }

    println!(
        "\n─── known space, Y{:.0} — {:.0} ly across (as reported; the frontier is older than it looks) ───",
        sim.time.as_years(),
        2.0 * radius
    );
    for cy in 0..H {
        let row = &grid[(cy * W) as usize..((cy + 1) * W) as usize];
        println!("{}", std::str::from_utf8(row).unwrap());
    }
    println!(
        "@ Sol   o colony (as last heard)   . ruins  w watcher  t territorial  x swarm (borders as observed)"
    );
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
    let put = |cell: Option<(i32, i32)>, ch: u8, grid: &mut Vec<u8>| {
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
