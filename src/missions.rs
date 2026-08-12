use bevy_ecs::prelude::*;
use bevy_math::Vec2;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MissionType {
    Tutorial,
    Exploration,
    Combat,
    Construction,
    Research,
    Survival,
    Timed,
    Puzzle,
}

#[derive(Clone)]
pub struct Mission {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub mission_type: MissionType,
    pub objectives: Vec<Objective>,
    pub rewards: Rewards,
    pub time_limit: Option<f32>,
    pub unlocked: bool,
    pub completed: bool,
}

#[derive(Clone)]
pub struct Objective {
    pub description: String,
    pub objective_type: ObjectiveType,
    pub progress: f32,
    pub target: f32,
    pub completed: bool,
}

#[derive(Clone)]
pub enum ObjectiveType {
    BuildProbes(usize),
    DestroyThreats(usize),
    ExploreArea(Vec2, f32),
    CollectResources(String, f32),
    ConstructStructure(String, usize),
    ResearchTech(String),
    SurviveTime(f32),
    ReachLocation(Vec2),
}

#[derive(Clone)]
pub struct Rewards {
    pub minerals: f32,
    pub computronium: f32,
    pub exotic_matter: f32,
    pub tech_points: u32,
    pub unlocks_missions: Vec<u32>,
}

impl Default for Rewards {
    fn default() -> Self {
        Self {
            minerals: 0.0,
            computronium: 0.0,
            exotic_matter: 0.0,
            tech_points: 0,
            unlocks_missions: Vec::new(),
        }
    }
}

#[derive(Resource)]
pub struct MissionManager {
    pub missions: Vec<Mission>,
    pub active_mission: Option<usize>,
}

impl Default for MissionManager {
    fn default() -> Self {
        let mut manager = Self {
            missions: Vec::new(),
            active_mission: None,
        };

        manager.missions.push(Mission {
            id: 0,
            name: "First Steps".to_string(),
            description: "Learn the basics of self-replication".to_string(),
            mission_type: MissionType::Tutorial,
            objectives: vec![
                Objective {
                    description: "Build your first probe replica".to_string(),
                    objective_type: ObjectiveType::BuildProbes(1),
                    progress: 0.0,
                    target: 1.0,
                    completed: false,
                },
            ],
            rewards: Rewards {
                minerals: 100.0,
                computronium: 50.0,
                unlocks_missions: vec![1],
                ..Default::default()
            },
            time_limit: None,
            unlocked: true,
            completed: false,
        });

        manager.missions.push(Mission {
            id: 1,
            name: "Exponential Growth".to_string(),
            description: "Scale up your probe population".to_string(),
            mission_type: MissionType::Exploration,
            objectives: vec![
                Objective {
                    description: "Build 10 probes".to_string(),
                    objective_type: ObjectiveType::BuildProbes(10),
                    progress: 0.0,
                    target: 10.0,
                    completed: false,
                },
                Objective {
                    description: "Explore 3 sectors".to_string(),
                    objective_type: ObjectiveType::ExploreArea(Vec2::ZERO, 3000.0),
                    progress: 0.0,
                    target: 3.0,
                    completed: false,
                },
            ],
            rewards: Rewards {
                minerals: 500.0,
                computronium: 200.0,
                tech_points: 1,
                unlocks_missions: vec![2, 3],
                ..Default::default()
            },
            time_limit: None,
            unlocked: false,
            completed: false,
        });

        manager.missions.push(Mission {
            id: 2,
            name: "First Contact".to_string(),
            description: "Defend against hostile entities".to_string(),
            mission_type: MissionType::Combat,
            objectives: vec![
                Objective {
                    description: "Destroy 5 rogue probes".to_string(),
                    objective_type: ObjectiveType::DestroyThreats(5),
                    progress: 0.0,
                    target: 5.0,
                    completed: false,
                },
            ],
            rewards: Rewards {
                minerals: 300.0,
                tech_points: 2,
                unlocks_missions: vec![4],
                ..Default::default()
            },
            time_limit: None,
            unlocked: false,
            completed: false,
        });

        manager.missions.push(Mission {
            id: 3,
            name: "Industrial Foundation".to_string(),
            description: "Build your first cathedral structure".to_string(),
            mission_type: MissionType::Construction,
            objectives: vec![
                Objective {
                    description: "Construct a cathedral".to_string(),
                    objective_type: ObjectiveType::ConstructStructure("Cathedral".to_string(), 1),
                    progress: 0.0,
                    target: 1.0,
                    completed: false,
                },
                Objective {
                    description: "Accumulate 1000 minerals".to_string(),
                    objective_type: ObjectiveType::CollectResources("minerals".to_string(), 1000.0),
                    progress: 0.0,
                    target: 1000.0,
                    completed: false,
                },
            ],
            rewards: Rewards {
                minerals: 1000.0,
                computronium: 500.0,
                exotic_matter: 10.0,
                tech_points: 3,
                unlocks_missions: vec![5],
            },
            time_limit: None,
            unlocked: false,
            completed: false,
        });

        manager.missions.push(Mission {
            id: 4,
            name: "Speedrun Challenge".to_string(),
            description: "Build 100 probes in under 5 minutes".to_string(),
            mission_type: MissionType::Timed,
            objectives: vec![
                Objective {
                    description: "Build 100 probes".to_string(),
                    objective_type: ObjectiveType::BuildProbes(100),
                    progress: 0.0,
                    target: 100.0,
                    completed: false,
                },
            ],
            rewards: Rewards {
                exotic_matter: 50.0,
                tech_points: 5,
                ..Default::default()
            },
            time_limit: Some(300.0),
            unlocked: false,
            completed: false,
        });

        manager
    }
}

impl MissionManager {
    pub fn update_progress(&mut self, objective_type: ObjectiveType, amount: f32) {
        if let Some(active_idx) = self.active_mission {
            if let Some(mission) = self.missions.get_mut(active_idx) {
                for objective in &mut mission.objectives {
                    if std::mem::discriminant(&objective.objective_type) == std::mem::discriminant(&objective_type) {
                        objective.progress = (objective.progress + amount).min(objective.target);
                        if objective.progress >= objective.target {
                            objective.completed = true;
                        }
                    }
                }

                if mission.objectives.iter().all(|o| o.completed) {
                    mission.completed = true;
                    println!("✅ Mission completed: {}", mission.name);
                    self.grant_rewards(&mission.rewards);
                }
            }
        }
    }

    pub fn activate_mission(&mut self, mission_id: u32) {
        if let Some(idx) = self.missions.iter().position(|m| m.id == mission_id) {
            if self.missions[idx].unlocked && !self.missions[idx].completed {
                self.active_mission = Some(idx);
                println!("📋 Mission activated: {}", self.missions[idx].name);
            }
        }
    }

    fn grant_rewards(&mut self, rewards: &Rewards) {
        println!("🎁 Rewards granted:");
        if rewards.minerals > 0.0 {
            println!("  +{} minerals", rewards.minerals);
        }
        if rewards.computronium > 0.0 {
            println!("  +{} computronium", rewards.computronium);
        }
        if rewards.exotic_matter > 0.0 {
            println!("  +{} exotic matter", rewards.exotic_matter);
        }
        if rewards.tech_points > 0 {
            println!("  +{} tech points", rewards.tech_points);
        }

        for &unlock_id in &rewards.unlocks_missions {
            if let Some(mission) = self.missions.iter_mut().find(|m| m.id == unlock_id) {
                mission.unlocked = true;
                println!("  🔓 Unlocked: {}", mission.name);
            }
        }
    }
}

pub fn mission_update_system(
    mut missions: ResMut<MissionManager>,
    game_state: Res<crate::resources::GameState>,
) {
}
