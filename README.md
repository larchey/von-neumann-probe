# Von Neumann Probe

A grand strategy game about self-replicating space probes conquering the galaxy through exponential growth, automation, and emergent complexity.

## Overview

Start with a single probe and expand across the galaxy by:
- Mining asteroids and planets for resources
- Building copies of yourself (self-replication)
- Researching technologies to unlock new capabilities
- Specializing probes for different tasks (scouts, miners, constructors, researchers, warriors)
- Automating fleets and sectors as you scale beyond manual control

## Current Features

- **Self-Replication**: Probes consume resources to build copies
- **Probe Specialization**: Scout, Miner, Constructor, Researcher, Warrior types
- **Resource Economy**: Minerals, Computronium, Exotic Matter
- **ECS Architecture**: Built on Bevy ECS for scalability (target: 10k+ entities)
- **Basic Combat**: Warrior probes defend territory

## Tech Stack

- **Rust** — Performance + safety for complex simulation
- **Bevy ECS** — Data-oriented entity-component-system
- **wgpu** — Modern GPU rendering (Vulkan/Metal/DX12)
- **glam** — Fast vector math

## Building & Running

```bash
cargo build --release
cargo run --release
```

## Development Roadmap

See [ENGINE_DESIGN.md](ENGINE_DESIGN.md) for technical architecture and [GAME_DESIGN.md](GAME_DESIGN.md) for gameplay mechanics.

### v0.1 — Prototype (Current)
- [x] ECS foundation
- [x] Basic probe types
- [x] Self-replication mechanics
- [x] Resource gathering
- [ ] Simple rendering
- [ ] Asteroid fields

### v0.2 — Core Loop
- [ ] Tech tree system
- [ ] Galaxy generation (50+ systems)
- [ ] Fleet movement
- [ ] Save/load

### v0.3 — Automation
- [ ] Spatial partitioning (hash grid)
- [ ] Fleet AI
- [ ] Sector automation
- [ ] 1000 probes @ 60 FPS

### v0.4 — Polish
- [ ] UI dashboards
- [ ] Win conditions
- [ ] Tutorial
- [ ] 10k probe stress test

### v1.0 — Release
- [ ] Steam/itch.io launch
- [ ] Modding support
- [ ] Achievements

## License

MIT
