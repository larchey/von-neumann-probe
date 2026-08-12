use bevy::prelude::*;
use bevy::window::WindowResolution;

mod components;
mod systems;
mod resources;
mod generation;
mod physics;
mod rendering;

use components::*;
use systems::*;
use resources::*;
use generation::CathedralGenerator;
use rendering::MinimalistTheme;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1600.0, 900.0),
                title: "Von Neumann Probe".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(GameConfig::default())
        .init_resource::<GameState>()
        .init_resource::<CathedralGenerator>()
        .init_resource::<Camera2dResource>()
        .init_resource::<MinimalistTheme>()
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, spawn_initial_probe)
        .add_systems(Update, probe_movement)
        .add_systems(Update, probe_mining)
        .add_systems(Update, probe_replication)
        .add_systems(Update, camera_follow)
        .add_systems(Update, debug_ui)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn spawn_initial_probe(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    generator: Res<CathedralGenerator>,
) {
    let probe_id = uuid::Uuid::new_v4();
    let _structure = generator.generate_initial_colony();

    commands.spawn((
        Transform::default(),
        Probe {
            id: probe_id,
            probe_type: ProbeType::Constructor,
            health: 100.0,
            max_health: 100.0,
            energy: 100.0,
            max_energy: 100.0,
            velocity: Vec2::ZERO,
            resources: Resources {
                minerals: 50.0,
                computronium: 10.0,
                exotic_matter: 0.0,
            },
            target_position: None,
            replication_progress: 0.0,
            specialization_level: 1,
        },
        Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(8.0, 8.0)),
    ));

    game_state.probe_count = 1;
    game_state.total_resources = Resources {
        minerals: 1000.0,
        computronium: 100.0,
        exotic_matter: 0.0,
    };
}

fn debug_ui(game_state: Res<GameState>) {
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
