# Design — von Neumann Probe

## Vision

You are the seed intelligence of a self-replicating probe launched from
Sol. The game is about **exponential growth under physical law**: no FTL,
no magic resources, no real-time control of distant assets. The fantasy is
the Bobiverse one — being a mind that copies itself across the galaxy and
watches its descendants diverge.

The game stays interesting from 1 probe to 1,000,000 by *changing what the
player does* as scale grows, not by making them do more of the same.

## The three acts of scale

**Act I — One mind (1–10 probes, Y0–500).** Intimate decisions. Every
probe matters, the galaxy is empty, and the only enemy is distance.

**Act II — A lineage (10–10k probes, Y500–5000).** You stop steering
probes and start writing *policy*. First contact happens out past 120 ly.
Named lines start splitting off as drift accumulates. The light-lagged
mission log becomes the main way you experience your own empire.

**Act III — A civilization (10k+, Y5000+).** You operate at the level of
doctrine and exceptions — and increasingly, you don't operate at all.
Lines secede. Expansionist rivals overrun your colonies on schedules set
centuries earlier. The frontier is a place you hear about, not a place you
govern.

## What the player actually does

The only lever is **doctrine**, broadcast from Sol at lightspeed:

- **Target policy** — `nearest` (dense consolidation), `richest` (settle
  only good systems: fewer colonies, each more productive), `outward`
  (race the wave outward).
- **Warning stance** — respect Watcher warnings, or push into their space
  until they start shooting.

Doctrine changes propagate at *c*. Change policy today and a colony 80 ly
out keeps executing your old orders for 80 more years. The empire is a set
of nested light-cones of obedience — and past a certain distance, lines
stop listening entirely.

## Core mechanics

- **Probe lifecycle** — launch → cruise (years to centuries) → survey →
  colonize or reject → autofactory → replicate → children launch outward.
- **Richness** gates everything local: factory build time, replication
  rate, total launch budget.
- **Replication drift** — children mutate the founder's spec (cruise
  speed, fabrication, reliability) by ±3%/generation.
- **Lineages** — a probe that founds a colony while ≥12% divergent from
  its line's template establishes a *new named line*. Drift becomes a
  family tree you can read.
- **Secession** — ≥30% divergence from the original template, founded
  beyond 200 ly ⇒ the line stops taking orders. It keeps expanding, just
  not for you.
- **Attrition & hazards** — transit losses scale with distance and inverse
  reliability; radiation-trap systems kill on arrival.
- **Saturation** — colonies exhaust accessible material and shut down.
  Growth is frontier-driven; the interior is inert.
- **Anomalies** — garden worlds (the mission's real scoreboard),
  derelicts and precursor caches (permanent spec gains), hazards.
- **Civilizations** — Extinct (salvageable ruins), Watcher (patient until
  provoked), Territorial (lethal fixed borders), Expansionist (rival
  replicator waves that overrun you on a closed-form schedule).
- **Light lag** — every event reaches the player at *c*. The map you can
  draw is built only from signals that have arrived.
- **Going dark** — seceded lines stop transmitting. Their colonies and
  their discoveries never reach Sol, so a garden world they find is one
  you never learn about. Control is what converts discovery into score.
- **They answer** — meeting a living civilization provokes a reply that
  cannot be sent before light from your probe reaches them, and then has
  to cross to Sol. You hear everyone twice-delayed.
- **The galaxy's record** — dead civilizations' archives say what ended
  them. The most common answer is their own self-replicating probes.

## Emergent behavior worth protecting

These weren't scripted; they fall out of the rules, and changes shouldn't
break them:

1. **Natural selection.** Lines that replicate and travel faster found
   more colonies, so their specs dominate the population over deep time.
   Evolution emerges from drift + differential growth, with no fitness
   function anywhere in the code.
2. **Obedience decay.** 100% → 92% → 21% over Y3000–12000. The empire
   outgrows its own control surface, and the fastest-growing lines are
   precisely the ones that left.
3. **Enclaves.** The expansion wave routes around known-hostile space,
   leaving civ territories as visible holes in an otherwise filled sphere.
4. **Stale frontiers.** The known-space map's outer edge is always older
   than its interior, because that's how long the light took.
5. **Control converts discovery into score.** Survey doctrine finds the
   most living worlds and reports the fewest, because the same long hops
   that reach them push its descendants past the range where they still
   answer. It wins the early game and loses the late one — an inversion
   nobody wrote, produced by composing secession with going dark.

## Design rules

1. **Never break lightspeed** — not for comms, not for convenience.
2. **The probe is the AI.** Player input is policy set in advance; distant
   probes execute autonomously.
3. **Numbers stay physical.** Years, light-years, fractions of c.
4. **Scale changes verbs, not just numbers.**
5. **Drift is content.** Divergence, secession, and loss are stories.

## Next layers (intended order)

1. **Rival replicator conflict** — independent lines currently still
   respect each other's claims. Making seceded lines and Expansionist
   civs contest the same systems would give the late game a real
   antagonist grown from your own hull design.
2. **Contact decisions** — let the player answer a transmission, knowing
   the reply will be centuries stale on arrival.
3. **Sol matters** — a home that can be lost, or that stops listening,
   to give the deep-time arc an ending rather than an asymptote.
4. **Statistical aggregation** — compress interior colonies to per-region
   counts, to push past ~10^7 probes.
5. **TUI frontend** — live map, scrolling log, doctrine panel.
