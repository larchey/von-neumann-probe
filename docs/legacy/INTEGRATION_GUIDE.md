# Integration Guide: Multi-Layer Simulation Architecture

## Overview

This guide explains how to integrate the new scaling systems into Von Neumann Probe's existing codebase. The architecture is **modular** — each layer can be adopted incrementally.

## New Modules

### 1. `simulation_layer.rs` — Multi-Layer Entity Management

**Purpose:** Manage entities across three simulation layers (active/strategic/archive)

**Key Types:**
- `SimulationLayerManager` — orchestrates layer transitions
- `StrategicSwarm` — aggregated entity representation
- `SimulationLayer` — enum: Active | Strategic | Archive

**Systems:**
- `update_viewport_center` — track camera for layer boundary calculation
- `layer_transition_system` — move swarms between layers as distance changes
- `active_swarm_simulation` — full-detail physics for viewport entities
- `strategic_swarm_simulation` — reduced-fidelity movement for far swarms
- `event_propagation_system` — cross-layer signaling (distant events affect active zone)

**Integration Point:** Add to main schedule:
```rust
schedule.add_systems((
    update_viewport_center,
    layer_transition_system,
    active_swarm_simulation,
    strategic_swarm_simulation,
    event_propagation_system,
));
```

---

### 2. `spatial_hash.rs` — O(1) Entity Queries

**Purpose:** Replace O(n²) brute-force targeting with spatial grid lookups

**Key Types:**
- `SpatialGrid` — hash grid for fast radius/rect queries

**Systems:**
- `maintain_spatial_grid` — update grid when entities move (runs on `Changed<Transform>`)
- `cleanup_spatial_grid` — remove entities from grid when despawned

**Usage Example:**
```rust
// Old: O(n²) threat targeting
for (threat, threat_pos) in threats.iter() {
    for (probe, probe_pos) in probes.iter() {
        if distance(threat_pos, probe_pos) < 200.0 {
            threat.target = probe; // Found target
        }
    }
}

// New: O(1) with spatial grid
let nearby_probes = spatial_grid.query_radius(threat_pos, 200.0);
if let Some(closest_probe) = nearest_entity(&nearby_probes, threat_pos) {
    threat.target = closest_probe;
}
```

**Integration Point:** Add to main schedule (early):
```rust
schedule.add_systems((
    maintain_spatial_grid,
    cleanup_spatial_grid,
));
```

---

### 3. `archive_system.rs` — Persistent Sector Storage

**Purpose:** Serialize dormant sectors to disk, restore on demand with guaranteed determinism

**Key Types:**
- `SectorArchive` — serialized snapshot of a sector
- `ArchiveManager` — handles save/load operations
- `ArchivedSwarm` — serde-compatible swarm representation

**Key Functions:**
- `create_archive()` — snapshot a sector before unloading
- `load_archive()` — retrieve sector from cache/disk
- `wake_archived_sector()` — re-simulate from archive to current time
- `verify_checksum()` — detect corruption

**Integration Point:**
```rust
// When a sector leaves the strategic layer:
let archive = archive_manager.create_archive(
    sector_coords,
    swarms,
    current_frame,
    wall_time,
    rng_seed
);
archive_manager.save_to_disk(&archive)?;

// When player approaches the sector:
let awakened = wake_archived_sector(&archive_manager, current_frame, sector_coords)?;
// Merge awakened swarms into strategic layer
```

---

## Integration Workflow

### Step 1: Adopt Spatial Hashing (Low Risk, High Reward)

**Effort:** 2-3 hours | **Performance gain:** 10-100x on targeting

1. Add `spatial_hash.rs` to main.rs
2. Initialize `SpatialGrid::new(100.0)` as resource
3. Add systems to schedule: `maintain_spatial_grid`, `cleanup_spatial_grid`
4. Replace `threat_targeting` system with grid-based version
5. Benchmark: Compare frame times before/after

**Backwards Compatible:** Yes, existing code works unchanged (add spatial as optimization layer)

---

### Step 2: Adopt Strategic Layer (Medium Effort)

**Effort:** 4-6 hours | **Performance gain:** 10-20x on far entities

1. Add `simulation_layer.rs` to main.rs
2. Initialize `SimulationLayerManager` as resource
3. Create swarm aggregation system:
   ```rust
   fn aggregate_strategic_entities(
       mut commands: Commands,
       mut layer_manager: ResMut<SimulationLayerManager>,
       entities_to_aggregate: Query<(Entity, &Transform, ...), Far>,
   ) {
       // Cluster entities by proximity
       // Create StrategicSwarm from each cluster
       // Despawn original entities
       // Spawn swarms in strategic layer
   }
   ```
4. Add systems: `update_viewport_center`, `layer_transition_system`, `active_swarm_simulation`, `strategic_swarm_simulation`
5. Test: Verify swarms move correctly, layer transitions smooth

**Backwards Compatible:** Partial (individual entities in active layer work as before; strategic swarms are new)

---

### Step 3: Adopt Archive System (Integration Required)

**Effort:** 5-7 hours | **Performance gain:** Unbounded scaling

1. Add `archive_system.rs` to main.rs
2. Initialize `ArchiveManager` with target archive directory
3. Add sector serialization on unload:
   ```rust
   fn archive_unloading_sectors(
       mut archive_manager: ResMut<ArchiveManager>,
       mut sector_manager: ResMut<SectorManager>,
       layers: Res<SimulationLayerManager>,
   ) {
       for sector_coords in sectors_to_unload {
           let swarms = layers.strategic_swarms.iter()
               .filter(|(_, s)| sector_of(s.position) == sector_coords)
               .map(|(_, s)| s.clone())
               .collect();
           
           archive_manager.create_archive(
               sector_coords,
               swarms,
               current_frame,
               get_wall_time(),
               get_rng_seed()
           );
       }
   }
   ```
4. Add sector restoration on load:
   ```rust
   fn restore_archived_sectors(
       archive_manager: Res<ArchiveManager>,
       mut commands: Commands,
       target_sector: Res<SectorToLoad>,
   ) {
       if let Ok(swarms) = wake_archived_sector(&archive_manager, current_frame, target_sector) {
           for swarm in swarms {
               commands.spawn((
                   StrategicSwarm { ..swarm },
                   // ... other components
               ));
           }
       }
   }
   ```
5. Test: Play for 1+ hour, verify no desync on sector reload

**Breaking Changes:** None if archive is optional; enable via `GameConfig`

---

## Configuration

Add to `resources.rs`:

```rust
#[derive(Resource, Clone)]
pub struct ScalingConfig {
    pub enable_spatial_hash: bool,        // Default: true
    pub enable_strategic_layer: bool,     // Default: false (Phase 2)
    pub enable_archive_system: bool,      // Default: false (Phase 3)
    
    pub active_distance: f32,             // 2000m
    pub strategic_distance: f32,          // 10000m
    pub spatial_cell_size: f32,           // 100m
    
    pub archive_dir: String,              // "./archives"
    pub archive_compression: u32,         // 1-22 (zstd)
    pub max_entities_active: usize,       // 50K
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            enable_spatial_hash: true,
            enable_strategic_layer: false,
            enable_archive_system: false,
            active_distance: 2000.0,
            strategic_distance: 10000.0,
            spatial_cell_size: 100.0,
            archive_dir: "./archives".to_string(),
            archive_compression: 6,
            max_entities_active: 50_000,
        }
    }
}
```

Then gate systems:
```rust
if config.enable_spatial_hash {
    schedule.add_systems(maintain_spatial_grid);
}
if config.enable_strategic_layer {
    schedule.add_systems(layer_transition_system);
}
if config.enable_archive_system {
    schedule.add_systems(archive_unloading_sectors);
}
```

---

## Testing Checklist

### Unit Tests
- [ ] `test_spatial_insert_query` — Spatial grid lookups work
- [ ] `test_layer_assignment` — Entities assigned to correct layer
- [ ] `test_swarm_transition` — Swarms move between layers smoothly
- [ ] `test_archive_determinism` — Same seed = identical re-simulation
- [ ] `test_archive_checksum` — Corruption detected

### Integration Tests
- [ ] Existing threat targeting still works with spatial grid
- [ ] Strategic swarms update position correctly
- [ ] Layer transitions don't cause entity duplication
- [ ] Archive save/restore maintains entity state
- [ ] Cross-layer events propagate correctly

### Performance Tests
- [ ] 10K entities: <2ms frame time
- [ ] 100K strategic: <2ms frame time
- [ ] Archive I/O doesn't block main thread
- [ ] Memory stays <100 MB on 8GB system

### Multiplayer Tests (if applicable)
- [ ] Archive checksums match across clients
- [ ] Layer transitions synchronized
- [ ] Deterministic re-simulation matches all peers

---

## Troubleshooting

### Swarms Flicker Between Layers
**Problem:** Swarm oscillates at layer boundary
**Solution:** Add hysteresis — move boundary threshold to 1.2x distance when promoting

```rust
if distance > manager.strategic_distance * 1.2 {
    // Actually transition to archive
}
```

### Memory Spike During Sector Load
**Problem:** Loading large archive causes frame stutter
**Solution:** Spread load over multiple frames

```rust
let mut loaded = 0;
for swarm in awakened_swarms {
    if loaded % 100 == 0 {
        // Yield every 100 swarms
        return; // Resume next frame
    }
    commands.spawn(swarm);
    loaded += 1;
}
```

### Archive Checksum Mismatch
**Problem:** Restored sector differs from original
**Solution:** 
1. Verify RNG seed is deterministic (don't use `thread_rng()`)
2. Check for non-deterministic operations (floating-point precision issues)
3. Use fixed-point math where precision matters

---

## Performance Targets (Recap)

| Layer | Budget | Entities | Typical Load |
|-------|--------|----------|--------------|
| Active | 1.5ms | 1K-50K | Full detail, every frame |
| Strategic | 0.3ms | 100K | Aggregated swarms, deterministic |
| Archive | 0.1ms (disk I/O async) | 100M+ | Serialized, loaded on demand |

**Total frame time:** <2ms for simulation (leave 14ms for rendering)

---

## Next: Multiplayer & Replication

Once archive system is stable, add:

1. **Sector Sync** — Archive checksums exchanged between peers
2. **Event Broadcast** — Cross-layer events sent to all clients
3. **Deterministic Rollback** — Restore universe to frame N if desync detected
4. **Save/Load** — Serialize entire universe state (use archive format)

See `MULTIPLAYER.md` (future)
