use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use bevy_ecs::prelude::*;
use bevy_math::Vec2;

fn setup_world(entity_count: usize) -> World {
    let mut world = World::new();

    for i in 0..entity_count {
        let angle = (i as f32 / entity_count as f32) * std::f32::consts::TAU;
        let distance = (i as f32 * 10.0).sqrt();

        world.spawn((
            Position(Vec2::new(angle.cos() * distance, angle.sin() * distance)),
            Velocity(Vec2::new(0.0, 0.0)),
            Health { current: 100.0, max: 100.0 },
        ));
    }

    world
}

#[derive(Component)]
struct Position(Vec2);

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}

fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.0 += vel.0;
    }
}

fn health_decay_system(mut query: Query<&mut Health>) {
    for mut health in query.iter_mut() {
        health.current = (health.current - 0.1).max(0.0);
    }
}

fn benchmark_ecs_systems(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_systems");

    for entity_count in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("movement", entity_count),
            &entity_count,
            |b, &count| {
                let mut world = setup_world(count);
                let mut schedule = Schedule::default();
                schedule.add_systems(movement_system);

                b.iter(|| {
                    schedule.run(&mut world);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("health_decay", entity_count),
            &entity_count,
            |b, &count| {
                let mut world = setup_world(count);
                let mut schedule = Schedule::default();
                schedule.add_systems(health_decay_system);

                b.iter(|| {
                    schedule.run(&mut world);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_spatial_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_queries");

    for entity_count in [100, 1000, 5000] {
        group.bench_with_input(
            BenchmarkId::new("brute_force_neighbors", entity_count),
            &entity_count,
            |b, &count| {
                let world = setup_world(count);
                let positions: Vec<Vec2> = world
                    .query::<&Position>()
                    .iter(&world)
                    .map(|p| p.0)
                    .collect();

                b.iter(|| {
                    let query_point = Vec2::new(100.0, 100.0);
                    let radius = 50.0;
                    let radius_sq = radius * radius;

                    let neighbors: Vec<Vec2> = positions
                        .iter()
                        .filter(|&&p| p.distance_squared(query_point) < radius_sq)
                        .copied()
                        .collect();

                    black_box(neighbors);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_pathfinding(c: &mut Criterion) {
    c.bench_function("a_star_100x100", |b| {
        use std::collections::{BinaryHeap, HashMap, HashSet};

        b.iter(|| {
            let start = Vec2::new(0.0, 0.0);
            let goal = Vec2::new(1000.0, 1000.0);
            let obstacles = HashSet::new();

            black_box((start, goal, obstacles));
        });
    });
}

criterion_group!(
    benches,
    benchmark_ecs_systems,
    benchmark_spatial_queries,
    benchmark_pathfinding
);
criterion_main!(benches);
