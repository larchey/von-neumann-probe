# Von Neumann Probe — Engine Design Document

## Tech Stack

### Language: **Rust** 🦀
**Why Rust?**
- **Safety**: No memory leaks, data races, or UB → critical for complex simulation
- **Performance**: Zero-cost abstractions, SIMD, parallelism
- **Concurrency**: Safe multi-threading via ownership model
- **Ecosystem**: Mature game dev crates (bevy, wgpu, egui)

### Core Architecture: **ECS (Entity-Component-System)**
**Why ECS?**
- **Scalability**: Handle 10k+ probes efficiently
- **Cache-friendly**: Data-oriented design → fast iteration
- **Flexibility**: Add/remove components without refactoring
- **Parallelism**: Systems can run concurrently by default

**Crate**: `bevy_ecs` (standalone, no full Bevy engine overhead)

### Rendering: **wgpu + egui**
- **wgpu**: Modern GPU API (Vulkan/Metal/DX12 backends)
- **egui**: Immediate-mode GUI for debug panels, menus
- **Target**: 60 FPS with 10k entities on-screen

### Spatial Partitioning: **Hash Grid / Octree**
**Problem**: Galaxy-scale pathfinding and collision detection
- Naive O(n²) proximity checks = death at 1k+ probes
- **Solution**: Divide space into cells, only check nearby entities
- **Hash Grid** for uniform distribution (asteroid fields)
- **Octree** for sparse space (galaxy map)

### Pathfinding: **Hierarchical A***
**Problem**: Pathfinding across 1000+ star systems
- Traditional A* on full graph = slow
- **Solution**: Multi-level navigation mesh
  1. **High-level**: Star system graph (A* on 1k nodes)
  2. **Mid-level**: System-local sectors (100s of nodes)
  3. **Low-level**: Direct movement within sector

### Deterministic Simulation
**Why**: Replays, debugging, potential networking
- Fixed timestep (60 TPS)
- Deterministic RNG (seeded `rand::ChaCha8Rng`)
- No floating-point variance (use fixed-point for positions?)

### Multi-threading Strategy
- **Systems**: Parallel execution via ECS (read-only systems run concurrently)
- **Sectors**: Partition galaxy, simulate regions on separate threads
- **Pathfinding**: Async pathfinding jobs (don't block main thread)

## Module Breakdown

### 1. **Simulation Core** (`src/simulation.rs`)
- Fixed timestep update loop
- ECS world management
- Event queue (probe destroyed, resource depleted, tech unlocked)

### 2. **Entity Components** (`src/components.rs`)
```rust
struct Position { x: f64, y: f64 }
struct Velocity { dx: f64, dy: f64 }
struct ProbeType(ProbeVariant)
struct Inventory { iron: u32, silicon: u32, ... }
struct TaskQueue(VecDeque<Task>)
struct TechLevel { propulsion: u8, mining: u8, ... }
```

### 3. **Systems** (`src/systems/*.rs`)
- `movement_system`: Update positions based on velocity
- `mining_system`: Extract resources from asteroids
- `construction_system`: Build new probes
- `research_system`: Unlock tech nodes
- `automation_system`: AI executes high-level goals

### 4. **Spatial Index** (`src/spatial.rs`)
```rust
struct HashGrid {
    cells: HashMap<(i32, i32), Vec<Entity>>,
    cell_size: f64,
}
impl HashGrid {
    fn insert(&mut self, entity: Entity, pos: Position) { ... }
    fn nearby(&self, pos: Position, radius: f64) -> Vec<Entity> { ... }
}
```

### 5. **Galaxy Generation** (`src/galaxy.rs`)
- Procedural star system placement (Poisson disk sampling)
- Resource distribution (perlin noise for realism)
- Seed-based reproducibility

### 6. **Pathfinding** (`src/pathfinding.rs`)
- Star system graph (petgraph crate)
- Hierarchical A* implementation
- Task queue for async pathfinding

### 7. **Resource Economy** (`src/resources.rs`)
```rust
enum Resource { Iron, Silicon, Uranium, RareEarths }
enum RefinedResource { Alloy, Circuit, Fuel }
struct Recipe {
    inputs: HashMap<Resource, u32>,
    outputs: HashMap<RefinedResource, u32>,
    time: u32,
}
```

### 8. **Technology Tree** (`src/tech.rs`)
```rust
struct TechNode {
    id: TechId,
    name: &'static str,
    cost: u32, // research points
    prerequisites: Vec<TechId>,
    effect: TechEffect, // unlock new probe type, improve stat, etc.
}
```

### 9. **Automation AI** (`src/automation.rs`)
- **Fleet AI**: Group movement, resource pooling
- **Sector AI**: High-level goal planning (GOAP or utility-based)
- Override system (player takes manual control)

### 10. **Rendering** (`src/render.rs`)
- Instanced rendering for probes (one draw call for 10k triangles)
- Camera zoom levels (local → system → galaxy)
- Particle effects (thrust trails, mining beams)

### 11. **UI** (`src/ui.rs`)
- egui panels: probe inspector, tech tree, resource dashboard
- Galaxy map overlay (territory, resource flows)
- Event log (critical events: probe lost, tech unlocked)

## Performance Targets

| Metric | Target | Strategy |
|--------|--------|----------|
| Probes on-screen | 10k+ | Instanced rendering, ECS batching |
| Simulation FPS | 60 TPS | Parallel systems, spatial partitioning |
| Pathfinding latency | <16ms | Hierarchical A*, async jobs |
| Memory usage | <2GB @ 10k probes | Packed component storage |

## Data Flow Example: "Build a Probe"

1. **Player**: Clicks "Build Scout" on Constructor probe
2. **UI System**: Adds `Task::Build(ProbeType::Scout)` to TaskQueue component
3. **Construction System**:
   - Checks Inventory for required resources
   - Deducts resources if available
   - Spawns timer entity with `BuildInProgress` component
4. **Timer System**: Decrements timer each tick
5. **On completion**:
   - Spawns new entity with Scout components (Position, Velocity, ProbeType, etc.)
   - Emits `ProbeBuilt` event
6. **Automation System** (if enabled): Assigns new scout to exploration task queue

## Milestones

### **v0.1** — Prototype (2 weeks)
- [ ] ECS setup (bevy_ecs integrated)
- [ ] Basic rendering (wgpu triangle rendering)
- [ ] 1 probe moves with WASD
- [ ] Simple asteroid field (static entities)
- [ ] Mining works (click asteroid → gain resources)

### **v0.2** — Core Loop (4 weeks)
- [ ] Build system (queue probes, consume resources)
- [ ] Tech tree (3 starter techs)
- [ ] Galaxy generation (50 star systems)
- [ ] Fleet movement (select multiple probes)
- [ ] Save/load (serde serialization)

### **v0.3** — Automation (6 weeks)
- [ ] Spatial partitioning (hash grid)
- [ ] Fleet AI (group tasks, rally points)
- [ ] Sector automation (high-level goals)
- [ ] Performance: 1000 probes @ 60 FPS

### **v0.4** — Polish (8 weeks)
- [ ] UI overhaul (egui dashboards)
- [ ] Win conditions
- [ ] Balancing (resource curves, tech costs)
- [ ] Tutorial/documentation
- [ ] 10k probe stress test

### **v1.0** — Release
- [ ] Steam page / itch.io
- [ ] Modding support (JSON tech trees, custom probes)
- [ ] Achievements
- [ ] Post-launch: multiplayer (v1.1?)

## Open Technical Questions

1. **2D or 3D?** → Start 2D for faster prototyping
2. **Fixed-point vs f64?** → f64 for now, profile later
3. **Networking architecture?** → Defer to v1.1, focus on determinism now
4. **Audio?** → Not critical for v0.1, add in v0.3

---

**Next Steps**: 
1. Scaffold Rust project (`cargo new --lib`)
2. Add dependencies: `bevy_ecs`, `wgpu`, `egui`, `rand`, `serde`
3. Spike: Render 100 moving triangles (ECS + wgpu)
