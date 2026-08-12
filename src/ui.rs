use bevy_math::Vec2;
use crate::components::ProbeType;
use crate::resources::{GameState, Resources};

#[derive(Clone)]
pub struct Notification {
    pub message: String,
    pub severity: NotificationSeverity,
    pub timestamp: f32,
    pub lifetime: f32,
}

#[derive(Clone, Copy)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Critical,
    Success,
}

pub struct NotificationManager {
    pub notifications: Vec<Notification>,
    pub max_visible: usize,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self {
            notifications: Vec::new(),
            max_visible: 5,
        }
    }
}

impl NotificationManager {
    pub fn add(&mut self, message: String, severity: NotificationSeverity, current_time: f32) {
        let lifetime = match severity {
            NotificationSeverity::Info => 3.0,
            NotificationSeverity::Warning => 5.0,
            NotificationSeverity::Critical => 8.0,
            NotificationSeverity::Success => 4.0,
        };

        self.notifications.push(Notification {
            message,
            severity,
            timestamp: current_time,
            lifetime,
        });
    }

    pub fn update(&mut self, current_time: f32) {
        self.notifications
            .retain(|n| current_time - n.timestamp < n.lifetime);
    }

    pub fn get_visible(&self) -> Vec<&Notification> {
        self.notifications
            .iter()
            .rev()
            .take(self.max_visible)
            .collect()
    }
}

pub struct HUD {
    pub show_debug: bool,
    pub show_minimap: bool,
    pub show_tech_tree: bool,
    pub minimap_size: f32,
    pub minimap_zoom: f32,
}

impl Default for HUD {
    fn default() -> Self {
        Self {
            show_debug: true,
            show_minimap: true,
            show_tech_tree: false,
            minimap_size: 200.0,
            minimap_zoom: 0.01,
        }
    }
}

pub struct UILayout;

impl UILayout {
    pub fn render_hud(game_state: &GameState, hud: &HUD) -> String {
        let mut output = String::new();

        output.push_str("┌─────────────────────────────────────┐\n");
        output.push_str(&format!(
            "│ Probes: {:>5}  │  Minerals: {:>8.0}  │\n",
            game_state.probe_count, game_state.total_resources.minerals
        ));
        output.push_str(&format!(
            "│ Threats: {:>4}  │  Comput:   {:>8.0}  │\n",
            game_state.threats_defeated, game_state.total_resources.computronium
        ));
        output.push_str(&format!(
            "│ Sectors: {:>4}  │  Exotic:   {:>8.2}  │\n",
            game_state.sectors_explored, game_state.total_resources.exotic_matter
        ));
        output.push_str("└─────────────────────────────────────┘\n");

        if hud.show_debug {
            output.push_str(&format!(
                "[DEBUG] Threat Level: {:.2} | Research: {:.0}%\n",
                game_state.threat_level,
                game_state.research_progress * 100.0
            ));
        }

        output
    }

    pub fn render_resource_bar(resources: &Resources, max_resources: &Resources) -> String {
        let mineral_pct = (resources.minerals / max_resources.minerals * 20.0) as usize;
        let compute_pct = (resources.computronium / max_resources.computronium * 20.0) as usize;

        format!(
            "Minerals  [{}{}] {:.0}/{:.0}\nCompute   [{}{}] {:.0}/{:.0}",
            "█".repeat(mineral_pct.min(20)),
            "░".repeat(20 - mineral_pct.min(20)),
            resources.minerals,
            max_resources.minerals,
            "█".repeat(compute_pct.min(20)),
            "░".repeat(20 - compute_pct.min(20)),
            resources.computronium,
            max_resources.computronium
        )
    }

    pub fn render_probe_type_icon(probe_type: ProbeType) -> char {
        match probe_type {
            ProbeType::Scout => '◆',
            ProbeType::Miner => '■',
            ProbeType::Constructor => '●',
            ProbeType::Researcher => '◉',
            ProbeType::Warrior => '▲',
            ProbeType::Administrator => '◈',
        }
    }
}

pub struct Minimap {
    pub entities: Vec<MinimapEntity>,
}

pub struct MinimapEntity {
    pub position: Vec2,
    pub entity_type: MinimapEntityType,
}

pub enum MinimapEntityType {
    Probe,
    Threat,
    Cathedral,
    AsteroidField,
}

impl Minimap {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn update_from_world(&mut self, world: &bevy_ecs::world::World) {
        self.entities.clear();
    }

    pub fn render(&self, center: Vec2, zoom: f32, size: f32) -> String {
        let grid_size = 40;
        let mut grid = vec![vec![' '; grid_size]; grid_size];

        for entity in &self.entities {
            let rel_x = (entity.position.x - center.x) * zoom;
            let rel_y = (entity.position.y - center.y) * zoom;

            let grid_x = ((rel_x + size / 2.0) / size * grid_size as f32) as usize;
            let grid_y = ((rel_y + size / 2.0) / size * grid_size as f32) as usize;

            if grid_x < grid_size && grid_y < grid_size {
                grid[grid_y][grid_x] = match entity.entity_type {
                    MinimapEntityType::Probe => '•',
                    MinimapEntityType::Threat => '×',
                    MinimapEntityType::Cathedral => '◆',
                    MinimapEntityType::AsteroidField => '.',
                };
            }
        }

        let mut output = String::new();
        for row in &grid {
            for ch in row {
                output.push(*ch);
            }
            output.push('\n');
        }
        output
    }
}

pub fn notification_update_system(
    mut notifications: ResMut<NotificationManager>,
    time: Res<crate::resources::GameTime>,
) {
    notifications.update(time.total_secs);
}

pub fn ui_render_system(game_state: Res<GameState>, hud: Res<HUD>) {
    println!("{}", UILayout::render_hud(&game_state, &hud));
}
