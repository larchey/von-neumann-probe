use bevy_ecs::system::Resource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct TechTree {
    pub techs: HashMap<String, Technology>,
    pub researched: Vec<String>,
    pub current_research: Option<String>,
    pub research_progress: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Technology {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: TechCost,
    pub prerequisites: Vec<String>,
    pub tier: u32,
    pub effects: Vec<TechEffect>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub struct TechCost {
    pub computronium: f32,
    pub time_seconds: f32,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum TechEffect {
    ProbeSpeedBoost(f32),
    MiningEfficiencyBoost(f32),
    ReplicationSpeedBoost(f32),
    CombatDamageBoost(f32),
    EnergyEfficiency(f32),
    ResourceYield(f32),
    MaxProbes(u32),
    SectorScanRange(f32),
}

impl Default for TechTree {
    fn default() -> Self {
        let mut tree = TechTree {
            techs: HashMap::new(),
            researched: vec![],
            current_research: None,
            research_progress: 0.0,
        };
        tree.initialize_tech_tree();
        tree
    }
}

impl TechTree {
    fn initialize_tech_tree(&mut self) {
        let techs = vec![
            // Tier 1: Foundational
            ("mining_efficiency_1", "Mining Efficiency I",
             "10% faster mineral extraction", 25.0, 15.0, vec![]),
            ("replication_speed_1", "Faster Replication I",
             "Probes replicate 10% faster", 30.0, 20.0, vec![]),
            ("energy_storage_1", "Energy Storage I",
             "Probe batteries hold 15% more", 20.0, 10.0, vec![]),
            ("scout_range", "Scout Range Extension",
             "Scouts detect resources 20% further", 35.0, 12.0, vec![]),

            // Tier 2: Intermediate
            ("mining_efficiency_2", "Mining Efficiency II",
             "15% faster mineral extraction (requires Tier 1)", 50.0, 30.0, vec!["mining_efficiency_1".to_string()]),
            ("combat_targeting", "Combat Targeting",
             "Warriors gain 20% accuracy bonus", 60.0, 25.0, vec![]),
            ("cathedral_scaling", "Cathedral Expansion",
             "Build structures 25% larger", 70.0, 35.0, vec![]),

            // Tier 3: Advanced
            ("exotic_matter_processing", "Exotic Matter Refining",
             "Unlock exotic matter collection", 150.0, 60.0, vec!["mining_efficiency_2".to_string()]),
            ("autonomous_fleet", "Autonomous Fleet Control",
             "Command probes in groups", 200.0, 90.0, vec!["replication_speed_1".to_string()]),
        ];

        for (id, name, desc, cost_comp, cost_time, prereqs) in techs {
            let tech = Technology {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                cost: TechCost {
                    computronium: cost_comp,
                    time_seconds: cost_time,
                },
                prerequisites: prereqs,
                tier: if id.contains("_1") { 1 } else if id.contains("_2") { 2 } else { 3 },
                effects: self.effects_for_tech(id),
            };
            self.techs.insert(id.to_string(), tech);
        }
    }

    fn effects_for_tech(&self, id: &str) -> Vec<TechEffect> {
        match id {
            "mining_efficiency_1" => vec![TechEffect::MiningEfficiencyBoost(0.1)],
            "mining_efficiency_2" => vec![TechEffect::MiningEfficiencyBoost(0.15)],
            "replication_speed_1" => vec![TechEffect::ReplicationSpeedBoost(0.1)],
            "energy_storage_1" => vec![TechEffect::EnergyEfficiency(0.15)],
            "scout_range" => vec![TechEffect::SectorScanRange(0.2)],
            "combat_targeting" => vec![TechEffect::CombatDamageBoost(0.2)],
            "cathedral_scaling" => vec![TechEffect::ResourceYield(0.25)],
            "exotic_matter_processing" => vec![TechEffect::ResourceYield(1.0)],
            "autonomous_fleet" => vec![TechEffect::MaxProbes(10)],
            _ => vec![],
        }
    }

    pub fn can_research(&self, tech_id: &str) -> bool {
        if self.researched.contains(&tech_id.to_string()) {
            return false;
        }
        if let Some(tech) = self.techs.get(tech_id) {
            tech.prerequisites.iter().all(|p| self.researched.contains(p))
        } else {
            false
        }
    }

    pub fn start_research(&mut self, tech_id: String) -> bool {
        if self.can_research(&tech_id) && self.current_research.is_none() {
            self.current_research = Some(tech_id);
            self.research_progress = 0.0;
            true
        } else {
            false
        }
    }

    pub fn add_research_progress(&mut self, delta: f32) {
        if let Some(tech_id) = &self.current_research.clone() {
            if let Some(tech) = self.techs.get(tech_id) {
                self.research_progress += delta;
                if self.research_progress >= tech.cost.time_seconds {
                    self.complete_research();
                }
            }
        }
    }

    fn complete_research(&mut self) {
        if let Some(tech_id) = self.current_research.take() {
            self.researched.push(tech_id.clone());
            self.research_progress = 0.0;
        }
    }

    pub fn get_tech_bonus(&self, effect_type: TechEffect) -> f32 {
        let mut total_bonus = 0.0;
        for tech_id in &self.researched {
            if let Some(tech) = self.techs.get(tech_id) {
                for effect in &tech.effects {
                    if std::mem::discriminant(effect) == std::mem::discriminant(&effect_type) {
                        match (effect, effect_type) {
                            (TechEffect::MiningEfficiencyBoost(a), TechEffect::MiningEfficiencyBoost(_)) => total_bonus += a,
                            (TechEffect::ReplicationSpeedBoost(a), TechEffect::ReplicationSpeedBoost(_)) => total_bonus += a,
                            (TechEffect::CombatDamageBoost(a), TechEffect::CombatDamageBoost(_)) => total_bonus += a,
                            (TechEffect::EnergyEfficiency(a), TechEffect::EnergyEfficiency(_)) => total_bonus += a,
                            (TechEffect::ResourceYield(a), TechEffect::ResourceYield(_)) => total_bonus += a,
                            (TechEffect::SectorScanRange(a), TechEffect::SectorScanRange(_)) => total_bonus += a,
                            _ => {}
                        }
                    }
                }
            }
        }
        total_bonus
    }

    pub fn available_techs(&self) -> Vec<&Technology> {
        self.techs
            .values()
            .filter(|t| self.can_research(&t.id))
            .collect()
    }
}
