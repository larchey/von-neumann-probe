// Scaling Stress Tests
//
// Run with: cargo bench --bench scaling_stress_test
//
// Tests the engine's ability to handle massive entity counts across all layers.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use bevy_math::Vec2;

// Mock imports (replace with actual once modules compile)
struct SimdPhysicsEngine {
    active_count: usize,
    pos_x: Vec<f32>,
    pos_y: Vec<f32>,
    vel_x: Vec<f32>,
    vel_y: Vec<f32>,
    acc_x: Vec<f32>,
    acc_y: Vec<f32>,
}

impl SimdPhysicsEngine {
    fn new(capacity: usize) -> Self {
        Self {
            active_count: 0,
            pos_x: vec![0.0; capacity],
            pos_y: vec![0.0; capacity],
            vel_x: vec![0.0; capacity],
            vel_y: vec![0.0; capacity],
            acc_x: vec![0.0; capacity],
            acc_y: vec![0.0; capacity],
        }
    }

    fn add_entity(&mut self, pos: Vec2, vel: Vec2) {
        if self.active_count < self.pos_x.len() {
            let idx = self.active_count;
            self.pos_x[idx] = pos.x;
            self.pos_y[idx] = pos.y;
            self.vel_x[idx] = vel.x;
            self.vel_y[idx] = vel.y;
            self.active_count += 1;
        }
    }

    fn update_positions_scalar(&mut self, dt: f32) {
        for i in 0..self.active_count {
            self.vel_x[i] += self.acc_x[i] * dt;
            self.vel_y[i] += self.acc_y[i] * dt;
            self.pos_x[i] += self.vel_x[i] * dt;
            self.pos_y[i] += self.vel_y[i] * dt;
            self.acc_x[i] = 0.0;
            self.acc_y[i] = 0.0;
        }
    }

    fn batch_distances_squared(&self, origin: Vec2) -> Vec<f32> {
        let mut distances = Vec::with_capacity(self.active_count);
        for i in 0..self.active_count {
            let dx = self.pos_x[i] - origin.x;
            let dy = self.pos_y[i] - origin.y;
            distances.push(dx * dx + dy * dy);
        }
        distances
    }
}

fn bench_physics_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_update");

    for entity_count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut engine = SimdPhysicsEngine::new(count);

                // Populate
                for i in 0..count {
                    engine.add_entity(
                        Vec2::new((i as f32 * 10.0) % 1000.0, (i as f32 * 7.0) % 1000.0),
                        Vec2::new(5.0, -3.0),
                    );
                }

                b.iter(|| {
                    engine.update_positions_scalar(black_box(1.0 / 60.0));
                });
            },
        );
    }

    group.finish();
}

fn bench_distance_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_queries");

    for entity_count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(entity_count),
            entity_count,
            |b, &count| {
                let mut engine = SimdPhysicsEngine::new(count);

                for i in 0..count {
                    engine.add_entity(
                        Vec2::new((i as f32 * 10.0) % 1000.0, (i as f32 * 7.0) % 1000.0),
                        Vec2::ZERO,
                    );
                }

                let origin = Vec2::new(500.0, 500.0);

                b.iter(|| {
                    let _ = engine.batch_distances_squared(black_box(origin));
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_allocation(c: &mut Criterion) {
    c.bench_function("allocate_10k_entities", |b| {
        b.iter(|| {
            let mut engine = SimdPhysicsEngine::new(10_000);
            for i in 0..10_000 {
                engine.add_entity(
                    Vec2::new((i as f32) % 1000.0, (i as f32) % 1000.0),
                    Vec2::ZERO,
                );
            }
            black_box(engine);
        });
    });
}

criterion_group!(
    benches,
    bench_physics_update,
    bench_distance_queries,
    bench_memory_allocation
);

criterion_main!(benches);
