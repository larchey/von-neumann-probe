use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Achievement {
    FirstReplication,
    TenProbes,
    HundredProbes,
    ThousandProbes,
    TenThousandProbes,
    FirstCombat,
    DestroyTenThreats,
    DestroyHundredThreats,
    FirstCathedral,
    TenCathedrals,
    ExploreTenSectors,
    ExploreHundredSectors,
    FirstTechResearched,
    AllTechsResearched,
    ReachTechLevel5,
    ReachTechLevel10,
    SurviveTenMinutes,
    SurviveHour,
    AccumulateTenThousandMinerals,
    AccumulateMillionMinerals,
    BuildDysonSphere,
    ColonizeHalfGalaxy,
    Singularity,
    PerfectEfficiency,
    NoProbesLost,
    SpeedrunTenMinutes,
    MassExtinction,
    PeacefulExpansion,
    ResearchFocus,
    MilitaryDominance,
}

impl Achievement {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FirstReplication => "Genesis",
            Self::TenProbes => "Small Colony",
            Self::HundredProbes => "Growing Swarm",
            Self::ThousandProbes => "Industrial Scale",
            Self::TenThousandProbes => "Exponential Growth",
            Self::FirstCombat => "First Blood",
            Self::DestroyTenThreats => "Defender",
            Self::DestroyHundredThreats => "Exterminator",
            Self::FirstCathedral => "Architect",
            Self::TenCathedrals => "City Builder",
            Self::ExploreTenSectors => "Scout",
            Self::ExploreHundredSectors => "Explorer",
            Self::FirstTechResearched => "Researcher",
            Self::AllTechsResearched => "Technological Singularity",
            Self::ReachTechLevel5 => "Advanced Civilization",
            Self::ReachTechLevel10 => "Transcendence",
            Self::SurviveTenMinutes => "Survivor",
            Self::SurviveHour => "Marathon",
            Self::AccumulateTenThousandMinerals => "Miner",
            Self::AccumulateMillionMinerals => "Resource Baron",
            Self::BuildDysonSphere => "Star Harvester",
            Self::ColonizeHalfGalaxy => "Galactic Empire",
            Self::Singularity => "Von Neumann Victory",
            Self::PerfectEfficiency => "Optimized",
            Self::NoProbesLost => "Flawless",
            Self::SpeedrunTenMinutes => "Speedrunner",
            Self::MassExtinction => "Destroyer of Worlds",
            Self::PeacefulExpansion => "Pacifist",
            Self::ResearchFocus => "Scientific Method",
            Self::MilitaryDominance => "Warmonger",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::FirstReplication => "Create your first probe replica",
            Self::TenProbes => "Build a colony of 10 probes",
            Self::HundredProbes => "Expand to 100 probes",
            Self::ThousandProbes => "Command 1,000 probes",
            Self::TenThousandProbes => "Achieve a swarm of 10,000 probes",
            Self::FirstCombat => "Destroy your first threat",
            Self::DestroyTenThreats => "Eliminate 10 hostile entities",
            Self::DestroyHundredThreats => "Destroy 100 threats",
            Self::FirstCathedral => "Construct your first cathedral",
            Self::TenCathedrals => "Build 10 cathedral structures",
            Self::ExploreTenSectors => "Explore 10 different sectors",
            Self::ExploreHundredSectors => "Map 100 sectors of space",
            Self::FirstTechResearched => "Complete your first research project",
            Self::AllTechsResearched => "Unlock the entire tech tree",
            Self::ReachTechLevel5 => "Advance to tech level 5",
            Self::ReachTechLevel10 => "Reach maximum tech level",
            Self::SurviveTenMinutes => "Survive for 10 minutes",
            Self::SurviveHour => "Maintain your colony for 1 hour",
            Self::AccumulateTenThousandMinerals => "Mine 10,000 minerals",
            Self::AccumulateMillionMinerals => "Stockpile 1 million minerals",
            Self::BuildDysonSphere => "Construct a Dyson Sphere",
            Self::ColonizeHalfGalaxy => "Control 50% of the galaxy",
            Self::Singularity => "Achieve technological singularity",
            Self::PerfectEfficiency => "Maintain 100% efficiency for 5 minutes",
            Self::NoProbesLost => "Complete a game without losing a probe",
            Self::SpeedrunTenMinutes => "Reach 1000 probes in under 10 minutes",
            Self::MassExtinction => "Destroy 1000 threats",
            Self::PeacefulExpansion => "Reach 1000 probes without combat",
            Self::ResearchFocus => "Unlock 5 techs before building 100 probes",
            Self::MilitaryDominance => "Have 50% warrior probes in a 200+ fleet",
        }
    }

    pub fn is_hidden(&self) -> bool {
        matches!(
            self,
            Self::Singularity | Self::PerfectEfficiency | Self::NoProbesLost
        )
    }
}

#[derive(Resource)]
pub struct AchievementManager {
    pub unlocked: Vec<Achievement>,
    pub unlock_times: Vec<(Achievement, f32)>,
    pub notifications_enabled: bool,
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self {
            unlocked: Vec::new(),
            unlock_times: Vec::new(),
            notifications_enabled: true,
        }
    }
}

impl AchievementManager {
    pub fn unlock(&mut self, achievement: Achievement, timestamp: f32) {
        if !self.unlocked.contains(&achievement) {
            self.unlocked.push(achievement);
            self.unlock_times.push((achievement, timestamp));

            if self.notifications_enabled {
                println!(
                    "🏆 ACHIEVEMENT UNLOCKED: {} - {}",
                    achievement.name(),
                    achievement.description()
                );
            }
        }
    }

    pub fn is_unlocked(&self, achievement: Achievement) -> bool {
        self.unlocked.contains(&achievement)
    }

    pub fn progress(&self) -> (usize, usize) {
        let total = 30;
        (self.unlocked.len(), total)
    }
}

pub fn achievement_check_system(
    mut achievements: ResMut<AchievementManager>,
    game_state: Res<crate::resources::GameState>,
    time: Res<crate::resources::GameTime>,
) {
    let timestamp = time.total_secs;

    if game_state.probe_count >= 1 && !achievements.is_unlocked(Achievement::FirstReplication) {
        achievements.unlock(Achievement::FirstReplication, timestamp);
    }

    if game_state.probe_count >= 10 && !achievements.is_unlocked(Achievement::TenProbes) {
        achievements.unlock(Achievement::TenProbes, timestamp);
    }

    if game_state.probe_count >= 100 && !achievements.is_unlocked(Achievement::HundredProbes) {
        achievements.unlock(Achievement::HundredProbes, timestamp);
    }

    if game_state.probe_count >= 1000 && !achievements.is_unlocked(Achievement::ThousandProbes) {
        achievements.unlock(Achievement::ThousandProbes, timestamp);
    }

    if game_state.probe_count >= 10000 && !achievements.is_unlocked(Achievement::TenThousandProbes) {
        achievements.unlock(Achievement::TenThousandProbes, timestamp);
    }

    if game_state.threats_defeated >= 1 && !achievements.is_unlocked(Achievement::FirstCombat) {
        achievements.unlock(Achievement::FirstCombat, timestamp);
    }

    if game_state.threats_defeated >= 10 && !achievements.is_unlocked(Achievement::DestroyTenThreats) {
        achievements.unlock(Achievement::DestroyTenThreats, timestamp);
    }

    if game_state.threats_defeated >= 100 && !achievements.is_unlocked(Achievement::DestroyHundredThreats) {
        achievements.unlock(Achievement::DestroyHundredThreats, timestamp);
    }

    if game_state.sectors_explored >= 10 && !achievements.is_unlocked(Achievement::ExploreTenSectors) {
        achievements.unlock(Achievement::ExploreTenSectors, timestamp);
    }

    if game_state.sectors_explored >= 100 && !achievements.is_unlocked(Achievement::ExploreHundredSectors) {
        achievements.unlock(Achievement::ExploreHundredSectors, timestamp);
    }

    if game_state.tech_level >= 5 && !achievements.is_unlocked(Achievement::ReachTechLevel5) {
        achievements.unlock(Achievement::ReachTechLevel5, timestamp);
    }

    if game_state.tech_level >= 10 && !achievements.is_unlocked(Achievement::ReachTechLevel10) {
        achievements.unlock(Achievement::ReachTechLevel10, timestamp);
    }

    if timestamp >= 600.0 && !achievements.is_unlocked(Achievement::SurviveTenMinutes) {
        achievements.unlock(Achievement::SurviveTenMinutes, timestamp);
    }

    if timestamp >= 3600.0 && !achievements.is_unlocked(Achievement::SurviveHour) {
        achievements.unlock(Achievement::SurviveHour, timestamp);
    }
}
