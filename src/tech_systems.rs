use bevy::prelude::*;
use crate::resources::GameState;
use crate::tech_tree::{TechTree, TechEffect};
use crate::components::{Probe, ProbeType};

pub fn research_progress_system(
    mut tech_tree: ResMut<TechTree>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
) {
    if tech_tree.current_research.is_some() {
        let delta = time.delta_secs();
        tech_tree.add_research_progress(delta);

        if let Some(tech_id) = &tech_tree.current_research {
            if let Some(tech) = tech_tree.techs.get(tech_id).cloned() {
                let progress_pct = (tech_tree.research_progress / tech.cost.time_seconds * 100.0) as u32;
                if progress_pct % 10 == 0 && game_state.is_changed() {
                    println!("[RESEARCH] {}: {}%", tech.name, progress_pct);
                }
            }
        }
    }
}

pub fn apply_tech_bonuses_mining(
    tech_tree: Res<TechTree>,
    mut query: Query<&mut Probe, With<Probe>>,
) {
    let mining_bonus = tech_tree.get_tech_bonus(TechEffect::MiningEfficiencyBoost(0.0));
    for _ in query.iter_mut() {
        for mut probe in query.iter_mut() {
            if probe.probe_type == ProbeType::Miner && mining_bonus > 0.0 {
                probe.specialization_level = (1.0 + mining_bonus) as u32;
            }
        }
    }
}

pub fn apply_tech_bonuses_replication(
    tech_tree: Res<TechTree>,
    mut game_state: ResMut<GameState>,
) {
    let replication_bonus = tech_tree.get_tech_bonus(TechEffect::ReplicationSpeedBoost(0.0));
    if replication_bonus > 0.0 {
        game_state.tech_level = (1.0 + replication_bonus) as u32;
    }
}

pub fn tech_debug_ui(tech_tree: Res<TechTree>) {
    if tech_tree.is_changed() && tech_tree.current_research.is_some() {
        println!(
            "[TECH TREE] Researched: {} | Current: {:?}",
            tech_tree.researched.len(),
            tech_tree.current_research
        );
    }
}
