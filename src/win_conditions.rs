use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct WinConditions {
    pub probe_colonization_target: usize,
    pub tech_singularity_target: u32,
    pub dyson_sphere_percentage: f32,
    pub cathedral_count_target: u32,
    pub population_target: u64,
    pub current_probes: usize,
    pub current_tech_level: u32,
    pub cathedral_count: u32,
    pub is_victory: bool,
    pub victory_type: Option<VictoryType>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VictoryType {
    Colonization,
    Singularity,
    DysonSphere,
    CathedralMastery,
    PopulationExplosion,
}

impl Default for WinConditions {
    fn default() -> Self {
        Self {
            probe_colonization_target: 1_000_000,
            tech_singularity_target: 20,
            dyson_sphere_percentage: 85.0,
            cathedral_count_target: 100,
            population_target: 10_000_000_000,
            current_probes: 1,
            current_tech_level: 1,
            cathedral_count: 0,
            is_victory: false,
            victory_type: None,
        }
    }
}

impl WinConditions {
    pub fn check_victory(&mut self) -> Option<VictoryType> {
        if self.is_victory {
            return self.victory_type;
        }

        if self.current_probes >= self.probe_colonization_target {
            self.is_victory = true;
            self.victory_type = Some(VictoryType::Colonization);
            return Some(VictoryType::Colonization);
        }

        if self.current_tech_level >= self.tech_singularity_target {
            self.is_victory = true;
            self.victory_type = Some(VictoryType::Singularity);
            return Some(VictoryType::Singularity);
        }

        if self.cathedral_count >= self.cathedral_count_target {
            self.is_victory = true;
            self.victory_type = Some(VictoryType::CathedralMastery);
            return Some(VictoryType::CathedralMastery);
        }

        None
    }

    pub fn victory_progress(&self) -> Vec<(String, f32)> {
        vec![
            (
                format!("Probes: {}/{}", self.current_probes, self.probe_colonization_target),
                (self.current_probes as f32 / self.probe_colonization_target as f32).min(1.0),
            ),
            (
                format!("Tech Level: {}/{}", self.current_tech_level, self.tech_singularity_target),
                (self.current_tech_level as f32 / self.tech_singularity_target as f32).min(1.0),
            ),
            (
                format!("Cathedrals: {}/{}", self.cathedral_count, self.cathedral_count_target),
                (self.cathedral_count as f32 / self.cathedral_count_target as f32).min(1.0),
            ),
        ]
    }

    pub fn dominance_percentage(&self) -> f32 {
        (self.current_probes as f32 / self.probe_colonization_target as f32).min(1.0) * 100.0
    }

    pub fn technological_era(&self) -> String {
        match self.current_tech_level {
            1..=3 => "Industrial".to_string(),
            4..=7 => "Computational".to_string(),
            8..=12 => "Quantum".to_string(),
            13..=16 => "Post-Quantum".to_string(),
            17..=19 => "Transcendent".to_string(),
            20.. => "Singularity".to_string(),
        }
    }
}

pub fn victory_check_system(mut win_conditions: ResMut<WinConditions>) {
    if win_conditions.is_changed() {
        if let Some(victory) = win_conditions.check_victory() {
            println!("🎉 VICTORY! Type: {:?}", victory);
            println!("   Dominance: {:.1}%", win_conditions.dominance_percentage());
        }
    }
}

pub fn progress_ui_system(win_conditions: Res<WinConditions>) {
    if win_conditions.is_changed() && !win_conditions.is_victory {
        for (label, progress) in win_conditions.victory_progress() {
            let bar = format_progress_bar(progress);
            println!("[PROGRESS] {} | {} | {:.0}%", label, bar, progress * 100.0);
        }
        println!("[ERA] {}", win_conditions.technological_era());
    }
}

fn format_progress_bar(progress: f32) -> String {
    let filled = (progress * 20.0) as usize;
    let empty = 20 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
