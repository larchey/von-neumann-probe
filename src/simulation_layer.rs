use bevy::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

/// Multi-layer simulation architecture:
/// - Active: Full detail, viewport + 2x buffer
/// - Strategic: Aggregated swarms, 2x-20x buffer
/// - Archive: Serialized sectors, >20x buffer (disk)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SimulationLayer {
    Active,
    Strategic,
    Archive,
}

#[derive(Component, Clone, Debug)]
pub struct StrategicSwarm {
    pub id: Uuid,
    pub swarm_type: SwarmType,
    pub count: u32,
    pub position: Vec2,
    pub velocity: Vec2,
    pub heading_angle: f32,
    pub cohesion_center: Vec2,
    pub health_total: f32,
    pub max_health: f32,
    pub threat_level: f32,
    pub formation: FormationType,
    pub current_layer: SimulationLayer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwarmType {
    ProbeScout,
    ProbeMiner,
    ProbeConstructor,
    ProbeResearcher,
    ProbeWarrior,
    ThreatRogue,
    ThreatSwarm,
    ThreatDreadnought,
    ThreatLeviathan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormationType {
    Dispersed,
    Wedge,
    Line,
    Circle,
    Grid,
}

#[derive(Resource)]
pub struct SimulationLayerManager {
    pub active_swarms: HashMap<Uuid, StrategicSwarm>,
    pub strategic_swarms: HashMap<Uuid, StrategicSwarm>,
    pub archive_swarms: HashMap<Uuid, StrategicSwarm>,
    pub viewport_center: Vec2,
    pub viewport_size: Vec2,
    pub active_distance: f32,      // Inner boundary
    pub strategic_distance: f32,   // Outer boundary
    pub pending_events: Vec<CrossLayerEvent>,
}

#[derive(Clone, Debug)]
pub struct CrossLayerEvent {
    pub id: Uuid,
    pub event_type: EventType,
    pub source_position: Vec2,
    pub severity: f32, // 0-1
    pub affected_radius: f32,
    pub age: f32,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum EventType {
    Collision,
    ThreatDetected,
    ResourceSpike,
    StructureDiscovered,
    SwarmMerge,
}

impl Default for SimulationLayerManager {
    fn default() -> Self {
        Self {
            active_swarms: HashMap::new(),
            strategic_swarms: HashMap::new(),
            archive_swarms: HashMap::new(),
            viewport_center: Vec2::ZERO,
            viewport_size: Vec2::new(1920.0, 1080.0),
            active_distance: 2000.0,
            strategic_distance: 10000.0,
            pending_events: Vec::new(),
        }
    }
}

impl SimulationLayerManager {
    /// Determine which layer an entity should be in based on distance to viewport
    pub fn layer_for_position(&self, position: Vec2) -> SimulationLayer {
        let distance = self.viewport_center.distance(position);

        if distance <= self.active_distance {
            SimulationLayer::Active
        } else if distance <= self.strategic_distance {
            SimulationLayer::Strategic
        } else {
            SimulationLayer::Archive
        }
    }

    /// Transition swarm between layers (active→strategic or strategic→archive)
    pub fn transition_swarm(&mut self, swarm_id: Uuid, from: SimulationLayer, to: SimulationLayer) {
        match (from, to) {
            (SimulationLayer::Active, SimulationLayer::Strategic) => {
                if let Some(mut swarm) = self.active_swarms.remove(&swarm_id) {
                    swarm.current_layer = SimulationLayer::Strategic;
                    self.strategic_swarms.insert(swarm_id, swarm);
                }
            }
            (SimulationLayer::Strategic, SimulationLayer::Active) => {
                if let Some(mut swarm) = self.strategic_swarms.remove(&swarm_id) {
                    swarm.current_layer = SimulationLayer::Active;
                    self.active_swarms.insert(swarm_id, swarm);
                }
            }
            (SimulationLayer::Strategic, SimulationLayer::Archive) => {
                if let Some(mut swarm) = self.strategic_swarms.remove(&swarm_id) {
                    swarm.current_layer = SimulationLayer::Archive;
                    self.archive_swarms.insert(swarm_id, swarm);
                }
            }
            (SimulationLayer::Archive, SimulationLayer::Strategic) => {
                if let Some(mut swarm) = self.archive_swarms.remove(&swarm_id) {
                    swarm.current_layer = SimulationLayer::Strategic;
                    self.strategic_swarms.insert(swarm_id, swarm);
                }
            }
            _ => {} // Other transitions not supported
        }
    }

    /// Update swarm position deterministically (no random walks)
    pub fn update_swarm_position(&mut self, swarm_id: Uuid, dt: f32) {
        let layer = self
            .active_swarms
            .get(&swarm_id)
            .map(|s| s.current_layer)
            .or_else(|| {
                self.strategic_swarms
                    .get(&swarm_id)
                    .map(|s| s.current_layer)
            })
            .or_else(|| {
                self.archive_swarms
                    .get(&swarm_id)
                    .map(|s| s.current_layer)
            });

        if let Some(layer) = layer {
            let map = match layer {
                SimulationLayer::Active => &mut self.active_swarms,
                SimulationLayer::Strategic => &mut self.strategic_swarms,
                SimulationLayer::Archive => &mut self.archive_swarms,
            };

            if let Some(swarm) = map.get_mut(&swarm_id) {
                swarm.position += swarm.velocity * dt;
                swarm.age = swarm.age.unwrap_or(0.0) + dt;
            }
        }
    }

    /// Broadcast event to active layer (affects nearby probes)
    pub fn emit_cross_layer_event(&mut self, event: CrossLayerEvent) {
        self.pending_events.push(event);
    }

    /// Process pending events and apply effects
    pub fn process_layer_events(&mut self) {
        for event in self.pending_events.drain(..) {
            let affected_swarms: Vec<Uuid> = self
                .active_swarms
                .iter()
                .filter(|(_, swarm)| {
                    swarm
                        .position
                        .distance(event.source_position)
                        <= event.affected_radius
                })
                .map(|(id, _)| *id)
                .collect();

            for swarm_id in affected_swarms {
                if let Some(swarm) = self.active_swarms.get_mut(&swarm_id) {
                    match event.event_type {
                        EventType::ThreatDetected => {
                            swarm.threat_level = (swarm.threat_level + event.severity).min(1.0);
                        }
                        EventType::Collision => {
                            // Deflect away from collision point
                            let away = (swarm.position - event.source_position).normalize_or_zero();
                            swarm.velocity = away * swarm.velocity.length().max(50.0);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// System: Update viewport center based on camera position
pub fn update_viewport_center(
    mut layer_manager: ResMut<SimulationLayerManager>,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    if let Ok(camera_transform) = camera_query.get_single() {
        layer_manager.viewport_center = camera_transform.translation.xy();
    }
}

/// System: Transition swarms between layers based on distance
pub fn layer_transition_system(mut layer_manager: ResMut<SimulationLayerManager>) {
    let mut transitions = Vec::new();

    // Check active swarms for promotion to strategic
    for (id, swarm) in layer_manager.active_swarms.iter() {
        let new_layer = layer_manager.layer_for_position(swarm.position);
        if new_layer != SimulationLayer::Active {
            transitions.push((*id, SimulationLayer::Active, new_layer));
        }
    }

    // Check strategic swarms for demotion/promotion
    for (id, swarm) in layer_manager.strategic_swarms.iter() {
        let new_layer = layer_manager.layer_for_position(swarm.position);
        if new_layer != SimulationLayer::Strategic {
            transitions.push((*id, SimulationLayer::Strategic, new_layer));
        }
    }

    // Check archive swarms for promotion to strategic
    for (id, swarm) in layer_manager.archive_swarms.iter() {
        let new_layer = layer_manager.layer_for_position(swarm.position);
        if new_layer != SimulationLayer::Archive {
            transitions.push((*id, SimulationLayer::Archive, new_layer));
        }
    }

    for (id, from, to) in transitions {
        layer_manager.transition_swarm(id, from, to);
    }
}

/// System: Simulate active swarms at full detail
pub fn active_swarm_simulation(
    mut layer_manager: ResMut<SimulationLayerManager>,
    time: Res<Time>,
) {
    for swarm_id in layer_manager.active_swarms.keys().copied().collect::<Vec<_>>() {
        layer_manager.update_swarm_position(swarm_id, time.delta_secs());
    }
}

/// System: Simulate strategic swarms with reduced fidelity
pub fn strategic_swarm_simulation(
    mut layer_manager: ResMut<SimulationLayerManager>,
    time: Res<Time>,
) {
    // Strategic swarms move at same speed but only tick every 10 frames
    for swarm_id in layer_manager
        .strategic_swarms
        .keys()
        .copied()
        .collect::<Vec<_>>()
    {
        layer_manager.update_swarm_position(swarm_id, time.delta_secs());
    }
}

/// System: Process cross-layer events
pub fn event_propagation_system(mut layer_manager: ResMut<SimulationLayerManager>) {
    layer_manager.process_layer_events();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_assignment() {
        let manager = SimulationLayerManager::default();

        assert_eq!(manager.layer_for_position(Vec2::new(500.0, 0.0)), SimulationLayer::Active);
        assert_eq!(
            manager.layer_for_position(Vec2::new(5000.0, 0.0)),
            SimulationLayer::Strategic
        );
        assert_eq!(
            manager.layer_for_position(Vec2::new(50000.0, 0.0)),
            SimulationLayer::Archive
        );
    }

    #[test]
    fn test_swarm_transition() {
        let mut manager = SimulationLayerManager::default();
        let swarm_id = Uuid::new_v4();

        let swarm = StrategicSwarm {
            id: swarm_id,
            swarm_type: SwarmType::ThreatRogue,
            count: 100,
            position: Vec2::new(500.0, 0.0),
            velocity: Vec2::new(10.0, 0.0),
            heading_angle: 0.0,
            cohesion_center: Vec2::new(500.0, 0.0),
            health_total: 5000.0,
            max_health: 5000.0,
            threat_level: 0.5,
            formation: FormationType::Wedge,
            current_layer: SimulationLayer::Active,
        };

        manager.active_swarms.insert(swarm_id, swarm);
        manager.transition_swarm(swarm_id, SimulationLayer::Active, SimulationLayer::Strategic);

        assert!(manager.active_swarms.get(&swarm_id).is_none());
        assert!(manager.strategic_swarms.get(&swarm_id).is_some());
    }

    #[test]
    fn test_deterministic_movement() {
        let mut manager = SimulationLayerManager::default();
        let swarm_id = Uuid::new_v4();

        let swarm = StrategicSwarm {
            id: swarm_id,
            swarm_type: SwarmType::ThreatRogue,
            count: 100,
            position: Vec2::ZERO,
            velocity: Vec2::new(100.0, 0.0),
            heading_angle: 0.0,
            cohesion_center: Vec2::ZERO,
            health_total: 5000.0,
            max_health: 5000.0,
            threat_level: 0.5,
            formation: FormationType::Dispersed,
            current_layer: SimulationLayer::Active,
        };

        manager.active_swarms.insert(swarm_id, swarm);

        for _ in 0..10 {
            manager.update_swarm_position(swarm_id, 1.0);
        }

        let final_pos = manager.active_swarms.get(&swarm_id).unwrap().position;
        assert_eq!(final_pos.x, 1000.0); // 100 * 10 = 1000
        assert_eq!(final_pos.y, 0.0);
    }
}
