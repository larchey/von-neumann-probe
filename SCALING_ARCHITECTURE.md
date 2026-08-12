# Von Neumann Probe: Infinite-Scale Simulation Architecture

## Problem

Current naive approach (all entities simulated at full detail every frame) breaks at:
- **10K entities**: O(n²) targeting/collision → frame rate tanks
- **100K entities**: Memory/cache thrashing, no room for game logic
- **1M+ entities**: Impossible without fundamental redesign

Goal: **Support 100M+ entities with stable 60 FPS performance on commodity hardware.**

## Solution: Multi-Layer Deterministic Simulation

### Layer 1: Active (High Detail) — Viewport + 2x Buffer

**What:** Only entities within camera view + safety buffer are simulated at full tick rate
- Player sees: detailed positions, individual combat, precise movement
- Cost: O(entities_in_view) per frame, not O(all_entities)
- Typical view: 100-1000 entities on screen → 10K-100K in active layer with buffer

**Implementation:**
- Viewport-relative coordinate system (0,0 = screen center)
- Spatial hash grid (100m cells) for O(1) collision/targeting lookups
- Full ECS ticks for physics, combat, AI

### Layer 2: Strategic (Low Detail) — Far Regions (2x-20x buffer distance)

**What:** Aggregated behavior for regions beyond active layer
- Instead of 1M individual Rogues, store "Swarm #4782: 1M units, position (420, 300), velocity NW"
- Movement is deterministic trajectory (Bezier curve or linear prediction)
- No per-entity simulation; only swarm-level math
- Cost: O(swarms) per frame, not O(entities)
- Typical: 100K entities → 50 swarms → 99.95% performance gain

**Benefits:**
- Player sees emergent macro behavior (swarms, waves, fronts)
- Scalable event backbone (swarm collision → event → affects active layer)
- Save state: ~100 bytes/swarm vs 100+ bytes/entity

**Mechanics:**
```
Strategic Swarm = {
  id: UUID,
  swarm_type: ProbeType | ThreatType,
  count: u32,
  position: Vec2,
  velocity: Vec2,
  heading_angle: f32,
  cohesion_center: Vec2,
  health_total: f32,
  resource_pool: Resources,
  threat_level: f32,
  formation: FormationType,
}
```

Movement: `position += velocity * dt` (pure deterministic, no randomness)

### Layer 3: Archive (Dormant) — Unseen Regions (>20x buffer)

**What:** Serialized sector snapshots, zero simulation cost
- Snapshot taken when sector unloads
- Sector wakes from snapshot when player approaches
- Deterministic re-simulation from snapshot for consistency
- Cost: disk I/O on load, O(1) storage per sector

**Mechanics:**
```
SectorArchive = {
  sector_coords: (i32, i32),
  timestamp: u64,
  swarms: Vec<StrategicSwarm>,
  structures: Vec<CathedralStructure>,
  threats: Vec<ThreatAggregate>,
  checksum: u64,  // CRC for integrity
}
```

Wake sequence:
1. Load archived sector
2. Verify checksum
3. Re-simulate from archive time to current time
4. Merge into active/strategic layers

### Event Backbone: Cross-Layer Signaling

**Problem:** Far swarms can't directly affect active probes (they're in different simulation layers)

**Solution:** Event bus with propagation rules:

```
Event = {
  source_layer: Layer,
  event_type: Collision | Discovery | Threat | ResourceSpike,
  source_position: Vec2,
  severity: f32,  // 0-1, determines propagation radius
  age: f32,
}

PropagationRule:
  if distance_to_active_layer < severity * 5000m:
    → spawn tactical response in active layer
  if distance_to_strategic < severity * 20000m:
    → update strategic swarm heading/state
```

**Example:** Strategic Leviathan swarm collides with archive obstacle
→ Event(Collision, position=(4200, 3100), severity=0.9)
→ Player's active probes 2000m away receive warning + threat increase
→ Nearby strategic swarms reroute around obstacle

## Deterministic Simulation Guarantee

**Goal:** Same seed + initial state = identical universe forever, regardless of save/load or camera movement.

**Implementation:**
- **Sector generation:** Seeded from coords: `seed = hash(x, y, VERSION)`
- **Threat spawning:** Seeded from sector + time slice: `seed = hash(sector, frame/100, VERSION)`
- **Swarm movement:** Deterministic velocity (no random walks)
- **Archive wakes:** Re-simulate from timestamp using same RNG seed

**Benefit:** 
- No desync across multiplayer or replay
- Save game = checkpoint + timestamp, not full entity list
- Can restore universe to *any* point in past (if archive kept)

## Memory Scaling Analysis

Assume:
- 10M total entities in universe
- 100 entities visible on screen
- Player in active layer: 10K entities
- Strategic layer: 100K entities
- Archive layers: 9.9M entities (serialized, on disk)

**Memory per entity:**
- Active ECS component: ~100 bytes (transform, health, velocity, resources)
- Strategic swarm: ~150 bytes (aggregated)
- Archive: ~50 bytes (serialized)

**RAM breakdown:**
- Active layer: 10K × 100B = 1 MB
- Strategic layer: 100K × 50B = 5 MB (pointers only; full state on disk)
- System resources: ~50 MB (spatial hash, event queue, manager state)
- **Total RAM: ~60 MB** (independent of total entity count!)

**Disk:**
- Archive: 9.9M × 50B = 495 MB (compressible to ~50-100 MB with zstd)

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Active layer entities | <50K | Full simulation cost = 1ms |
| Strategic entities | <500K | Aggregated queries = 1ms |
| Frame time budget | <16.6ms @ 60 FPS | Leave 15ms for rendering |
| Simulation time | <2ms | Physics + AI + events |
| Archive save | <100ms | Zstd compress on worker thread |
| Sector load (from disk) | <200ms | Async I/O, pre-fetch next sectors |

## Implementation Phases

### Phase 1: Spatial Hashing (2-3 days)
Replace O(n²) targeting with O(1) grid lookups.
- `SpatialGrid`: divide world into 100m cells
- `SpatialQuery`: radius/rect queries return nearby entities
- Benchmark: 10K entities, 1000 queries/frame → 0.5ms (was 50ms)

### Phase 2: Strategic Layer (3-5 days)
Aggregate far entities into swarms.
- `SwarmAggregator`: detect swarms from entity clusters
- `StrategicSwarm` component: replaces 1000s of individual entities
- Event propagation: swarms ↔ active layer signaling
- Benchmark: 100K entities → 50 swarms → <2ms/frame

### Phase 3: Archive System (2-3 days)
Serialize unseen sectors to disk.
- `SectorArchive`: bincode + zstd compression
- Wake/sleep transitions: load from disk, re-simulate timestamp
- Checksum verification: catch corruption
- Benchmark: sector load from disk <200ms, wake-recompute <100ms

### Phase 4: Determinism & Replay (2-3 days)
Guarantee reproducibility.
- Seed all RNG from sector + time
- Remove any floating-point non-determinism (use fixed-point or integer math where needed)
- Replay system: restore universe to frame N
- Benchmark: identical simulation across 1000 frame rollback

### Phase 5: Optimization (ongoing)
Profiling & tuning.
- SIMD vectorization for batch transforms
- Async task spawning for heavy computations
- Spatial hash tuning (cell size for your entity density)
- Memory pooling (pre-alloc entity buckets)

## Test Plan

```rust
#[test]
fn test_1M_entities_60fps() {
  let world = generate_universe_with_1M_entities();
  let mut frame_times = vec![];
  for frame in 0..300 {
    let start = Instant::now();
    world.tick();
    frame_times.push(start.elapsed());
  }
  let p99 = percentile(frame_times, 99);
  assert!(p99 < Duration::from_millis(16), "P99 frame time: {:?}", p99);
}

#[test]
fn test_deterministic_1000_rollback() {
  let mut world_a = Universe::new(seed=12345);
  let mut world_b = Universe::new(seed=12345);
  
  for _ in 0..1100 {
    world_a.tick();
    world_b.tick();
  }
  
  // Roll A back 100 frames
  world_a.restore_checkpoint(frame=1000);
  
  // Should be identical
  for _ in 0..100 {
    world_a.tick();
    world_b.tick();
    assert_eq!(world_a.entities(), world_b.entities());
  }
}
```

## Risk Mitigations

| Risk | Mitigation |
|------|-----------|
| Swarm LOD oscillation (flicker) | Hysteresis: swarm ↔ detail only at layer boundary, not constantly |
| Desync in multiplayer | Central authority server, all clients use server's RNG seed |
| Disk fragmentation (archive churn) | Pre-allocate sector file slots, archive compaction weekly |
| Floating-point drift in determinism | Use integer/fixed-point math for movement, convert to float for rendering only |
| Memory spikes during sector load | Async I/O + load spreading over 2-3 frames |

## Future Extensibility

This architecture supports:
- **Multiplayer:** Event backbone + archive checksums enable peer-to-peer sector sync
- **AI Expansion:** Strategic swarms can have complex behaviors (patrolling, retreating, converging)
- **Modding:** Determinism + reproducibility means user-created swarm behaviors are verifiable
- **Time dilation:** Simulation time ≠ wall time; can speed up far regions' "perception" of time
- **Undo/Redo:** Archive system naturally supports frame rollback and branching timelines
