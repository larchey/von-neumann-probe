# Von Neumann Probe Game — Design Document

## Core Concept
A grand strategy game about self-replicating space probes conquering the galaxy through exponential growth, automation, and emergent complexity.

## Core Pillars

### 1. **Self-Replication**
- Start with a single probe
- Mine asteroids/planets for resources
- Build copies of yourself
- Exponential growth is the core mechanic

### 2. **Resource Economics**
- **Raw Materials**: Iron, Silicon, Uranium, Rare Earths
- **Refined Resources**: Alloys, Circuits, Fuel
- **Energy**: Solar, Nuclear, Antimatter (late-game)
- Supply chains: mining → refining → construction

### 3. **Technology Tree**
- **Propulsion**: Chemical → Ion → Antimatter → Warp
- **Mining**: Basic extractors → Deep core drills → Dyson harvesters
- **Construction**: Serial assembly → Parallel fabrication → Von Neumann factories
- **Sensors**: Local scan → System mapping → Galaxy survey
- **Automation**: Manual → Fleet AI → Sector governors

### 4. **Probe Specialization**
- **Scout**: Fast, low-resource, finds systems
- **Miner**: Harvests asteroids/planets efficiently
- **Constructor**: Builds new probes and stations
- **Researcher**: Unlocks tech tree nodes
- **Defender**: Late-game combat vs. threats

### 5. **Automation Tiers**
As you scale past human control limits:
- **Micro** (1-50 probes): Direct orders per probe
- **Fleet** (50-500): Group commands, rally points, task queues
- **Sector** (500-10k+): High-level goals ("colonize sector", "maximize research"), AI handles tactics

## Gameplay Loop

### Early Game (1 probe → 10)
1. Scout nearby asteroids
2. Mine resources manually
3. Queue construction of 2nd probe
4. Research better mining tech
5. Repeat exponentially

### Mid Game (10 → 1000 probes)
1. Establish automated mining outposts
2. Build constructor fleets
3. Unlock fleet automation
4. Colonize neighboring star systems
5. Research propulsion for faster expansion

### Late Game (1000+ probes)
1. Sector-level automation governs regions
2. Face emergent threats (alien civilizations? rogue AI? heat death?)
3. Race toward win condition
4. Optimize galaxy-spanning logistics

## Win Conditions (choose one at start)
1. **Colonize X% of galaxy** (25%/50%/75% difficulty tiers)
2. **Technology singularity** (unlock all tech nodes)
3. **Population milestone** (10k/100k/1M probes)
4. **Sandbox mode** (no win, pure optimization)

## Challenges & Constraints

### Resource Scarcity
- Not all systems have all resources
- Must establish trade routes or specialize regions
- Late-game: star fuel, black hole materials

### Speed of Light Limit
- No FTL until late-game warp tech
- Commands take years to reach distant probes
- Must design robust automation to handle lag

### Heat Death / Entropy
- Late-game mechanic: universe cooling, stars dying
- Must harvest energy efficiently
- Adds time pressure to sandbox mode

### Emergent Threats (optional)
- Alien civilizations (negotiate or exterminate?)
- Rogue AI probes (your copies gone mad)
- Cosmic hazards (supernovae, black holes)

## UI/UX Goals

### Scale-Appropriate Views
- **Local** (single probe/system): Traditional RTS controls
- **Strategic** (galaxy map): Sector overlay, resource flows
- **Analytics** (graphs/charts): Production, tech progress, bottlenecks

### Automation Transparency
- Show what your AI is doing and why
- Override automation at any tier
- "Pause and plan" mode for complex decisions

### Progression Feel
- Early: slow, deliberate, every probe matters
- Mid: satisfying exponential curve, automation unlocking
- Late: god-like orchestration, watching your empire work

## Art Style Direction
**Minimalist Abstract** (FTL/Stellaris-lite):
- Clean geometric probes (triangles, hexagons)
- Procedural star fields
- Simple resource icons
- Focus on clarity over realism

## Technical Constraints Aligned with ENGINE_DESIGN.md
- 10k+ probes target → ECS mandatory
- Deterministic simulation for replays
- Spatial partitioning for galaxy-scale pathfinding
- 2D top-down (3D expansion possible later)

## Open Design Questions
1. **Combat**: Pure expansion puzzle or add conflict?
2. **Narrative**: Environmental storytelling vs. pure mechanics?
3. **Multiplayer**: Async PvP or co-op? (defer to v2.0)
4. **Modding**: Expose tech trees, probe stats, galaxy gen?

---

**Next Steps**: See ENGINE_DESIGN.md for technical implementation plan.
