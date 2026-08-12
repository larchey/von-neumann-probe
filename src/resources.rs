use serde::{Deserialize, Serialize};
use bevy_ecs::system::Resource;
use bevy_ecs::entity::Entity;
use bevy_math::Vec2;
use crate::components::{Resources, ColonyStructure};

#[derive(Resource)]
pub struct GameState {
    pub time_elapsed: f32,
    pub probe_count: usize,
    pub total_resources: Resources,
    pub tech_level: u32,
    pub colony_structures: Vec<ColonyStructure>,
    pub threat_level: f32,
    pub expansion_progress: f32,
    pub threats_defeated: usize,
    pub sectors_explored: usize,
    pub research_progress: f32,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            time_elapsed: 0.0,
            probe_count: 0,
            total_resources: Resources::default(),
            tech_level: 1,
            colony_structures: Vec::new(),
            threat_level: 0.0,
            expansion_progress: 0.0,
            threats_defeated: 0,
            sectors_explored: 1,
            research_progress: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct GameTime {
    pub delta_secs: f32,
    pub total_secs: f32,
}

impl Default for GameTime {
    fn default() -> Self {
        Self {
            delta_secs: 0.016,
            total_secs: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct Camera2dResource {
    pub follow_target: Option<Entity>,
    pub zoom: f32,
    pub position: Vec2,
}

impl Default for Camera2dResource {
    fn default() -> Self {
        Self {
            follow_target: None,
            zoom: 1.0,
            position: Vec2::ZERO,
        }
    }
}

#[derive(Resource, Debug)]
pub struct GameConfig {
    pub probe_speed: f32,
    pub mining_rate: f32,
    pub replication_cost: Resources,
    pub max_colony_size: usize,
    pub combat_enabled: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            probe_speed: 50.0,
            mining_rate: 5.0,
            replication_cost: Resources {
                minerals: 30.0,
                computronium: 20.0,
                exotic_matter: 0.0,
            },
            max_colony_size: 10000,
            combat_enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub game_state: GameStateSave,
    pub probes: Vec<ProbeSave>,
    pub structures: Vec<StructureSave>,
}

#[derive(Serialize, Deserialize)]
pub struct GameStateSave {
    pub time_elapsed: f32,
    pub tech_level: u32,
    pub threat_level: f32,
    pub expansion_progress: f32,
}

#[derive(Serialize, Deserialize)]
pub struct ProbeSave {
    pub id: String,
    pub probe_type: String,
    pub position: (f32, f32),
    pub resources: (f32, f32, f32),
}

#[derive(Serialize, Deserialize)]
pub struct StructureSave {
    pub id: String,
    pub structure_type: String,
    pub position: (f32, f32),
    pub health: f32,
}
