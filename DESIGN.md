# Design — von Neumann Probe

## Vision

You are the seed intelligence of a self-replicating probe launched from
Sol. The game is about **exponential growth under physical law**: no FTL,
no magic resources, no real-time control of distant assets. The fantasy is
the Bobiverse one — being a mind that copies itself across the galaxy and
watches its descendants diverge.

The game should stay fun from 1 probe to 1,000,000 probes by *changing what
the player does* as scale grows, not by making them do more of the same.

## The three acts of scale

**Act I — One mind (1–10 probes).** Intimate decisions: which star first?
Colonize this marginal system or push on? Every probe has a name and a
story. Losing one hurts.

**Act II — A lineage (10–10k probes).** You stop steering probes and start
writing *policy*: expansion doctrine (fill in vs. race outward), spec
investment (speed vs. reliability vs. fabrication), what to do about
drifted lineages that behave differently than designed. The mission log —
delayed by light — becomes the main way you experience your own empire.

**Act III — A civilization (10k+).** You operate at the level of doctrine
and exceptions. The frontier is centuries of light-lag away; whatever you
decreed long ago is what the frontier *is*. The game surfaces anomalies:
a lineage that stopped reporting, a region expanding wrong, something
found out there that isn't yours.

## Core mechanics (current engine)

- **Probe lifecycle**: launch → cruise (years–decades) → survey → colonize
  or reject → autofactory → replicate → children launch outward.
- **Richness**: each system's resource quality scales factory build time,
  replication rate, and total launch budget. Barren systems are stepping
  stones at best.
- **Replication drift**: children mutate the founder's spec (cruise speed,
  fabrication, reliability) by ±3% per generation. Over 20+ generations,
  lineages meaningfully diverge — free emergent narrative.
- **Attrition**: probes die in transit proportionally to distance and
  inversely to reliability. Claims are released so the frontier self-heals.
- **Saturation**: colonies exhaust accessible material after a
  richness-scaled number of replicas. Growth is frontier-driven — the wave
  matters, not the interior.
- **Light lag**: every event reaches the player at c. The UI must never
  show ground truth the player couldn't physically know yet.

## Design rules

1. **Never break lightspeed** — not for comms, not for convenience. Lag is
   the game's signature feeling.
2. **The probe is the AI.** Player input is policy set *in advance*;
   distant probes execute autonomously. No micromanaging what you couldn't
   physically reach.
3. **Numbers stay physical.** Years, light-years, fractions of c. No
   abstract "energy points."
4. **Scale changes verbs, not just numbers.** Each order of magnitude
   should retire an old activity and introduce a new one.
5. **Drift is content.** Divergence, dormancy, and loss are stories, not
   error states.

## Next gameplay layers (in intended order)

1. **Directives** — player-set expansion policy consumed at replication
   time: target selection weights (near vs. rich vs. outward), spec
   investment allocation, lineage naming. This is the first real *choice*
   layer and slots directly into `launch_from`/`on_replica_complete`.
2. **Spec investment** — colonies can spend extra replication time to
   build better children (trade growth rate for quality). Interacts with
   drift: engineered gains vs. random walk.
3. **Anomalies & threats** — rare procedural discoveries at survey time
   (derelicts, hazards, signals) that inject decisions and danger into
   Act II/III. A hostile replicator lineage — possibly your own drifted
   descendants — is the natural late-game antagonist.
4. **Deep-time events** — stellar flares, dust lanes, resource-poor voids
   shaping the map at the 1,000-year scale.
