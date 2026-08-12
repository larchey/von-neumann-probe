use bevy::prelude::*;
use serde::{Deserialize, Serialize};
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
        }
    }
}

#[derive(Resource)]
pub struct Camera2dResource {
    pub follow_target: Option<Entity>,
    pub zoom: f32,
}

impl Default for Camera2dResource {
    fn default() -> Self {
        Self {
            follow_target: None,
            zoom: 1.0,
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
