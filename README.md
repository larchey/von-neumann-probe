# von Neumann Probe

A game about self-replicating interstellar probes, in the spirit of the
Bobiverse: one seed probe, real physics, and an expansion wave that crawls
across the galaxy at a fraction of lightspeed while you read decades-old
news from the frontier.

Built as a deterministic, event-driven simulation engine (`vn-engine`) with
a headless CLI runner (`vn-cli`). No graphics yet — gameplay first.

## Quick start

```bash
cargo test                 # engine test suite
cargo run --release --bin vnp -- --years 500
cargo run --release --bin vnp -- --seed 7 --years 5000 --step 500 --map
cargo run --release --bin vnp -- --policy survey --invest speed --save empire.json
cargo run --release --bin vnp -- --load empire.json --years 1000
cargo run --release --bin vnp -- --interactive     # mission control REPL
```

Sample output:

```
─── mission control log (light-lagged; 21548 of 26224 signals received) ───
[recv Y2924.9 |  234.1 ly] GARDEN WORLD at HIP-06323. Oxygen, liquid water, a
                           biosphere. Found by the Bob line, 234 ly from Sol.
                           This is what we were built for.
[recv Y3931.0 |  331.2 ly] A gen-15 probe of the Bob line has founded its own at
                           HIP-35876: the Curie line, prolific — 0.11c, fab 1.13.
[recv Y4463.1 |  440.0 ly] The Qoldraren Watchers have issued a formal warning:
                           cease expansion into their space.

╔══ mission scorecard — Y4000 ══
║ GARDEN WORLDS FOUND              61   (1.52 per century, 1 per 82 surveyed)
║ colonies / population          2908 / 19966
║ frontier reached                489 ly
║ still answering to Sol           84%   (148 lines, 34 independent)
╚══
```

## The constraints are the game

A von Neumann probe has no FTL, no real-time control, and no supply line.
Everything in the design flows from that:

| Physical constraint | Gameplay consequence |
|---|---|
| No FTL (~0.1c cruise) | Hops between stars take decades; expansion is a wave, not a teleport |
| Light-speed comms | You see the frontier as it *was* — a colony 60 ly out is 60-year-old news |
| Local resources only | Colony output gated by system richness; barren systems get surveyed and abandoned |
| Delta-v budget | Single hops capped (~25 ly); dead-end frontiers go dormant |
| Imperfect replication | Each generation's spec drifts — by gen 25 your probes are not what you launched |
| Interstellar hazards | Transit attrition; lost probes release their claim for another colony to retry |
| Finite accessible material | Colonies saturate and shut down their replication lines |
| No real-time control | You set doctrine (`--policy`, `--bold`); every colony executes it autonomously |

## The mission has a point

Rare garden worlds — living planets — are what the probes were launched
to find, and they're the run's headline score. Every run ends with a
scorecard; doctrine choice visibly moves it (4,000-year runs, same seed):

| doctrine | garden worlds | frontier | still answering to Sol |
|---|---|---|---|
| `nearest` | 48 | 408 ly | 99% |
| `richest` | 40 | 434 ly | 76% |
| `outward` | 50 | 422 ly | 100% |
| `survey`  | **61** | **489 ly** | 84% |

`survey` chases the rich systems where life is likeliest, which means
longer hops — so it finds the most worlds and expands fastest, and its
descendants cross the secession range quickly enough to cost you a sixth
of your empire's obedience. `richest` is the extreme of that trade: the
best worlds-per-survey ratio and the worst control, holding only 76%.

## Your descendants stop being you

Every replica mutates its parent's spec by ±3%. That sounds like flavor;
over deep time it's the whole game.

- A probe that founds a colony while ≥12% divergent from its line's
  template **establishes a new named line** — drift becomes a family tree.
- A line ≥30% divergent from the original Sol template, founded beyond
  200 ly, **secedes**: out there your orders arrive centuries stale and
  nothing enforces them. It keeps replicating, just not for you.
- Nothing in the code scores fitness — but lines that travel and replicate
  faster found more colonies, so **selection happens anyway**. By Y12000
  the largest lines are almost all breakaways, running +60% to +200%
  cruise speed.

The share of your empire that still answers to Sol decays on its own:
**100% at Y3000, 92% at Y6000, 21% at Y12000.**

## You are not alone

The deep galaxy is procedurally inhabited (never within 120 ly of Sol —
the early game is yours). Civs are lazy formulas like everything else:
existence and disposition are hashed from the seed, territory is
closed-form in time, and reactions are events traveling at sublight speed.

- **Extinct** — ruins that permanently upgrade the lineage that finds them
- **Watchers** — tolerant elders; ignore their warning and colonies start
  dying to interceptors launched from their homeworld (real flight time)
- **Territorial** — fixed borders, lethal pickets; the expansion wave
  learns to flow around them, leaving enclaves in your sphere
- **Expansionist** — rival replicator waves; their border growth schedules
  the exact year each of your colonies gets overrun

## Engine architecture (why it scales)

The core insight: at interstellar timescales, **almost nothing happens in
any given second**. A probe in a 40-year cruise needs zero computation
until it arrives. So there is no tick loop — the engine is a discrete-event
simulation driven by a time-ordered queue. Cost scales with *events*
(arrivals, replications, surveys), never with entities × frames.

- **Event-driven core** — `BinaryHeap` keyed by (time, seq); handling an
  event schedules future events. 20,000 simulated years ≈ 854k population
  across 124k colonies in 13.9 s / 244 MB.
- **Lazy procedural galaxy** — stars are pure functions of (seed, cell);
  only touched systems hold mutable state. Unbounded space, zero idle cost.
- **Fully deterministic** — hand-rolled splitmix64 streams, no external RNG
  deps, no HashMap iteration in logic paths. Same seed ⇒ bit-identical
  `digest()`, verified in tests (including chunked-vs-one-shot execution).
- **Light-lag observation layer** — the sim is ground truth; the player's
  view is reports propagating to Sol at c. Frontends render *knowledge*,
  not state.

See [DESIGN.md](DESIGN.md) for gameplay direction and
[ENGINE.md](ENGINE.md) for the technical deep-dive. The previous
(pre-rewrite) docs live in `docs/legacy/`.

## Status & roadmap

- [x] Deterministic DES core, procedural galaxy, probe lifecycle
- [x] Replication drift, attrition, saturation, light-lagged reports
- [x] CLI runner + test suite
- [x] Advanced civilizations (4 dispositions, first contact, retaliation)
- [x] Expansion doctrines (`nearest`/`richest`/`outward`/`survey`, `--bold`)
- [x] Save/load with bit-identical resume (digest-verified)
- [x] ASCII galaxy chart (`--map`)
- [x] Interactive mission-control REPL (`--interactive`): doctrine
  broadcasts propagate at *c* — the frontier obeys your old orders until
  the light-front of the new ones arrives
- [x] Player-visible lag: the `map` command draws only what signals have
  physically delivered to Sol
- [x] Named lineages: drift forks the family tree, and distant divergent
  lines secede and stop taking orders
- [x] Survey anomalies: garden worlds, derelicts, precursor caches, hazards
- [x] Deep-time cold-state compression (20k years in 13.9s / 244 MB)
- [x] Garden-world doctrine (`survey`) + end-of-run mission scorecard
- [x] Directed spec investment (`--invest speed|fab|rel`): steer evolution
  instead of only watching it
- [ ] Statistical aggregation for 10⁸+ probes
- [ ] TUI frontend: live map, scrolling log, doctrine panel

## License

MIT
