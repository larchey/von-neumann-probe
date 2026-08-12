use bevy::prelude::*;
use crate::components::*;
use crate::resources::*;

pub fn probe_movement(
    mut query: Query<(&mut Transform, &mut Probe)>,
    config: Res<GameConfig>,
    time: Res<Time>,
) {
    for (mut transform, mut probe) in query.iter_mut() {
        if let Some(target) = probe.target_position {
            let direction = (target - transform.translation.xy()).normalize();
            let move_speed = config.probe_speed * (1.0 + probe.specialization_level as f32 * 0.1);

            transform.translation += (direction * move_speed * time.delta_secs()).extend(0.0);

            let distance = (target - transform.translation.xy()).length();
            if distance < 10.0 {
                probe.target_position = None;
            }

            probe.energy = (probe.energy - move_speed * time.delta_secs() * 0.5).max(0.0);
        }

        transform.translation.z = 0.0;
    }
}

pub fn probe_mining(
    mut query: Query<&mut Probe>,
    mut game_state: ResMut<GameState>,
    config: Res<GameConfig>,
    time: Res<Time>,
) {
    for mut probe in query.iter_mut() {
        if probe.probe_type == ProbeType::Miner && probe.energy > 10.0 {
            let mining_amount = config.mining_rate * time.delta_secs();
            probe.resources.minerals += mining_amount;
            probe.energy -= mining_amount * 0.1;

            game_state.total_resources.minerals += mining_amount * 0.1;
        }
    }
}

pub fn probe_replication(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &mut Probe)>,
    mut game_state: ResMut<GameState>,
    config: Res<GameConfig>,
) {
    let mut new_probes = Vec::new();

    for (entity, transform, mut probe) in query.iter_mut() {
        if probe.resources.minerals >= config.replication_cost.minerals
            && probe.resources.computronium >= config.replication_cost.computronium
        {
            if game_state.probe_count < config.max_colony_size {
                probe.replication_progress += 0.01;

                if probe.replication_progress >= 1.0 {
                    if probe.resources.subtract(config.replication_cost) {
                        probe.replication_progress = 0.0;

                        let new_probe_type = match rand::random::<f32>() {
                            x if x < 0.3 => ProbeType::Miner,
                            x if x < 0.5 => ProbeType::Scout,
                            x if x < 0.7 => ProbeType::Researcher,
                            _ => ProbeType::Constructor,
                        };

                        let offset = Vec2::new(rand::random::<f32>() * 20.0 - 10.0, rand::random::<f32>() * 20.0 - 10.0);

                        new_probes.push((
                            transform.translation.xy() + offset,
                            new_probe_type,
                        ));

                        game_state.probe_count += 1;
                    }
                }
            }
        }
    }

    for (position, probe_type) in new_probes {
        let probe_id = uuid::Uuid::new_v4();
        commands.spawn((
            Transform::from_xyz(position.x, position.y, 0.0),
            Probe {
                id: probe_id,
                probe_type,
                health: 100.0,
                max_health: 100.0,
                energy: 100.0,
                max_energy: 100.0,
                velocity: Vec2::ZERO,
                resources: Resources::default(),
                target_position: None,
                replication_progress: 0.0,
                specialization_level: 1,
            },
            Sprite {
                color: match probe_type {
                    ProbeType::Miner => Color::srgb(0.8, 0.6, 0.2),
                    ProbeType::Scout => Color::srgb(0.3, 0.6, 0.8),
                    ProbeType::Researcher => Color::srgb(0.8, 0.3, 0.8),
                    ProbeType::Warrior => Color::srgb(0.8, 0.2, 0.2),
                    _ => Color::srgb(0.2, 0.8, 0.2),
                },
                custom_size: Some(Vec2::new(8.0, 8.0)),
                ..default()
            },
        ));
    }
}

pub fn combat_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &mut Probe, &mut CombatEntity)>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
) {
    for (entity, transform, mut probe, mut combat) in query.iter_mut() {
        if probe.probe_type != ProbeType::Warrior {
            continue;
        }

        combat.current_cooldown = (combat.current_cooldown - time.delta_secs()).max(0.0);

        if combat.current_cooldown <= 0.0 && probe.energy >= 20.0 {
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            let projectile_velocity = Vec2::new(angle.cos(), angle.sin()) * 100.0;

            commands.spawn((
                Transform::from_xyz(transform.translation.x, transform.translation.y, 0.0),
                Projectile {
                    velocity: projectile_velocity,
                    damage: combat.attack_power,
                    lifetime: 5.0,
                    owner: probe.id,
                },
                Sprite {
                    color: Color::srgb(0.9, 0.4, 0.0),
                    custom_size: Some(Vec2::new(3.0, 3.0)),
                    ..default()
                },
            ));

            probe.energy -= 20.0;
            combat.current_cooldown = combat.attack_cooldown;
        }
    }
}

pub fn camera_follow(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    camera_res: Res<Camera2dResource>,
    probe_query: Query<&Transform, (With<Probe>, Without<Camera2d>)>,
) {
    if let Some(entity) = camera_res.follow_target {
        if let Ok(probe_transform) = probe_query.get(entity) {
            if let Ok(mut camera_transform) = camera_query.get_single_mut() {
                camera_transform.translation = probe_transform.translation;
                camera_transform.translation.z = 999.0;
            }
        }
    } else if let Ok(mut camera_transform) = camera_query.get_single_mut() {
        if let Some(first_probe) = probe_query.iter().next() {
            camera_transform.translation = first_probe.translation;
            camera_transform.translation.z = 999.0;
        }
    }
}

pub fn debug_ui(game_state: Res<GameState>) {
    if game_state.is_changed() {
        println!(
            "[GAME] Probes: {} | Minerals: {:.0} | Computronium: {:.0} | Tech: {}",
            game_state.probe_count,
            game_state.total_resources.minerals,
            game_state.total_resources.computronium,
            game_state.tech_level,
        );
    }
}
