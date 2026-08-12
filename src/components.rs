use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct Probe {
    pub id: uuid::Uuid,
    pub probe_type: ProbeType,
    pub health: f32,
    pub max_health: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub velocity: Vec2,
    pub resources: Resources,
    pub target_position: Option<Vec2>,
    pub replication_progress: f32,
    pub specialization_level: u32,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    Scout,
    Miner,
    Constructor,
    Researcher,
    Warrior,
    Administrator,
}

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct ColonyStructure {
    pub id: uuid::Uuid,
    pub structure_type: StructureType,
    pub position: Vec2,
    pub health: f32,
    pub max_health: f32,
    pub storage: Resources,
    pub max_storage: Resources,
    pub power_generation: f32,
    pub power_consumption: f32,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureType {
    Refinery,
    Foundry,
    Laboratory,
    PowerPlant,
    StorageBay,
    FactoryCathedral,
    DefenseTurret,
    ComputeNode,
    TransmissionArray,
}

#[derive(Clone, Debug, Copy, Default, Serialize, Deserialize)]
pub struct Resources {
    pub minerals: f32,
    pub computronium: f32,
    pub exotic_matter: f32,
}

impl Resources {
    pub fn total(&self) -> f32 {
        self.minerals + self.computronium + self.exotic_matter
    }

    pub fn add(&mut self, other: Resources) {
        self.minerals += other.minerals;
        self.computronium += other.computronium;
        self.exotic_matter += other.exotic_matter;
    }

    pub fn subtract(&mut self, other: Resources) -> bool {
        if self.minerals >= other.minerals
            && self.computronium >= other.computronium
            && self.exotic_matter >= other.exotic_matter
        {
            self.minerals -= other.minerals;
            self.computronium -= other.computronium;
            self.exotic_matter -= other.exotic_matter;
            true
        } else {
            false
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct AsteroidField {
    pub position: Vec2,
    pub size: f32,
    pub mineral_density: f32,
    pub resources: Resources,
}

#[derive(Component, Clone, Debug)]
pub struct CombatEntity {
    pub threat_level: f32,
    pub attack_power: f32,
    pub attack_cooldown: f32,
    pub current_cooldown: f32,
}

#[derive(Component, Clone, Debug)]
pub struct Projectile {
    pub velocity: Vec2,
    pub damage: f32,
    pub lifetime: f32,
    pub owner: uuid::Uuid,
}

#[derive(Component)]
pub struct Camera2dComponent;

#[derive(Component)]
pub struct FollowTarget;
