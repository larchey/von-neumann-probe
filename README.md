# Von Neumann Probe Game

A 2D grand strategy game about self-replicating space probes conquering the galaxy through exponential growth, automation, and emergent complexity. Game Engine

A 2D RTS/sandbox game engine in Rust built with Bevy, featuring infinite scaling, procedural cathedral generation, self-replicating probes, and real-time combat mechanics.

## Core Features

### Game Mechanics
- **Self-Replication**: Probes create copies using collected resources (minerals, computronium)
- **Combat System**: Warrior probes engage in tactical battles with dynamic threat management
- **Expansion Puzzle**: Balance probe specialization, resource management, and territory control
- **Infinite Scaling**: Play from single probe to galaxy-spanning colonies

### World Generation
- **Cathedral Architecture**: Procedurally generated hexagonal cathedral structures with specialization types
- **Sector Generation**: Infinite procedurally generated sectors with varied layouts
- **Dynamic Expansion**: Ring-based growth system for scaling colonies

### Probe Types
- **Scout**: Fast movement, low resource consumption, exploration focused
- **Miner**: High mining rate, medium energy drain, resource gathering
- **Constructor**: Builds structures, medium speed, automation focused
- **Researcher**: Generates new tech, energy intensive, unlocks capabilities
- **Warrior**: Combat damage, attack patterns, threat defense
- **Administrator**: Command hierarchy, fleet coordination bonuses

### Structures
- **Factory Cathedral**: Mass production hubs (Hybrid focus)
- **Refinery**: Mineral processing
- **Foundry**: Computronium crafting
- **Laboratory**: Technology research
- **Power Plant**: Energy generation
- **Storage Bay**: Resource depot
- **Defense Turret**: Automated combat
- **Compute Node**: Intelligence and automation
- **Transmission Array**: Communication and coordination

## Architecture

### ECS (Entity Component System)
Built on Bevy ECS for high performance:
- **Probes** as entities with Components: Position, Type, Health, Energy, Resources
- **Structures** as static/semi-static entities with maintenance/production
- **Combat Entities** with attack patterns and cooldown systems
- **Spatial indexing** for O(log n) collision queries at 10K+ probes

### Procedural Generation
- **Perlin noise** based asteroid field generation
- **Seeded RNG** for deterministic sector generation
- **Cathedral layouts** using ring-based radial symmetry
- **Spire systems** for power distribution and visual hierarchy

### Rendering
- **2D Top-Down View**: Minimalist aesthetic with focus on gameplay clarity
- **Sprite-based**: Efficient rendering for 10K+ entities
- **Color coding**: Probe types and structure functions at a glance
- **Grid-based positioning**: Quantized movement for deterministic simulation

### Physics
- **Spatial Grid**: Hash-grid based spatial partitioning
- **Quadtree**: Optional recursive spatial index for large queries
- **Bounding Box collision**: Rectangle-based intersection detection
- **Circle collision**: Radial hit detection for combat/mining

## Building & Running

### Prerequisites
- Rust 1.70+ ([install](https://rustup.rs/))
- Cargo (comes with Rust)

### Compile
```bash
cargo build --release
```

### Run
```bash
cargo run --release
```

### Development
```bash
cargo run
# or with logging:
RUST_LOG=debug cargo run
```

## Gameplay Loop

1. **Early Game**: Single Constructor probe mines asteroid, collects resources
2. **Growth**: Probe replicates into Miners and Scouts to expand resource flow
3. **Specialization**: Build dedicated Researcher probes for tech advancement
4. **Colony**: Construct cathedral structures for production scaling
5. **Defense**: Deploy Warrior probes as threat level increases
6. **Expansion**: Establish new colonies in nearby sectors
7. **Infinity**: Auto-scale gameplay to handle 100K+ probes

## Extensibility

Designed for easy feature addition:
- **New Probe Types**: Add struct + color mapping + behavior systems
- **New Structures**: Add StructureType + generation logic + upgrade paths
- **Tech Tree**: Resource costs and unlock trees (prepared but not yet implemented)
- **Anomalies**: Procedural dangerous zones (nebulae, black holes, storms)
- **Multiplayer**: Save/load serialization + deterministic RNG ready

## Performance Targets

- **60 FPS** at 1600x900 with 10K probes
- **120 FPS** at 1K probes (comfortable mid-game)
- **Frame-time < 16ms** for interactive camera panning
- **Memory < 2GB** for million-entity simulation

## Code Structure

```
src/
├── main.rs          # App setup, camera, initial spawning
├── components.rs    # ECS component definitions
├── systems.rs       # Game logic (movement, mining, combat, etc.)
├── resources.rs     # Global game state and configuration
├── generation.rs    # Procedural world generation (cathedrals, sectors)
├── physics.rs       # Spatial indexing and collision detection
└── rendering.rs     # Rendering systems and visual themes
```

## Future Roadmap

- [ ] Tech tree system with resource gates
- [ ] Campaign-style mission objectives
- [ ] Multi-threaded simulation for 100K+ probes
- [ ] Networking for cooperative/competitive multiplayer
- [ ] Replay system (deterministic RNG + state snapshots)
- [ ] Custom probe programming/AI scripting
- [ ] Advanced graphics with shader effects
- [ ] Sound design and music integration

## License

MIT (or your preference)

## Build Status

Current Version: **0.1.0 - Pre-Alpha**
- Core ECS architecture: ✅
- Probe spawning & replication: ✅
- Mining system: ✅
- Combat basics: ⚠️ (wired, needs polish)
- Cathedral generation: ✅
- Sector generation: ✅
- Camera system: ✅
- Save/load: 🔄 (infrastructure ready)

---

*Built for infinite scaling. Designed for endless strategy.*
