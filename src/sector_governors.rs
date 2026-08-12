use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use crate::components::Resources;

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct SectorGovernor {
    pub id: Uuid,
    pub sector_coords: (i32, i32),
    pub name: String,
    pub priority: GovernorPriority,
    pub resource_targets: ResourceTargets,
    pub automation_level: u8,
    pub efficiency: f32,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernorPriority {
    Mining,
    Research,
    Expansion,
    Defense,
    Balanced,
}

#[derive(Clone, Debug, Copy, Default, Serialize, Deserialize)]
pub struct ResourceTargets {
    pub mineral_quota: f32,
    pub computronium_quota: f32,
    pub exotic_matter_quota: f32,
}

#[derive(Resource)]
pub struct GovernorManager {
    pub governors: HashMap<Uuid, SectorGovernor>,
    pub sector_resources: HashMap<(i32, i32), Resources>,
}

impl Default for GovernorManager {
    fn default() -> Self {
        Self {
            governors: HashMap::new(),
            sector_resources: HashMap::new(),
        }
    }
}

impl GovernorManager {
    pub fn create_governor(
        &mut self,
        sector_coords: (i32, i32),
        name: String,
        priority: GovernorPriority,
    ) -> Uuid {
        let governor = SectorGovernor {
            id: Uuid::new_v4(),
            sector_coords,
            name,
            priority,
            resource_targets: ResourceTargets::default(),
            automation_level: 1,
            efficiency: 0.8,
        };
        let id = governor.id;
        self.governors.insert(id, governor);
        self.sector_resources
            .insert(sector_coords, Resources::default());
        id
    }

    pub fn set_resource_targets(
        &mut self,
        governor_id: Uuid,
        targets: ResourceTargets,
    ) -> bool {
        if let Some(gov) = self.governors.get_mut(&governor_id) {
            gov.resource_targets = targets;
            true
        } else {
            false
        }
    }

    pub fn upgrade_automation(&mut self, governor_id: Uuid) -> bool {
        if let Some(gov) = self.governors.get_mut(&governor_id) {
            if gov.automation_level < 5 {
                gov.automation_level += 1;
                gov.efficiency = 0.8 + (gov.automation_level as f32 * 0.04);
                return true;
            }
        }
        false
    }

    pub fn get_sector_efficiency(&self, sector_coords: (i32, i32)) -> f32 {
        if let Some(gov) = self
            .governors
            .values()
            .find(|g| g.sector_coords == sector_coords)
        {
            gov.efficiency
        } else {
            0.5
        }
    }

    pub fn get_total_resources(&self) -> Resources {
        let mut total = Resources::default();
        for resources in self.sector_resources.values() {
            total.add(*resources);
        }
        total
    }

    pub fn distribute_resources(&mut self, sector: (i32, i32), amount: Resources) {
        self.sector_resources
            .entry(sector)
            .or_default()
            .add(amount);
    }

    pub fn governor_count(&self) -> usize {
        self.governors.len()
    }

    pub fn governs_sector(&self, sector: (i32, i32)) -> bool {
        self.governors
            .values()
            .any(|g| g.sector_coords == sector)
    }
}

pub fn governor_efficiency_system(
    mut manager: ResMut<GovernorManager>,
) {
    for governor in manager.governors.values_mut() {
        match governor.priority {
            GovernorPriority::Mining => governor.efficiency = 0.95,
            GovernorPriority::Research => governor.efficiency = 0.85,
            GovernorPriority::Expansion => governor.efficiency = 0.80,
            GovernorPriority::Defense => governor.efficiency = 0.75,
            GovernorPriority::Balanced => governor.efficiency = 0.82,
        }
        governor.efficiency *= (governor.automation_level as f32 / 5.0).max(0.5);
    }
}

pub fn governor_debug_ui(manager: Res<GovernorManager>) {
    if manager.is_changed() && !manager.governors.is_empty() {
        for gov in manager.governors.values().take(3) {
            println!(
                "[GOVERNOR] {} @ {:?} | Priority: {:?} | Efficiency: {:.1}%",
                gov.name,
                gov.sector_coords,
                gov.priority,
                gov.efficiency * 100.0
            );
        }
    }
}
