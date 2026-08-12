use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
    Nightmare,
    Custom,
}

impl Difficulty {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Peaceful => "Peaceful",
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
            Self::Nightmare => "Nightmare",
            Self::Custom => "Custom",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Peaceful => "No threats, pure expansion sandbox",
            Self::Easy => "Casual difficulty with reduced threat spawns",
            Self::Normal => "Balanced challenge",
            Self::Hard => "Aggressive threats, limited resources",
            Self::Nightmare => "Extreme difficulty for experienced players",
            Self::Custom => "Player-defined difficulty settings",
        }
    }

    pub fn get_modifiers(&self) -> DifficultyModifiers {
        match self {
            Self::Peaceful => DifficultyModifiers {
                threat_spawn_rate: 0.0,
                threat_damage_multiplier: 0.0,
                resource_multiplier: 1.5,
                replication_cost_multiplier: 0.7,
                starting_resources_multiplier: 2.0,
            },
            Self::Easy => DifficultyModifiers {
                threat_spawn_rate: 0.5,
                threat_damage_multiplier: 0.7,
                resource_multiplier: 1.2,
                replication_cost_multiplier: 0.9,
                starting_resources_multiplier: 1.5,
            },
            Self::Normal => DifficultyModifiers {
                threat_spawn_rate: 1.0,
                threat_damage_multiplier: 1.0,
                resource_multiplier: 1.0,
                replication_cost_multiplier: 1.0,
                starting_resources_multiplier: 1.0,
            },
            Self::Hard => DifficultyModifiers {
                threat_spawn_rate: 1.5,
                threat_damage_multiplier: 1.3,
                resource_multiplier: 0.8,
                replication_cost_multiplier: 1.2,
                starting_resources_multiplier: 0.7,
            },
            Self::Nightmare => DifficultyModifiers {
                threat_spawn_rate: 2.5,
                threat_damage_multiplier: 2.0,
                resource_multiplier: 0.5,
                replication_cost_multiplier: 1.5,
                starting_resources_multiplier: 0.5,
            },
            Self::Custom => DifficultyModifiers::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DifficultyModifiers {
    pub threat_spawn_rate: f32,
    pub threat_damage_multiplier: f32,
    pub resource_multiplier: f32,
    pub replication_cost_multiplier: f32,
    pub starting_resources_multiplier: f32,
}

impl Default for DifficultyModifiers {
    fn default() -> Self {
        Self {
            threat_spawn_rate: 1.0,
            threat_damage_multiplier: 1.0,
            resource_multiplier: 1.0,
            replication_cost_multiplier: 1.0,
            starting_resources_multiplier: 1.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GameMode {
    Sandbox,
    Campaign,
    Survival,
    Speedrun,
    PuzzleMode,
    Endless,
}

impl GameMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sandbox => "Sandbox",
            Self::Campaign => "Campaign",
            Self::Survival => "Survival",
            Self::Speedrun => "Speedrun",
            Self::PuzzleMode => "Puzzle Mode",
            Self::Endless => "Endless",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Sandbox => "Free-form exploration with no objectives",
            Self::Campaign => "Story-driven missions with progression",
            Self::Survival => "Last as long as possible against escalating threats",
            Self::Speedrun => "Race against the clock to reach milestones",
            Self::PuzzleMode => "Solve optimization challenges with constraints",
            Self::Endless => "Infinite scaling with increasing difficulty",
        }
    }

    pub fn allows_saving(&self) -> bool {
        matches!(self, Self::Sandbox | Self::Campaign)
    }

    pub fn has_time_limit(&self) -> bool {
        matches!(self, Self::Speedrun)
    }
}

#[derive(Resource)]
pub struct GameModeSettings {
    pub mode: GameMode,
    pub difficulty: Difficulty,
    pub modifiers: DifficultyModifiers,
    pub ironman: bool,
    pub permadeath: bool,
    pub starting_probe_count: usize,
    pub galaxy_size: GalaxySize,
}

impl Default for GameModeSettings {
    fn default() -> Self {
        let difficulty = Difficulty::Normal;
        Self {
            mode: GameMode::Sandbox,
            difficulty,
            modifiers: difficulty.get_modifiers(),
            ironman: false,
            permadeath: false,
            starting_probe_count: 1,
            galaxy_size: GalaxySize::Medium,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GalaxySize {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
}

impl GalaxySize {
    pub fn sector_count(&self) -> usize {
        match self {
            Self::Tiny => 25,
            Self::Small => 100,
            Self::Medium => 400,
            Self::Large => 1600,
            Self::Huge => 6400,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Tiny => "Tiny (25 sectors)",
            Self::Small => "Small (100 sectors)",
            Self::Medium => "Medium (400 sectors)",
            Self::Large => "Large (1600 sectors)",
            Self::Huge => "Huge (6400 sectors)",
        }
    }
}

pub struct SpeedrunTimer {
    pub elapsed: f32,
    pub milestones: Vec<(String, f32)>,
    pub target_time: Option<f32>,
}

impl Default for SpeedrunTimer {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            milestones: Vec::new(),
            target_time: None,
        }
    }
}

impl SpeedrunTimer {
    pub fn add_milestone(&mut self, description: String, time: f32) {
        self.milestones.push((description, time));
        println!("[SPEEDRUN] {} @ {:.2}s", description, time);
    }
}

pub struct SurvivalStats {
    pub waves_survived: usize,
    pub current_wave: usize,
    pub time_until_next_wave: f32,
    pub wave_interval: f32,
}

impl Default for SurvivalStats {
    fn default() -> Self {
        Self {
            waves_survived: 0,
            current_wave: 1,
            time_until_next_wave: 60.0,
            wave_interval: 60.0,
        }
    }
}

pub fn survival_wave_system(
    mut survival: ResMut<SurvivalStats>,
    time: Res<crate::resources::GameTime>,
) {
    survival.time_until_next_wave -= time.delta_secs;

    if survival.time_until_next_wave <= 0.0 {
        survival.current_wave += 1;
        survival.waves_survived = survival.current_wave - 1;
        survival.time_until_next_wave = survival.wave_interval;

        println!("⚠️  Wave {} incoming!", survival.current_wave);
    }
}
