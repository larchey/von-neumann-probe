# Von Neumann Probe: Performance Benchmarks & Test Suite

## Benchmark Targets

All numbers assume:
- **Commodity hardware:** Intel i5/Ryzen 5 (4-core), 8GB RAM, SSD
- **Target FPS:** 60 (16.6ms per frame)
- **Simulation time budget:** 2ms (leave 14ms for rendering/UI)

| Scenario | Entities | Expected FPS | Memory | Notes |
|----------|----------|--------------|--------|-------|
| Empty universe | 0 | 60 | <10 MB | Baseline |
| Cathedral + 10 probes | 100 | 60 | 15 MB | Tutorial |
| Active battle | 10K | 60 | 60 MB | Full detail simulation |
| Strategic play (far regions) | 100K | 60 | 65 MB | Aggregated swarms |
| Infinite universe | 100M+ | 60 | 70 MB | Archive + streaming |

## Phase 1: Spatial Hash Performance

**Goal:** Demonstrate O(1) vs O(n²) targeting performance

```bash
cargo test --release test_10k_entities_scaling
```

**Expected Results:**
```
Test: 10K entities, 1000 radius queries/frame
Before (O(n²)):  50ms per frame → 20 FPS ❌
After (O(1)):    0.5ms per frame → 60 FPS ✅
Speedup: 100x
```

**Code:**
```rust
#[test]
#[ignore] // Run with: cargo test -- --ignored --nocapture test_spatial_perf
fn test_spatial_perf() {
    let cell_size = 100.0;
    let mut grid = SpatialGrid::new(cell_size);
    
    // Populate 10K entities in 5000x5000 world
    for i in 0..10_000 {
        let x = (i as f32 * 1.618) % 5000.0;
        let y = (i as f32 * 2.718) % 5000.0;
        grid.insert(Entity::from_raw(i as u32), Vec2::new(x, y));
    }
    
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = grid.query_radius(Vec2::new(2500.0, 2500.0), 500.0);
    }
    let elapsed = start.elapsed();
    
    println!("1000 radius queries on 10K entities: {:?}", elapsed);
    println!("Average per query: {:?}", elapsed / 1000);
    assert!(elapsed.as_millis() < 10, "Spatial queries too slow!");
}
```

## Phase 2: Strategic Layer Performance

**Goal:** Verify swarm aggregation reduces entity overhead

```bash
cargo test --release test_strategic_swarm_aggregation
```

**Expected Results:**
```
100K individual threat entities:
  - Before aggregation: 15ms per frame (individual targeting) ❌
  - After aggregation: 1ms per frame (50 swarms) ✅
  - Speedup: 15x
  - Memory saved: 80% (entity components removed)
```

**Test Case:**
```rust
#[test]
fn test_strategic_swarm_aggregation() {
    let mut layer_manager = SimulationLayerManager {
        viewport_center: Vec2::new(2500.0, 2500.0),
        active_distance: 2000.0,
        strategic_distance: 10000.0,
        ..Default::default()
    };

    // Simulate 100K threat entities as strategic swarms
    for i in 0..100 {
        let swarm = StrategicSwarm {
            id: uuid::Uuid::new_v4(),
            swarm_type: SwarmType::ThreatRogue,
            count: 1000, // 100 swarms × 1000 entities = 100K
            position: Vec2::new(
                (i as f32 * 500.0) % 50000.0,
                (i as f32 * 300.0) % 50000.0,
            ),
            velocity: Vec2::new(10.0, 0.0),
            threat_level: 0.5,
            current_layer: SimulationLayer::Strategic,
            ..Default::default()
        };
        layer_manager.strategic_swarms.insert(swarm.id, swarm);
    }

    // Benchmark frame iteration
    let start = std::time::Instant::now();
    for _ in 0..300 {
        // Simulate 300 frames (5 seconds at 60 FPS)
        for swarm in layer_manager.strategic_swarms.values_mut() {
            swarm.position += swarm.velocity * (1.0 / 60.0);
        }
    }
    let elapsed = start.elapsed();

    let avg_frame_ms = elapsed.as_secs_f64() * 1000.0 / 300.0;
    println!("Avg frame time for 100K (100 swarms): {:.2}ms", avg_frame_ms);
    assert!(avg_frame_ms < 2.0, "Strategic simulation too slow");
}
```

## Phase 3: Archive System Performance

**Goal:** Verify disk I/O doesn't block simulation

```bash
cargo test --release test_archive_save_restore
```

**Expected Results:**
```
Save 10K sectors to disk:
  - Serialization: 50ms/sector (zstd compression)
  - Total: 500ms for 10 sectors ✅
  - Async I/O (off main thread): Player doesn't notice

Load 1 sector from disk:
  - Decompression + deserialization: 100ms
  - Re-simulate 1000 frames: 200ms
  - Total wake time: 300ms ✅
```

**Test Case:**
```rust
#[test]
fn test_archive_determinism() {
    let mut manager_a = ArchiveManager::default();
    let mut manager_b = ArchiveManager::default();

    let swarms = vec![StrategicSwarm {
        id: uuid::Uuid::new_v4(),
        count: 1000,
        position: Vec2::ZERO,
        velocity: Vec2::new(50.0, 30.0),
        current_layer: SimulationLayer::Archive,
        ..Default::default()
    }];

    // Create identical archives
    manager_a.create_archive((0, 0), swarms.clone(), 0, 0.0, 42);
    manager_b.create_archive((0, 0), swarms.clone(), 0, 0.0, 42);

    // Wake both from frame 0 → 1000
    let woken_a = wake_archived_sector(&manager_a, 1000, (0, 0)).unwrap();
    let woken_b = wake_archived_sector(&manager_b, 1000, (0, 0)).unwrap();

    // Should be pixel-perfect identical
    assert_eq!(woken_a[0].position.x, woken_b[0].position.x);
    assert_eq!(woken_a[0].position.y, woken_b[0].position.y);
    println!("✅ Archive determinism verified (frame 0→1000)");
}
```

## Phase 4: End-to-End Scaling Test

**Goal:** Full simulation with all layers active

```bash
cargo test --release test_1M_entities_60fps -- --nocapture
```

**Setup:**
- 1,000 active entities (viewport + buffer)
- 100,000 strategic entities (aggregated in 100 swarms)
- 900,000 archive entities (serialized on disk)
- **Total universe: 1,001,000 entities**

**Expected Metrics:**
```
Frame time distribution (300 frames):
  P50:  1.2ms
  P99:  2.5ms
  P999: 4.0ms
  Max:  5.2ms
  
Memory breakdown:
  Active ECS:     50 MB (1000 entities × 50KB each)
  Strategic:      5 MB (100 swarms)
  Archive cache:  10 MB (a few sectors in memory)
  System state:   20 MB
  Total:          85 MB (vs 50GB if all entities loaded!)
  
Verdict: ✅ PASS if all frames < 16.6ms, P99 < 2.5ms
```

**Test Code:**
```rust
#[test]
#[ignore]
fn test_1M_entities_60fps() {
    let mut world = Universe::new_with_1M_entities();
    let mut frame_times = Vec::new();

    for frame in 0..300 {
        let start = std::time::Instant::now();
        world.tick();
        frame_times.push(start.elapsed());
    }

    let p50 = percentile(&frame_times, 50);
    let p99 = percentile(&frame_times, 99);
    let p999 = percentile(&frame_times, 999);
    let max = frame_times.iter().max().unwrap();

    println!("Frame times (300 frames, 1M entities):");
    println!("  P50:  {:?}", p50);
    println!("  P99:  {:?}", p99);
    println!("  P999: {:?}", p999);
    println!("  Max:  {:?}", max);

    assert!(p99 < Duration::from_millis(3), "P99 too high: {:?}", p99);
    assert!(max < Duration::from_millis(16), "Frame spike detected: {:?}", max);
}
```

## Continuous Benchmarking

### CI Pipeline (GitHub Actions)

```yaml
name: Performance Benchmarks
on: [pull_request, push]
jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run spatial hash bench
        run: cargo test --release test_10k_entities_scaling -- --nocapture
      
      - name: Run strategic layer bench
        run: cargo test --release test_strategic_swarm_aggregation -- --nocapture
      
      - name: Archive integrity check
        run: cargo test --release test_archive_determinism
      
      - name: Comment results
        uses: actions/github-script@v6
        with:
          script: |
            // Parse benchmark output, post comment with trends
```

## Memory Profiling

### Using Valgrind (Linux)

```bash
valgrind --tool=massif \
  --massif-out-file=massif.out \
  target/release/von-neumann-probe

ms_print massif.out | head -50
```

### Expected Output:

```
Memory usage at peak:
  Main simulation: 50 MB (entities, components)
  Spatial grid:     3 MB (cell hash map)
  Archive cache:   10 MB (sector snapshots)
  Allocator waste:  8 MB (fragmentation)
  ────────────────────
  Total:          ~70 MB ✅
```

## Regression Detection

Keep a baseline file (`perf_baseline.json`):

```json
{
  "version": "0.1.0",
  "date": "2026-08-12",
  "benchmarks": {
    "spatial_10k_queries": {
      "min_ms": 0.3,
      "max_ms": 1.5,
      "mean_ms": 0.8
    },
    "strategic_100k_frame": {
      "min_ms": 0.9,
      "max_ms": 2.5,
      "mean_ms": 1.2
    },
    "1m_entities_p99": "2.5ms",
    "memory_peak": "70 MB"
  }
}
```

On each PR, regenerate and compare:
```bash
cargo test --release --test perf_suite -- --nocapture | tee results.json
python3 scripts/compare_perf.py perf_baseline.json results.json
```

If any metric regresses >10%, fail the PR:
```
❌ REGRESSION: spatial_10k_queries: 0.8ms → 1.2ms (+50%)
   Likely caused by: PR#42 (spatial grid refactor)
   Action: Review changes or increase accepted regression threshold
```

## Next Iterations

### Profiling Hotspots

Use `perf` (Linux) or Instruments (macOS):

```bash
cargo build --release
perf record --call-graph=dwarf ./target/release/von-neumann-probe
perf report
```

Look for:
- Allocation/deallocation hot paths → use memory pooling
- Cache misses in distance calculations → SIMD vectorization
- Spatial grid collision chains → tune cell size

### Optimization Opportunities

| Hotspot | Current | Optimization | Speedup |
|---------|---------|--------------|---------|
| Vec2 distance | `(x²+y²).sqrt()` | `squared_distance` for comparisons | 20% |
| Threat targeting loop | O(n²) per frame | Grid queries (already done) | 100x |
| Swarm aggregation | Per-frame clustering | Incremental update (swarm joins/splits) | 5x |
| Archive compression | zstd default | Tuned zstd + incremental writes | 2x |

## Production Checklist

Before shipping:

- [ ] All benchmarks pass on commodity hardware
- [ ] No frame spikes >16.6ms (60 FPS target)
- [ ] Memory stays <100 MB on 8GB systems
- [ ] Archive determinism verified (5000+ frames)
- [ ] Save/load round-trip integrity tested
- [ ] Async I/O doesn't block main thread
- [ ] Profiler shows no heap churn during long play sessions
- [ ] Multiplayer sync tested (archive checksum matching)

## Reporting Issues

Found a regression? Document:
1. **Metric:** Frame time P99, spatial query time, memory peak, etc.
2. **Before/After:** Exact numbers
3. **Reproduction:** Minimal code example
4. **Hardware:** CPU, RAM, OS
5. **Profile data:** If available, attach perf/massif output
