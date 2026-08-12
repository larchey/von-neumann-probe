use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::components::{Probe, ProbeType};

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct Fleet {
    pub id: Uuid,
    pub name: String,
    pub members: Vec<Uuid>,
    pub leader: Option<Uuid>,
    pub target_position: Option<Vec2>,
    pub mission_type: MissionType,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionType {
    Mining,
    Exploration,
    Expansion,
    Defense,
    Research,
    Idle,
}

impl Fleet {
    pub fn new(name: String, leader: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            members: vec![leader],
            leader: Some(leader),
            target_position: None,
            mission_type: MissionType::Idle,
        }
    }

    pub fn add_member(&mut self, probe_id: Uuid) -> bool {
        if !self.members.contains(&probe_id) && self.members.len() < 50 {
            self.members.push(probe_id);
            true
        } else {
            false
        }
    }

    pub fn remove_member(&mut self, probe_id: Uuid) -> bool {
        if let Some(pos) = self.members.iter().position(|&id| id == probe_id) {
            self.members.remove(pos);
            if self.leader == Some(probe_id) {
                self.leader = self.members.first().copied();
            }
            true
        } else {
            false
        }
    }

    pub fn set_mission(&mut self, mission: MissionType, target: Option<Vec2>) {
        self.mission_type = mission;
        self.target_position = target;
    }

    pub fn composition(&self) -> FleetComposition {
        FleetComposition {
            scouts: 0,
            miners: 0,
            constructors: 0,
            researchers: 0,
            warriors: 0,
            administrators: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FleetComposition {
    pub scouts: usize,
    pub miners: usize,
    pub constructors: usize,
    pub researchers: usize,
    pub warriors: usize,
    pub administrators: usize,
}

#[derive(Resource)]
pub struct FleetManager {
    pub fleets: Vec<Fleet>,
}

impl Default for FleetManager {
    fn default() -> Self {
        Self {
            fleets: Vec::new(),
        }
    }
}

impl FleetManager {
    pub fn create_fleet(&mut self, name: String, leader_id: Uuid) -> Uuid {
        let fleet = Fleet::new(name, leader_id);
        let id = fleet.id;
        self.fleets.push(fleet);
        id
    }

    pub fn get_fleet_mut(&mut self, fleet_id: Uuid) -> Option<&mut Fleet> {
        self.fleets.iter_mut().find(|f| f.id == fleet_id)
    }

    pub fn get_fleet(&self, fleet_id: Uuid) -> Option<&Fleet> {
        self.fleets.iter().find(|f| f.id == fleet_id)
    }

    pub fn dissolve_fleet(&mut self, fleet_id: Uuid) -> bool {
        if let Some(pos) = self.fleets.iter().position(|f| f.id == fleet_id) {
            self.fleets.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn fleet_count(&self) -> usize {
        self.fleets.len()
    }

    pub fn total_probes_in_fleets(&self) -> usize {
        self.fleets.iter().map(|f| f.members.len()).sum()
    }
}

pub fn fleet_coordination_system(
    fleet_manager: Res<FleetManager>,
    mut query: Query<(&mut Probe, &Transform), With<Probe>>,
) {
    if !fleet_manager.is_changed() {
        return;
    }

    for fleet in &fleet_manager.fleets {
        if let Some(target) = fleet.target_position {
            for (mut probe, transform) in query.iter_mut() {
                if fleet.members.contains(&probe.id) {
                    probe.target_position = Some(target);
                }
            }
        }
    }
}

pub fn fleet_status_debug(fleet_manager: Res<FleetManager>) {
    if fleet_manager.is_changed() {
        for fleet in &fleet_manager.fleets {
            println!(
                "[FLEET] {} ({}): {} members | Mission: {:?}",
                fleet.name, fleet.id, fleet.members.len(), fleet.mission_type
            );
        }
    }
}
