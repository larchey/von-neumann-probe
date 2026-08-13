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
cargo run --release --bin vnp -- --policy richest --bold --save empire.json
cargo run --release --bin vnp -- --load empire.json --years 1000
```

Sample output:

```
  year   probes   transit   colonies   frontier  max gen      lost
   250       64        23         12     20.6ly        3         0
   500      239        46         37     44.3ly        4         0

─── mission control log (light-lagged; 150 of 167 signals received) ───
[recv Y 481.6 | sent Y 443.6 |  38.0 ly] Colony established at HIP-44353 (richness 0.90).
[recv Y 486.9 | sent Y 462.9 |  24.0 ly] HIP-82099 surveyed: richness 0.25, below viability.
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
  event schedules future events. 20,000 simulated years ≈ 3M events ≈ 47 s.
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
- [x] Expansion doctrines (`nearest` / `richest` / `outward`, `--bold`)
- [x] Save/load with bit-identical resume (digest-verified)
- [x] ASCII galaxy chart (`--map`)
- [ ] Interactive mode: change doctrine mid-run, respond to first contact
- [ ] Spec investment choices (speed vs reliability vs fabrication)
- [ ] Aggregation layer for 10⁸+ probes (statistical colonies + report pruning)
- [ ] Player-visible lag: strategic view built only from received reports
- [ ] TUI map view; graphical frontend later

## License

MIT
