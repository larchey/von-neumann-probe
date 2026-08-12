use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use rand::Rng;

#[derive(Clone, Copy)]
pub struct RGB {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub lifetime: f32,
    pub elapsed: f32,
    pub color: RGB,
    pub size: f32,
    pub fade: bool,
}

#[derive(Component)]
pub struct ParticlePosition(pub Vec2);

#[derive(Resource)]
pub struct ParticleSystem {
    pub active_count: usize,
    pub max_particles: usize,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            active_count: 0,
            max_particles: 10000,
        }
    }
}

pub enum ParticleEffect {
    Explosion,
    MiningSparkle,
    WarpTrail,
    ConstructionDust,
    CombatHit,
    ThrusterFlame,
}

pub fn spawn_particle_burst(
    commands: &mut Commands,
    position: Vec2,
    effect: ParticleEffect,
    particle_system: &mut ParticleSystem,
) {
    let (count, lifetime, speed_range, color, size) = match effect {
        ParticleEffect::Explosion => (
            50,
            1.5,
            (100.0, 300.0),
            RGB { r: 1.0, g: 0.5, b: 0.0 },
            3.0,
        ),
        ParticleEffect::MiningSparkle => (
            15,
            0.8,
            (30.0, 80.0),
            RGB { r: 0.8, g: 0.8, b: 0.2 },
            2.0,
        ),
        ParticleEffect::WarpTrail => (
            8,
            0.5,
            (20.0, 50.0),
            RGB { r: 0.3, g: 0.8, b: 1.0 },
            1.5,
        ),
        ParticleEffect::ConstructionDust => (
            20,
            1.2,
            (40.0, 90.0),
            RGB { r: 0.5, g: 0.5, b: 0.5 },
            2.5,
        ),
        ParticleEffect::CombatHit => (
            30,
            0.6,
            (80.0, 200.0),
            RGB { r: 1.0, g: 0.2, b: 0.2 },
            2.0,
        ),
        ParticleEffect::ThrusterFlame => (
            5,
            0.3,
            (10.0, 30.0),
            RGB { r: 0.2, g: 0.5, b: 1.0 },
            1.0,
        ),
    };

    if particle_system.active_count + count > particle_system.max_particles {
        return;
    }

    let mut rng = rand::thread_rng();

    for _ in 0..count {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(speed_range.0..speed_range.1);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;

        commands.spawn((
            Particle {
                velocity,
                lifetime,
                elapsed: 0.0,
                color,
                size,
                fade: true,
            },
            ParticlePosition(position),
        ));

        particle_system.active_count += 1;
    }
}

pub fn update_particles(
    mut commands: Commands,
    mut particle_system: ResMut<ParticleSystem>,
    mut query: Query<(Entity, &mut ParticlePosition, &mut Particle)>,
    time: Res<crate::resources::GameTime>,
) {
    let dt = time.delta_secs;

    for (entity, mut pos, mut particle) in query.iter_mut() {
        particle.elapsed += dt;

        if particle.elapsed >= particle.lifetime {
            commands.entity(entity).despawn();
            particle_system.active_count = particle_system.active_count.saturating_sub(1);
            continue;
        }

        pos.0 += particle.velocity * dt;
        particle.velocity *= 0.98;
    }
}

pub fn particle_cleanup_system(
    mut commands: Commands,
    mut particle_system: ResMut<ParticleSystem>,
    query: Query<(Entity, &Particle)>,
) {
    if particle_system.active_count > particle_system.max_particles * 9 / 10 {
        let mut sorted: Vec<(Entity, f32)> = query
            .iter()
            .map(|(e, p)| (e, p.elapsed / p.lifetime))
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let to_remove = particle_system.active_count - (particle_system.max_particles / 2);
        for (entity, _) in sorted.iter().take(to_remove) {
            commands.entity(*entity).despawn();
            particle_system.active_count = particle_system.active_count.saturating_sub(1);
        }
    }
}
