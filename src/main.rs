use bevy_ecs::prelude::*;
use bevy_math::Vec2;

mod components;
mod systems;
mod resources;
mod generation;
mod physics;
mod rendering;
mod threat_system;
mod sector_streaming;
mod tech_tree;
mod tech_systems;
mod fleet_automation;
mod sector_governors;
mod win_conditions;
mod particles;
mod audio;
mod ui;
mod input;
mod pathfinding;
mod simulation_layer;
mod spatial_hash;
mod archive_system;
mod commands;
mod achievements;
mod missions;
mod difficulty;
mod multiplayer;
mod wgpu_renderer;
mod game_events;

use components::*;
use systems::*;
use resources::{GameState, GameConfig, Camera2dResource, GameTime};
use generation::CathedralGenerator;
use rendering::MinimalistTheme;
use threat_system::*;
use sector_streaming::*;
use tech_tree::TechTree;
use tech_systems::*;
use fleet_automation::{FleetManager, fleet_coordination_system, fleet_status_debug};
use sector_governors::{GovernorManager, governor_efficiency_system, governor_debug_ui};
use win_conditions::{WinConditions, victory_check_system, progress_ui_system};
use particles::{ParticleSystem, update_particles, particle_cleanup_system};
use audio::{AudioManager, audio_update_system, dynamic_music_system};
use ui::{NotificationManager, HUD, notification_update_system, ui_render_system};
use input::{InputState, SelectionManager, input_clear_system, selection_input_system, camera_control_system};
use pathfinding::{Pathfinder, pathfinding_system};
use simulation_layer::{SimulationLayerManager, update_viewport_center, layer_transition_system, active_swarm_simulation, strategic_swarm_simulation, event_propagation_system};
use spatial_hash::SpatialGrid;
use archive_system::ArchiveManager;
use commands::{CommandSystem, command_execution_system};
use achievements::{AchievementManager, achievement_check_system};
use missions::{MissionManager, mission_update_system};
use difficulty::GameModeSettings;

fn main() {
    println!("🚀 Von Neumann Probe Engine v0.1.0");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Initializing game systems...\n");

    let mut world = World::new();
    let mut schedule = Schedule::default();

    world.insert_resource(GameConfig::default());
    world.insert_resource(GameState::default());
    world.insert_resource(GameTime::default());
    world.insert_resource(CathedralGenerator::default());
    world.insert_resource(Camera2dResource::default());
    world.insert_resource(MinimalistTheme::default());
    world.insert_resource(SectorManager::default());
    world.insert_resource(TechTree::default());
    world.insert_resource(FleetManager::default());
    world.insert_resource(GovernorManager::default());
    world.insert_resource(WinConditions::default());
    world.insert_resource(ParticleSystem::default());
    world.insert_resource(AudioManager::default());
    world.insert_resource(NotificationManager::default());
    world.insert_resource(HUD::default());
    world.insert_resource(InputState::default());
    world.insert_resource(SelectionManager::default());
    world.insert_resource(Pathfinder::default());
    world.insert_resource(CommandSystem::default());
    world.insert_resource(AchievementManager::default());
    world.insert_resource(MissionManager::default());
    world.insert_resource(GameModeSettings::default());

    // Scaling systems (multi-layer simulation)
    world.insert_resource(SimulationLayerManager::default());
    world.insert_resource(SpatialGrid::new(100.0));
    world.insert_resource(ArchiveManager::default());

    // Startup systems
    let mut startup_schedule = Schedule::default();
    startup_schedule.add_systems((spawn_initial_probe,).chain());
    startup_schedule.execute(&mut world);

    println!("✅ Game initialized!");
    println!("📊 Simulation ready for rendering integration");
}


fn spawn_initial_probe(mut world: &mut World) {
    let mut game_state = world.get_resource_mut::<GameState>().unwrap();

    game_state.probe_count = 1;
    game_state.total_resources = Resources {
        minerals: 1000.0,
        computronium: 100.0,
        exotic_matter: 0.0,
    };

    let probe_id = uuid::Uuid::new_v4();
    println!("[INIT] Spawned initial Constructor probe: {}", probe_id);
    println!("[INIT] Starting resources - Minerals: 1000.0, Computronium: 100.0");
}

