use bevy::prelude::*;
use rand::Rng;
use crate::components::*;
use crate::resources::*;

#[derive(Component, Clone, Debug)]
pub struct Threat {
    pub id: uuid::Uuid,
    pub threat_type: ThreatType,
    pub health: f32,
    pub max_health: f32,
    pub velocity: Vec2,
    pub target: Option<Entity>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ThreatType {
    Rogue,
    Swarm,
    Dreadnought,
    Leviathan,
}

pub fn spawn_threats(
    mut commands: Commands,
    game_state: Res<GameState>,
    time: Res<Time>,
    mut threat_timer: Local<f32>,
) {
    *threat_timer += time.delta_secs();

    let spawn_interval = (60.0 / (1.0 + game_state.threat_level)).max(10.0);

    if *threat_timer >= spawn_interval {
        *threat_timer = 0.0;

        let mut rng = rand::thread_rng();
        let threat_type = match rng.gen_range(0..100) {
            0..=60 => ThreatType::Rogue,
            61..=85 => ThreatType::Swarm,
            86..=95 => ThreatType::Dreadnought,
            _ => ThreatType::Leviathan,
        };

        let (health, attack, color, size) = match threat_type {
            ThreatType::Rogue => (50.0, 10.0, Color::srgb(0.8, 0.3, 0.1), Vec2::splat(6.0)),
            ThreatType::Swarm => (20.0, 5.0, Color::srgb(0.9, 0.1, 0.1), Vec2::splat(4.0)),
            ThreatType::Dreadnought => (200.0, 40.0, Color::srgb(1.0, 0.0, 0.0), Vec2::splat(12.0)),
            ThreatType::Leviathan => (500.0, 80.0, Color::srgb(0.5, 0.0, 0.0), Vec2::splat(20.0)),
        };

        let spawn_angle = rng.gen::<f32>() * std::f32::consts::TAU;
        let spawn_distance = 800.0 + rng.gen::<f32>() * 400.0;
        let spawn_pos = Vec2::new(spawn_angle.cos(), spawn_angle.sin()) * spawn_distance;

        commands.spawn((
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 0.0),
            Threat {
                id: uuid::Uuid::new_v4(),
                threat_type,
                health,
                max_health: health,
                velocity: Vec2::ZERO,
                target: None,
            },
            CombatEntity {
                threat_level: attack,
                attack_power: attack,
                attack_cooldown: 2.0,
                current_cooldown: 0.0,
            },
            Sprite::from_color(color, size),
        ));
    }
}

pub fn threat_targeting(
    mut threat_query: Query<(&Transform, &mut Threat)>,
    probe_query: Query<(Entity, &Transform), With<Probe>>,
) {
    for (threat_transform, mut threat) in threat_query.iter_mut() {
        if threat.target.is_none() {
            let mut closest_probe: Option<(Entity, f32)> = None;

            for (entity, probe_transform) in probe_query.iter() {
                let distance = threat_transform.translation.xy().distance(probe_transform.translation.xy());

                if let Some((_, closest_dist)) = closest_probe {
                    if distance < closest_dist {
                        closest_probe = Some((entity, distance));
                    }
                } else {
                    closest_probe = Some((entity, distance));
                }
            }

            if let Some((entity, _)) = closest_probe {
                threat.target = Some(entity);
            }
        }
    }
}

pub fn threat_movement(
    mut threat_query: Query<(&mut Transform, &mut Threat)>,
    probe_query: Query<&Transform, With<Probe>>,
    time: Res<Time>,
) {
    for (mut threat_transform, mut threat) in threat_query.iter_mut() {
        if let Some(target) = threat.target {
            if let Ok(target_transform) = probe_query.get(target) {
                let direction = (target_transform.translation.xy() - threat_transform.translation.xy()).normalize_or_zero();
                let speed = match threat.threat_type {
                    ThreatType::Rogue => 40.0,
                    ThreatType::Swarm => 60.0,
                    ThreatType::Dreadnought => 25.0,
                    ThreatType::Leviathan => 15.0,
                };

                threat_transform.translation += (direction * speed * time.delta_secs()).extend(0.0);
                threat.velocity = direction * speed;
            } else {
                threat.target = None;
            }
        }
    }
}

pub fn threat_combat(
    mut commands: Commands,
    mut threat_query: Query<(Entity, &Transform, &mut Threat, &mut CombatEntity)>,
    mut probe_query: Query<(Entity, &Transform, &mut Probe)>,
    time: Res<Time>,
) {
    let mut damage_queue = Vec::new();

    for (threat_entity, threat_transform, threat, mut combat) in threat_query.iter_mut() {
        combat.current_cooldown = (combat.current_cooldown - time.delta_secs()).max(0.0);

        if combat.current_cooldown <= 0.0 {
            for (probe_entity, probe_transform, _) in probe_query.iter() {
                let distance = threat_transform.translation.xy().distance(probe_transform.translation.xy());

                if distance < 50.0 {
                    damage_queue.push((probe_entity, combat.attack_power));
                    combat.current_cooldown = combat.attack_cooldown;
                    break;
                }
            }
        }
    }

    for (probe_entity, damage) in damage_queue {
        if let Ok((entity, _, mut probe)) = probe_query.get_mut(probe_entity) {
            probe.health -= damage;

            if probe.health <= 0.0 {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn projectile_movement(
    mut commands: Commands,
    mut projectile_query: Query<(Entity, &mut Transform, &mut Projectile)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut projectile) in projectile_query.iter_mut() {
        transform.translation += projectile.velocity.extend(0.0) * time.delta_secs();
        projectile.lifetime -= time.delta_secs();

        if projectile.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub fn projectile_hit_detection(
    mut commands: Commands,
    projectile_query: Query<(Entity, &Transform, &Projectile)>,
    mut threat_query: Query<(Entity, &Transform, &mut Threat)>,
) {
    for (projectile_entity, projectile_transform, projectile) in projectile_query.iter() {
        for (threat_entity, threat_transform, mut threat) in threat_query.iter_mut() {
            let distance = projectile_transform.translation.xy().distance(threat_transform.translation.xy());

            if distance < 10.0 {
                threat.health -= projectile.damage;

                if threat.health <= 0.0 {
                    commands.entity(threat_entity).despawn();
                }

                commands.entity(projectile_entity).despawn();
                break;
            }
        }
    }
}
