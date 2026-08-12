use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use crate::components::{ProbeType, Position};

#[derive(Clone, Copy)]
pub enum Command {
    Move(Vec2),
    Attack(Entity),
    Mine(Entity),
    Build(BuildTarget),
    Patrol(Vec2, Vec2),
    Guard(Entity),
    Follow(Entity),
    Replicate,
    Research(ResearchTarget),
    Harvest(Entity),
}

#[derive(Clone, Copy)]
pub enum BuildTarget {
    Refinery,
    Foundry,
    Laboratory,
    PowerPlant,
    DefenseTurret,
    StorageBay,
    Cathedral,
}

#[derive(Clone, Copy)]
pub enum ResearchTarget {
    ImprovedMining,
    FasterReplication,
    EnhancedCombat,
    AdvancedSensors,
    EnergyEfficiency,
    ExoticMatterHarvesting,
}

#[derive(Component)]
pub struct CommandQueue {
    pub commands: Vec<Command>,
    pub current: Option<Command>,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            current: None,
        }
    }

    pub fn add(&mut self, command: Command) {
        self.commands.push(command);
    }

    pub fn add_priority(&mut self, command: Command) {
        self.commands.insert(0, command);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.current = None;
    }

    pub fn next(&mut self) -> Option<Command> {
        if self.commands.is_empty() {
            self.current = None;
            None
        } else {
            self.current = Some(self.commands.remove(0));
            self.current
        }
    }

    pub fn current(&self) -> Option<Command> {
        self.current
    }
}

#[derive(Resource)]
pub struct CommandSystem {
    pub command_history: Vec<(f32, String)>,
    pub max_history: usize,
}

impl Default for CommandSystem {
    fn default() -> Self {
        Self {
            command_history: Vec::new(),
            max_history: 100,
        }
    }
}

impl CommandSystem {
    pub fn log_command(&mut self, timestamp: f32, description: String) {
        self.command_history.push((timestamp, description));
        if self.command_history.len() > self.max_history {
            self.command_history.remove(0);
        }
    }
}

pub fn command_execution_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Position, &ProbeType, &mut CommandQueue)>,
    time: Res<crate::resources::GameTime>,
) {
    for (entity, position, probe_type, mut queue) in query.iter_mut() {
        if queue.current.is_none() {
            queue.next();
        }

        if let Some(command) = queue.current {
            match command {
                Command::Move(target) => {
                    if position.0.distance(target) < 10.0 {
                        queue.next();
                    }
                }
                Command::Replicate => {
                    queue.next();
                }
                Command::Mine(_) => {
                    queue.next();
                }
                _ => {
                    queue.next();
                }
            }
        }
    }
}

pub fn rally_point_system(
    mut query: Query<(&Position, &mut CommandQueue), With<RallyPoint>>,
) {
    for (position, mut queue) in query.iter_mut() {
    }
}

#[derive(Component)]
pub struct RallyPoint {
    pub position: Vec2,
}

#[derive(Component)]
pub struct Formation {
    pub formation_type: FormationType,
    pub spacing: f32,
}

#[derive(Clone, Copy)]
pub enum FormationType {
    Line,
    Box,
    Circle,
    Wedge,
}

pub fn formation_system(
    mut query: Query<(Entity, &Position, &Formation)>,
    all_positions: Query<&Position>,
) {
}
