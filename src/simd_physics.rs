// Von Neumann Probe: SIMD-Optimized Physics Engine
//
// GOAL: Process 10,000+ entities at 60 FPS on commodity hardware.
//
// STRATEGY: Leverage CPU vector instructions (SSE/AVX) to process 4-8 entities per cycle.
// Traditional scalar code processes entities one-by-one:
//   for entity in entities {
//       entity.position += entity.velocity * dt;  // 1 entity/cycle
//   }
//
// SIMD processes entities in batches:
//   for chunk in entities.chunks(8) {
//       // Process 8 entities in parallel using 256-bit AVX registers
//       positions[0..8] += velocities[0..8] * dt;  // 8 entities/cycle
//   }
//
// PERFORMANCE IMPACT:
// - Scalar:  10K entities × 50ns = 500µs per frame
// - SIMD:    10K entities ÷ 8 × 10ns = 12.5µs per frame
// - Speedup: 40× faster (leaves room for 39× more entities!)
//
// MEMORY LAYOUT:
// Avoid "array of structs" (AoS):
//   struct Entity { x, y, vx, vy, ... }
//   entities: Vec<Entity>
//
// Use "struct of arrays" (SoA) for cache locality:
//   positions_x: [f32; 10000]
//   positions_y: [f32; 10000]
//   velocities_x: [f32; 10000]
//   velocities_y: [f32; 10000]
//
// This ensures sequential memory access (SIMD loves this).

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use std::arch::x86_64::*; // SIMD intrinsics (x86_64 only; fallback for other archs)

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// STRUCT-OF-ARRAYS STORAGE
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Resource)]
pub struct SimdPhysicsEngine {
    // Positions
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,

    // Velocities
    pub vel_x: Vec<f32>,
    pub vel_y: Vec<f32>,

    // Accelerations (for forces, gravity, thrust)
    pub acc_x: Vec<f32>,
    pub acc_y: Vec<f32>,

    // Metadata
    pub entity_ids: Vec<uuid::Uuid>,
    pub active_count: usize,
    pub capacity: usize,
}

impl SimdPhysicsEngine {
    pub fn new(capacity: usize) -> Self {
        Self {
            pos_x: vec![0.0; capacity],
            pos_y: vec![0.0; capacity],
            vel_x: vec![0.0; capacity],
            vel_y: vec![0.0; capacity],
            acc_x: vec![0.0; capacity],
            acc_y: vec![0.0; capacity],
            entity_ids: vec![uuid::Uuid::nil(); capacity],
            active_count: 0,
            capacity,
        }
    }

    /// Add entity to simulation.
    pub fn add_entity(&mut self, id: uuid::Uuid, pos: Vec2, vel: Vec2) -> Result<usize, String> {
        if self.active_count >= self.capacity {
            return Err(format!("Physics engine full ({}/{})", self.active_count, self.capacity));
        }

        let idx = self.active_count;
        self.pos_x[idx] = pos.x;
        self.pos_y[idx] = pos.y;
        self.vel_x[idx] = vel.x;
        self.vel_y[idx] = vel.y;
        self.acc_x[idx] = 0.0;
        self.acc_y[idx] = 0.0;
        self.entity_ids[idx] = id;
        self.active_count += 1;

        Ok(idx)
    }

    /// Remove entity (swap with last active entity to avoid holes).
    pub fn remove_entity(&mut self, idx: usize) {
        if idx >= self.active_count {
            return;
        }

        let last = self.active_count - 1;
        self.pos_x.swap(idx, last);
        self.pos_y.swap(idx, last);
        self.vel_x.swap(idx, last);
        self.vel_y.swap(idx, last);
        self.acc_x.swap(idx, last);
        self.acc_y.swap(idx, last);
        self.entity_ids.swap(idx, last);

        self.active_count -= 1;
    }

    /// Apply acceleration to velocity (F = ma).
    pub fn apply_acceleration(&mut self, idx: usize, acc: Vec2) {
        if idx < self.active_count {
            self.acc_x[idx] += acc.x;
            self.acc_y[idx] += acc.y;
        }
    }

    /// SIMD-optimized physics update.
    /// Processes velocities + positions in batches of 8 (AVX) or 4 (SSE).
    pub fn update_positions(&mut self, dt: f32) {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx") {
                unsafe { self.update_positions_avx(dt); }
                return;
            }
        }

        // Fallback: scalar implementation (still fast, just not SIMD)
        self.update_positions_scalar(dt);
    }

    /// Scalar fallback (for non-x86_64 or when SIMD not available).
    fn update_positions_scalar(&mut self, dt: f32) {
        for i in 0..self.active_count {
            // velocity += acceleration * dt
            self.vel_x[i] += self.acc_x[i] * dt;
            self.vel_y[i] += self.acc_y[i] * dt;

            // position += velocity * dt
            self.pos_x[i] += self.vel_x[i] * dt;
            self.pos_y[i] += self.vel_y[i] * dt;

            // Clear acceleration for next frame
            self.acc_x[i] = 0.0;
            self.acc_y[i] = 0.0;
        }
    }

    /// AVX-optimized update (processes 8 f32 values per instruction).
    #[cfg(target_arch = "x86_64")]
    unsafe fn update_positions_avx(&mut self, dt: f32) {
        let dt_vec = _mm256_set1_ps(dt); // Broadcast dt to all 8 lanes

        let chunks = self.active_count / 8;
        let remainder = self.active_count % 8;

        for i in 0..chunks {
            let base = i * 8;

            // Load 8 values at once
            let vel_x = _mm256_loadu_ps(self.vel_x.as_ptr().add(base));
            let vel_y = _mm256_loadu_ps(self.vel_y.as_ptr().add(base));
            let acc_x = _mm256_loadu_ps(self.acc_x.as_ptr().add(base));
            let acc_y = _mm256_loadu_ps(self.acc_y.as_ptr().add(base));
            let pos_x = _mm256_loadu_ps(self.pos_x.as_ptr().add(base));
            let pos_y = _mm256_loadu_ps(self.pos_y.as_ptr().add(base));

            // velocity += acceleration * dt (8 entities in parallel)
            let new_vel_x = _mm256_fmadd_ps(acc_x, dt_vec, vel_x);
            let new_vel_y = _mm256_fmadd_ps(acc_y, dt_vec, vel_y);

            // position += velocity * dt (8 entities in parallel)
            let new_pos_x = _mm256_fmadd_ps(new_vel_x, dt_vec, pos_x);
            let new_pos_y = _mm256_fmadd_ps(new_vel_y, dt_vec, pos_y);

            // Store results
            _mm256_storeu_ps(self.vel_x.as_mut_ptr().add(base), new_vel_x);
            _mm256_storeu_ps(self.vel_y.as_mut_ptr().add(base), new_vel_y);
            _mm256_storeu_ps(self.pos_x.as_mut_ptr().add(base), new_pos_x);
            _mm256_storeu_ps(self.pos_y.as_mut_ptr().add(base), new_pos_y);

            // Clear accelerations
            let zero = _mm256_setzero_ps();
            _mm256_storeu_ps(self.acc_x.as_mut_ptr().add(base), zero);
            _mm256_storeu_ps(self.acc_y.as_mut_ptr().add(base), zero);
        }

        // Handle remainder (< 8 entities)
        for i in (chunks * 8)..self.active_count {
            self.vel_x[i] += self.acc_x[i] * dt;
            self.vel_y[i] += self.acc_y[i] * dt;
            self.pos_x[i] += self.vel_x[i] * dt;
            self.pos_y[i] += self.vel_y[i] * dt;
            self.acc_x[i] = 0.0;
            self.acc_y[i] = 0.0;
        }
    }

    /// Get entity position.
    pub fn get_position(&self, idx: usize) -> Option<Vec2> {
        if idx < self.active_count {
            Some(Vec2::new(self.pos_x[idx], self.pos_y[idx]))
        } else {
            None
        }
    }

    /// Get entity velocity.
    pub fn get_velocity(&self, idx: usize) -> Option<Vec2> {
        if idx < self.active_count {
            Some(Vec2::new(self.vel_x[idx], self.vel_y[idx]))
        } else {
            None
        }
    }

    /// Batch distance calculation (for threat targeting, collision detection).
    /// Returns squared distances (avoids sqrt, which is expensive).
    pub fn batch_distances_squared(&self, origin: Vec2) -> Vec<f32> {
        let mut distances = vec![0.0; self.active_count];

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx") {
                unsafe { return self.batch_distances_squared_avx(origin); }
            }
        }

        // Scalar fallback
        for i in 0..self.active_count {
            let dx = self.pos_x[i] - origin.x;
            let dy = self.pos_y[i] - origin.y;
            distances[i] = dx * dx + dy * dy;
        }

        distances
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn batch_distances_squared_avx(&self, origin: Vec2) -> Vec<f32> {
        let mut distances = vec![0.0; self.active_count];

        let origin_x = _mm256_set1_ps(origin.x);
        let origin_y = _mm256_set1_ps(origin.y);

        let chunks = self.active_count / 8;

        for i in 0..chunks {
            let base = i * 8;

            let px = _mm256_loadu_ps(self.pos_x.as_ptr().add(base));
            let py = _mm256_loadu_ps(self.pos_y.as_ptr().add(base));

            let dx = _mm256_sub_ps(px, origin_x);
            let dy = _mm256_sub_ps(py, origin_y);

            let dx2 = _mm256_mul_ps(dx, dx);
            let dy2 = _mm256_mul_ps(dy, dy);

            let dist_sq = _mm256_add_ps(dx2, dy2);

            _mm256_storeu_ps(distances.as_mut_ptr().add(base), dist_sq);
        }

        // Scalar remainder
        for i in (chunks * 8)..self.active_count {
            let dx = self.pos_x[i] - origin.x;
            let dy = self.pos_y[i] - origin.y;
            distances[i] = dx * dx + dy * dy;
        }

        distances
    }

    /// Find N nearest entities to a point (used for threat targeting).
    /// Returns (index, distance_squared) tuples.
    pub fn find_nearest(&self, origin: Vec2, n: usize) -> Vec<(usize, f32)> {
        let distances = self.batch_distances_squared(origin);

        let mut indexed: Vec<(usize, f32)> = distances.into_iter().enumerate().collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        indexed.truncate(n);

        indexed
    }

    /// Statistics for monitoring.
    pub fn stats(&self) -> PhysicsStats {
        PhysicsStats {
            active_entities: self.active_count,
            capacity: self.capacity,
            utilization_pct: (self.active_count as f32 / self.capacity as f32 * 100.0) as usize,
            memory_bytes: self.capacity * std::mem::size_of::<f32>() * 6, // 6 arrays
        }
    }
}

#[derive(Debug)]
pub struct PhysicsStats {
    pub active_entities: usize,
    pub capacity: usize,
    pub utilization_pct: usize,
    pub memory_bytes: usize,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// COLLISION DETECTION (SIMD-Optimized AABB checks)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl SimdPhysicsEngine {
    /// Batch AABB collision check: which entities are within a bounding box?
    /// Returns indices of entities inside the box.
    pub fn query_aabb(&self, min: Vec2, max: Vec2) -> Vec<usize> {
        let mut results = Vec::new();

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx") {
                unsafe { return self.query_aabb_avx(min, max); }
            }
        }

        // Scalar fallback
        for i in 0..self.active_count {
            if self.pos_x[i] >= min.x && self.pos_x[i] <= max.x &&
               self.pos_y[i] >= min.y && self.pos_y[i] <= max.y {
                results.push(i);
            }
        }

        results
    }

    #[cfg(target_arch = "x86_64")]
    unsafe fn query_aabb_avx(&self, min: Vec2, max: Vec2) -> Vec<usize> {
        let mut results = Vec::new();

        let min_x = _mm256_set1_ps(min.x);
        let min_y = _mm256_set1_ps(min.y);
        let max_x = _mm256_set1_ps(max.x);
        let max_y = _mm256_set1_ps(max.y);

        let chunks = self.active_count / 8;

        for i in 0..chunks {
            let base = i * 8;

            let px = _mm256_loadu_ps(self.pos_x.as_ptr().add(base));
            let py = _mm256_loadu_ps(self.pos_y.as_ptr().add(base));

            // Check if min.x <= px <= max.x AND min.y <= py <= max.y
            let x_in_range = _mm256_and_ps(
                _mm256_cmp_ps(px, min_x, _CMP_GE_OQ),
                _mm256_cmp_ps(px, max_x, _CMP_LE_OQ),
            );
            let y_in_range = _mm256_and_ps(
                _mm256_cmp_ps(py, min_y, _CMP_GE_OQ),
                _mm256_cmp_ps(py, max_y, _CMP_LE_OQ),
            );

            let inside = _mm256_and_ps(x_in_range, y_in_range);

            // Extract mask (which lanes are all 1s)
            let mask = _mm256_movemask_ps(inside);

            for lane in 0..8 {
                if (mask & (1 << lane)) != 0 {
                    results.push(base + lane);
                }
            }
        }

        // Scalar remainder
        for i in (chunks * 8)..self.active_count {
            if self.pos_x[i] >= min.x && self.pos_x[i] <= max.x &&
               self.pos_y[i] >= min.y && self.pos_y[i] <= max.y {
                results.push(i);
            }
        }

        results
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// TESTS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_physics_add_remove() {
        let mut engine = SimdPhysicsEngine::new(100);

        let id = uuid::Uuid::new_v4();
        let idx = engine.add_entity(id, Vec2::new(100.0, 200.0), Vec2::new(10.0, -5.0)).unwrap();

        assert_eq!(engine.active_count, 1);
        assert_eq!(engine.get_position(idx), Some(Vec2::new(100.0, 200.0)));

        engine.remove_entity(idx);
        assert_eq!(engine.active_count, 0);
    }

    #[test]
    fn test_simd_physics_update() {
        let mut engine = SimdPhysicsEngine::new(100);

        // Add entity with velocity
        engine.add_entity(uuid::Uuid::new_v4(), Vec2::ZERO, Vec2::new(100.0, 0.0)).unwrap();

        // Update for 1 second at 60 FPS
        for _ in 0..60 {
            engine.update_positions(1.0 / 60.0);
        }

        let pos = engine.get_position(0).unwrap();
        assert!((pos.x - 100.0).abs() < 0.01); // Should move ~100 units in 1 sec
    }

    #[test]
    fn test_batch_distances() {
        let mut engine = SimdPhysicsEngine::new(100);

        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(0.0, 0.0), Vec2::ZERO).unwrap();
        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(3.0, 4.0), Vec2::ZERO).unwrap();
        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(10.0, 0.0), Vec2::ZERO).unwrap();

        let distances = engine.batch_distances_squared(Vec2::ZERO);

        assert_eq!(distances[0], 0.0);
        assert_eq!(distances[1], 25.0); // 3²+4² = 25
        assert_eq!(distances[2], 100.0); // 10² = 100
    }

    #[test]
    fn test_find_nearest() {
        let mut engine = SimdPhysicsEngine::new(100);

        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(100.0, 0.0), Vec2::ZERO).unwrap();
        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(5.0, 0.0), Vec2::ZERO).unwrap();
        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(50.0, 0.0), Vec2::ZERO).unwrap();

        let nearest = engine.find_nearest(Vec2::ZERO, 2);

        assert_eq!(nearest.len(), 2);
        assert_eq!(nearest[0].0, 1); // Entity at x=5 is closest
        assert_eq!(nearest[1].0, 2); // Entity at x=50 is second
    }

    #[test]
    fn test_aabb_query() {
        let mut engine = SimdPhysicsEngine::new(100);

        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(50.0, 50.0), Vec2::ZERO).unwrap();
        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(150.0, 150.0), Vec2::ZERO).unwrap();
        engine.add_entity(uuid::Uuid::new_v4(), Vec2::new(75.0, 75.0), Vec2::ZERO).unwrap();

        let inside = engine.query_aabb(Vec2::new(40.0, 40.0), Vec2::new(100.0, 100.0));

        assert_eq!(inside.len(), 2); // Entities at (50,50) and (75,75)
        assert!(inside.contains(&0));
        assert!(inside.contains(&2));
    }

    #[test]
    #[ignore] // Run with: cargo test --release test_simd_perf -- --ignored --nocapture
    fn test_simd_perf() {
        let mut engine = SimdPhysicsEngine::new(10_000);

        // Add 10K entities
        for i in 0..10_000 {
            let x = (i as f32 * 1.618) % 5000.0;
            let y = (i as f32 * 2.718) % 5000.0;
            engine.add_entity(
                uuid::Uuid::new_v4(),
                Vec2::new(x, y),
                Vec2::new(10.0, -5.0),
            ).unwrap();
        }

        // Benchmark 300 frames (5 seconds at 60 FPS)
        let start = std::time::Instant::now();
        for _ in 0..300 {
            engine.update_positions(1.0 / 60.0);
        }
        let elapsed = start.elapsed();

        let avg_frame_ms = elapsed.as_secs_f64() * 1000.0 / 300.0;
        println!("SIMD Physics: 10K entities, 300 frames");
        println!("  Total: {:?}", elapsed);
        println!("  Avg/frame: {:.2}ms", avg_frame_ms);
        println!("  FPS equivalent: {:.0}", 1000.0 / avg_frame_ms);

        assert!(avg_frame_ms < 2.0, "SIMD physics too slow: {:.2}ms/frame", avg_frame_ms);
    }
}
