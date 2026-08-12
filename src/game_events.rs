use bevy::prelude::*;
use uuid::Uuid;
use crate::components::{ProbeType, Resources};
use crate::threat_system::ThreatLevel;
use crate::tech_tree::TechEffect;

#[derive(Event, Clone, Debug)]
pub enum GameEvent {
    ProbeSpawned {
        probe_id: Uuid,
        probe_type: ProbeType,
        position: Vec2,
    },
    ProbeDied {
        probe_id: Uuid,
        position: Vec2,
    },
    ResourceHarvested {
        amount: Resources,
        source: String,
    },
    ProbeReplicated {
        parent_id: Uuid,
        child_id: Uuid,
    },
    TechResearched {
        tech_id: String,
        tech_name: String,
    },
    ThreatSpawned {
        threat_level: ThreatLevel,
        position: Vec2,
    },
    CombatOccurred {
        attacker: Uuid,
        defender: Uuid,
        damage: f32,
    },
    SectorColonized {
        sector: (i32, i32),
    },
    FleetCreated {
        fleet_name: String,
        member_count: usize,
    },
    GameVictory {
        victory_type: String,
    },
}

#[derive(Resource, Default)]
pub struct EventLog {
    pub events: Vec<(f32, GameEvent)>,
    pub max_history: usize,
}

impl EventLog {
    pub fn new(max_history: usize) -> Self {
        Self {
            events: Vec::new(),
            max_history,
        }
    }

    pub fn log_event(&mut self, time: f32, event: GameEvent) {
        self.events.push((time, event));
        if self.events.len() > self.max_history {
            self.events.remove(0);
        }
    }

    pub fn last_n_events(&self, n: usize) -> Vec<&(f32, GameEvent)> {
        let start = self.events.len().saturating_sub(n);
        self.events[start..].iter().collect()
    }

    pub fn events_by_type(&self, event_type: &str) -> Vec<&(f32, GameEvent)> {
        self.events
            .iter()
            .filter(|(_, e)| std::mem::discriminant(e).to_string().contains(event_type))
            .collect()
    }
}

pub fn event_logging_system(
    mut events: EventReader<GameEvent>,
    mut event_log: ResMut<EventLog>,
    time: Res<Time>,
) {
    for event in events.read() {
        event_log.log_event(time.elapsed_secs(), event.clone());
    }
}

pub fn event_debug_system(event_log: Res<EventLog>) {
    if event_log.is_changed() && !event_log.events.is_empty() {
        if let Some((_, last_event)) = event_log.events.last() {
            println!("[EVENT] {:?}", last_event);
        }
    }
}
