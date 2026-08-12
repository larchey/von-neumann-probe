use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::resources::GameState;
use crate::components::Resources;
use crate::tech_tree::TechTree;
use crate::fleet_automation::FleetManager;
use crate::sector_governors::GovernorManager;
use crate::win_conditions::WinConditions;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveData {
    pub version: String,
    pub timestamp: String,
    pub playtime_seconds: f32,
    pub game_state: SerializedGameState,
    pub tech_tree_state: SerializedTechState,
    pub victory_progress: SerializedVictoryState,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializedGameState {
    pub probe_count: usize,
    pub total_resources: Resources,
    pub tech_level: u32,
    pub threat_level: f32,
    pub expansion_progress: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializedTechState {
    pub researched_count: usize,
    pub current_research: Option<String>,
    pub research_progress: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializedVictoryState {
    pub current_probes: usize,
    pub current_tech_level: u32,
    pub cathedral_count: u32,
    pub is_victory: bool,
}

pub struct SaveSystem;

impl SaveSystem {
    pub fn create_save(
        game_state: &GameState,
        tech_tree: &TechTree,
        win_conditions: &WinConditions,
    ) -> SaveData {
        SaveData {
            version: "0.1.0".to_string(),
            timestamp: chrono::Local::now().to_rfc3339(),
            playtime_seconds: game_state.time_elapsed,
            game_state: SerializedGameState {
                probe_count: game_state.probe_count,
                total_resources: game_state.total_resources,
                tech_level: game_state.tech_level,
                threat_level: game_state.threat_level,
                expansion_progress: game_state.expansion_progress,
            },
            tech_tree_state: SerializedTechState {
                researched_count: tech_tree.researched.len(),
                current_research: tech_tree.current_research.clone(),
                research_progress: tech_tree.research_progress,
            },
            victory_progress: SerializedVictoryState {
                current_probes: win_conditions.current_probes,
                current_tech_level: win_conditions.current_tech_level,
                cathedral_count: win_conditions.cathedral_count,
                is_victory: win_conditions.is_victory,
            },
        }
    }

    pub fn save_to_file(data: &SaveData, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)?;
        println!("[SAVE] Game saved to {}", path);
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<SaveData> {
        let contents = std::fs::read_to_string(path)?;
        let data: SaveData = serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        println!("[LOAD] Game loaded from {} (v{})", path, data.version);
        Ok(data)
    }

    pub fn auto_save_path(slot: u32) -> String {
        format!("saves/autosave_{}.json", slot)
    }

    pub fn latest_save_file(saves_dir: &str) -> Option<String> {
        if !Path::new(saves_dir).exists() {
            return None;
        }

        std::fs::read_dir(saves_dir)
            .ok()?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.extension()?.to_str()? == "json" {
                        Some(path.to_string_lossy().to_string())
                    } else {
                        None
                    }
                })
            })
            .max()
    }
}

pub fn create_autosave_system(
    game_state: Res<GameState>,
    tech_tree: Res<TechTree>,
    win_conditions: Res<WinConditions>,
) {
    // Would be called periodically (e.g., every 5 minutes)
    if game_state.time_elapsed % 300.0 < 0.016 {
        let save_data = SaveSystem::create_save(&game_state, &tech_tree, &win_conditions);
        let path = SaveSystem::auto_save_path(1);
        let _ = SaveSystem::save_to_file(&save_data, &path);
    }
}
