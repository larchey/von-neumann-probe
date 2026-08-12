# Changelog

All notable changes to the Von Neumann Probe game engine will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-08-12

### Added

#### Core Architecture
- ECS-based game engine using Bevy 0.15
- Component system for Probes, Structures, and Combat entities
- Modular system design (movement, mining, replication, combat, rendering)
- Global game state tracking (probe count, resources, tech level)

#### Probe System
- Six probe types: Scout, Miner, Constructor, Researcher, Warrior, Administrator
- Probe specialization levels affecting speed and efficiency
- Energy system with per-action drain rates
- Resource carrying capacity and management
- Target-seeking movement with collision-free pathfinding prep

#### Replication System
- Self-replication mechanics consuming minerals and computronium
- Progressive replication with visual feedback
- Random probe type generation with weighted distributions
- Scalable colony size management (configurable up to 10K probes)

#### Mining & Resources
- Three resource types: Minerals, Computronium, Exotic Matter
- Mining-specific probe behavior with energy management
- Resource accumulation in colony pool
- Structure-based resource refinement (prepared)

#### Combat System
- Warrior probe combat with attack cooldowns
- Projectile spawning and lifetime management
- Attack power and threat level tracking
- Combat entity components for flexible extensibility

#### World Generation
- Cathedral structure generation with ring-based layouts
- Procedural sector generation (infinite universe)
- Cathedral type specialization (Computational, Manufacturing, Agricultural, Military, Scientific, Hybrid)
- Spire systems for visual hierarchy
- Seeded RNG for deterministic procedural generation

#### Rendering
- Minimalist 2D top-down aesthetic
- Color-coded probe types and structures
- Camera following system
- 8x8 pixel sprite basis (extensible to larger assets)
- Sprite-based particle effect infrastructure

#### Physics & Spatial
- Spatial grid hash-based partitioning for O(n) queries
- Quadtree implementation for hierarchical spatial queries
- Bounding box and circle collision detection
- Distance calculation for proximity detection

#### Configuration
- Game configuration system (probe speed, mining rate, costs)
- Extensible resource definitions
- Customizable replication costs and probe parameters

#### Serialization
- Serde integration for save/load support (infrastructure)
- Probe, structure, and game state serialization types
- JSON serialization ready for save files

### Infrastructure

- Cargo.toml with optimized build profiles (LTO in release)
- Modular source code organization
- Comprehensive README with gameplay overview and technical design
- Extensible architecture for adding new probe types and structures

### Known Limitations

- Projectiles don't affect probes yet (combat UI only)
- No persistence system (save/load skeleton only)
- No tech tree implementation
- Limited to local single-player
- No sound or advanced graphics yet
- Combat balance not tuned
- Mining rate is static (no scaling with tech)

---

## Future Versions

### [0.2.0] - Combat Polish
- Projectile collision and damage application
- Combat balance tuning
- Threat escalation mechanics
- Warrior probe AI behavior trees

### [0.3.0] - Tech Tree
- Technology progression system
- Cost-based unlocking of probe types
- Structure tier progression
- Tech-gated resource types

### [0.4.0] - Persistence
- Save/load system with compression
- Game state serialization
- Replay recording
- Cloud sync (future)

### [1.0.0] - Full Release
- Multi-threaded simulation
- Networking/multiplayer foundation
- Advanced graphics and effects
- Music and sound design
- Campaign/mission framework
