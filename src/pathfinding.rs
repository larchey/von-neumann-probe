use bevy_math::Vec2;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq)]
struct Node {
    position: Vec2,
    g_cost: f32,
    h_cost: f32,
}

impl Node {
    fn f_cost(&self) -> f32 {
        self.g_cost + self.h_cost
    }
}

impl Eq for Node {}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_cost()
            .partial_cmp(&self.f_cost())
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Pathfinder {
    grid_size: f32,
}

impl Default for Pathfinder {
    fn default() -> Self {
        Self { grid_size: 100.0 }
    }
}

impl Pathfinder {
    pub fn find_path(
        &self,
        start: Vec2,
        goal: Vec2,
        obstacles: &HashSet<(i32, i32)>,
        max_iterations: usize,
    ) -> Option<Vec<Vec2>> {
        if start.distance(goal) < 10.0 {
            return Some(vec![start, goal]);
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut g_score: HashMap<(i32, i32), f32> = HashMap::new();

        let start_grid = self.world_to_grid(start);
        let goal_grid = self.world_to_grid(goal);

        g_score.insert(start_grid, 0.0);
        open_set.push(Node {
            position: start,
            g_cost: 0.0,
            h_cost: self.heuristic(start, goal),
        });

        let mut iterations = 0;

        while let Some(current_node) = open_set.pop() {
            iterations += 1;
            if iterations > max_iterations {
                break;
            }

            let current_grid = self.world_to_grid(current_node.position);

            if current_grid == goal_grid {
                return Some(self.reconstruct_path(came_from, current_grid, start_grid));
            }

            for neighbor_grid in self.get_neighbors(current_grid) {
                if obstacles.contains(&neighbor_grid) {
                    continue;
                }

                let neighbor_pos = self.grid_to_world(neighbor_grid);
                let tentative_g = current_node.g_cost + current_node.position.distance(neighbor_pos);

                if tentative_g < *g_score.get(&neighbor_grid).unwrap_or(&f32::INFINITY) {
                    came_from.insert(neighbor_grid, current_grid);
                    g_score.insert(neighbor_grid, tentative_g);
                    open_set.push(Node {
                        position: neighbor_pos,
                        g_cost: tentative_g,
                        h_cost: self.heuristic(neighbor_pos, goal),
                    });
                }
            }
        }

        None
    }

    fn heuristic(&self, a: Vec2, b: Vec2) -> f32 {
        a.distance(b)
    }

    fn world_to_grid(&self, pos: Vec2) -> (i32, i32) {
        (
            (pos.x / self.grid_size).floor() as i32,
            (pos.y / self.grid_size).floor() as i32,
        )
    }

    fn grid_to_world(&self, grid: (i32, i32)) -> Vec2 {
        Vec2::new(
            grid.0 as f32 * self.grid_size + self.grid_size / 2.0,
            grid.1 as f32 * self.grid_size + self.grid_size / 2.0,
        )
    }

    fn get_neighbors(&self, grid: (i32, i32)) -> Vec<(i32, i32)> {
        vec![
            (grid.0 + 1, grid.1),
            (grid.0 - 1, grid.1),
            (grid.0, grid.1 + 1),
            (grid.0, grid.1 - 1),
            (grid.0 + 1, grid.1 + 1),
            (grid.0 + 1, grid.1 - 1),
            (grid.0 - 1, grid.1 + 1),
            (grid.0 - 1, grid.1 - 1),
        ]
    }

    fn reconstruct_path(
        &self,
        came_from: HashMap<(i32, i32), (i32, i32)>,
        mut current: (i32, i32),
        start: (i32, i32),
    ) -> Vec<Vec2> {
        let mut path = vec![self.grid_to_world(current)];

        while current != start {
            if let Some(&prev) = came_from.get(&current) {
                current = prev;
                path.push(self.grid_to_world(current));
            } else {
                break;
            }
        }

        path.reverse();
        path
    }
}

#[derive(bevy_ecs::component::Component)]
pub struct Path {
    pub waypoints: Vec<Vec2>,
    pub current_index: usize,
}

impl Path {
    pub fn new(waypoints: Vec<Vec2>) -> Self {
        Self {
            waypoints,
            current_index: 0,
        }
    }

    pub fn current_waypoint(&self) -> Option<Vec2> {
        self.waypoints.get(self.current_index).copied()
    }

    pub fn advance(&mut self) {
        self.current_index += 1;
    }

    pub fn is_complete(&self) -> bool {
        self.current_index >= self.waypoints.len()
    }
}

pub fn pathfinding_system(
    mut query: Query<(&crate::components::Position, &crate::components::Velocity, &mut Path)>,
) {
    for (position, velocity, mut path) in query.iter_mut() {
        if let Some(waypoint) = path.current_waypoint() {
            if position.0.distance(waypoint) < 20.0 {
                path.advance();
            }
        }
    }
}
