# Engine — technical architecture

`vn-engine` is a headless, deterministic, discrete-event simulation (DES)
library. Frontends (CLI today; TUI/graphics later) are thin consumers that
render state and inject player policy. Nothing in the engine knows about
rendering, input, or wall-clock time.

## Why discrete-event, not tick-based

A tick/ECS loop costs `entities × ticks` even when nothing changes. At
interstellar timescales that's catastrophic: a probe cruising for 40 years
would be visited ~10⁹ times to do nothing. The DES core inverts this:

- Every future happening is an `Event` in a `BinaryHeap` ordered by
  `(SimTime, seq)`.
- `run_until(t)` pops and handles events until the queue's head passes
  `t`. Handling an event mutates state and schedules future events.
- Between events, **zero work happens**. A million dormant probes cost
  nothing per simulated year.

Measured: 20,000 simulated years → ~1.45M probes, ~3M events, ~47 s
(release build). Cost is linear in events handled.

The `seq` tiebreaker makes simultaneous events fire in insertion order,
which is itself deterministic — so event ordering is total and stable.

## Determinism (a hard requirement)

Same seed + config ⇒ bit-identical state, forever, on every platform.
This buys: reproducible bugs, save-file integrity, replays, and eventually
lockstep multiplayer. The rules that keep it true:

1. **All randomness is hand-rolled splitmix64** (`rng.rs`) — no external
   RNG crates whose output could change under a dependency bump.
   - *Stateless hashing* for procedural content: star properties are pure
     functions of `(seed, cell, salt)`, so generation order can never
     matter.
   - *Forked streams* for event-time rolls: each purpose forks its own
     stream from stable keys (probe id, target id, salt), so adding a new
     consumer never perturbs existing sequences.
2. **No `HashMap`/`HashSet` iteration in logic paths.** `Simulation` uses
   `BTreeMap`/`BTreeSet` wherever order could leak into behavior.
3. **f64 restricted to IEEE-deterministic ops** (+, −, ×, ÷, sqrt) in
   state-affecting code. No libm transcendentals in the sim.
4. **Integer time.** `SimTime` is u64 seconds (~5.8×10¹¹ year range), so
   event ordering never suffers float comparison hazards.
5. **`Simulation::digest()`** folds all state into a u64 fingerprint.
   Tests assert repeat-run equality, seed divergence, and that chunked
   `run_until` calls match a single one-shot run. The digest also verified
   the name-generation optimization was behavior-neutral (identical digest
   before/after a 3.4× speedup).

## Lazy infinite galaxy (`galaxy.rs`)

Space is a grid of 16-ly cells. A cell's star count (1–3), positions, and
richness are stateless hashes of `(seed, cell coords)` — the galaxy is
unbounded and costs nothing until touched. Only systems gameplay has
visited hold mutable state (`Colony` entries in a `BTreeMap`).

Target selection (`nearest_star`) searches cells in expanding rings with a
deterministic scan order and early exit once no farther ring can beat the
best candidate. Star display names are generated on demand (`Galaxy::name`)
so the search path is allocation-free.

## State model (`sim.rs`)

Sparse maps, all serde-serializable:

- `probes: BTreeMap<ProbeId, Probe>` — individuals with hereditary
  `ProbeSpec` (cruise speed, fabrication, reliability) that mutates per
  generation.
- `colonies: BTreeMap<StarId, Colony>` — founded systems with a
  richness-scaled launch budget; saturated colonies go quiet.
- `claimed: BTreeSet<StarId>` — reservation set preventing duplicate
  targeting; released when an inbound probe is lost, so failures self-heal.
- `reports: Vec<Report>` — the light-lag observation layer (below).

## Light-lag observation (`report.rs`)

The simulation is ground truth; the *player's knowledge* is a separate,
physically-honest layer. Every event emits a `Report` stamped with
`occurred_at` and `received_at = occurred_at + distance/c`.
`reports_received_by(now)` is the only view frontends should render.
This is cheap (one Vec push per event) and gives the game its signature
feeling: the frontier you see is the frontier as it was.

## Scaling roadmap

Current design is comfortable to ~10⁶–10⁷ probes. Beyond that:

1. **Archive dormant individuals.** Settled probes at saturated colonies
   are historical record, not simulation participants — move them to an
   append-only archive keyed by colony; keep only active probes in the hot
   map.
2. **Aggregate colonies statistically.** Interior regions produce no
   events; their record can be compressed to per-region counts +
   generation histograms, regenerable on demand.
3. **Per-cell claim index.** `nearest_star` currently probes the claim set
   per candidate; a `BTreeMap<(i32,i32), SmallVec<StarId>>` sidecar makes
   ring scans skip fully-claimed cells.
4. **Event-queue sharding** by spatial region if the heap ever dominates —
   regions only interact through probe transfers, which are themselves
   events.

None of these change the architecture; they're all compression of cold
state. That's the property that makes the engine "infinitely" scalable:
activity lives only at the frontier, and the frontier is a thin shell.

## Crate layout

```
crates/
  vn-engine/          # the simulation library (this document)
    src/lib.rs        # SimConfig + module docs
    src/time.rs       # SimTime (u64 seconds)
    src/rng.rs        # splitmix64: stateless hashing + forked streams
    src/galaxy.rs     # lazy procedural starfield
    src/probe.rs      # Probe, ProbeSpec (hereditary, drifting)
    src/events.rs     # Event + deterministic BinaryHeap queue
    src/report.rs     # light-lagged player-knowledge layer
    src/sim.rs        # Simulation: state + event handlers
    tests/engine.rs   # determinism, expansion, light-lag invariants
  vn-cli/             # headless runner (binary: vnp)
```
