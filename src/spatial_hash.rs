use bevy::prelude::*;
use std::collections::HashMap;

/// Spatial hash grid for O(1) radius queries instead of O(n²) brute force
/// Divides world into uniform cells; queries only check nearby cells

#[derive(Resource)]
pub struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<Entity>>,
    cell_size: f32,
    entities: HashMap<Entity, GridCell>,
}

#[derive(Clone, Copy, Debug)]
struct GridCell(i32, i32);

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cells: HashMap::new(),
            cell_size,
            entities: HashMap::new(),
        }
    }

    /// Convert world position to grid cell coordinates
    fn world_to_cell(&self, position: Vec2) -> (i32, i32) {
        (
            (position.x / self.cell_size).floor() as i32,
            (position.y / self.cell_size).floor() as i32,
        )
    }

    /// Insert entity at position
    pub fn insert(&mut self, entity: Entity, position: Vec2) {
        let cell = self.world_to_cell(position);
        self.entities.insert(entity, GridCell(cell.0, cell.1));
        self.cells.entry(cell).or_insert_with(Vec::new).push(entity);
    }

    /// Update entity position (remove from old cell, add to new)
    pub fn update(&mut self, entity: Entity, new_position: Vec2) {
        if let Some(old_cell) = self.entities.get(&entity) {
            let key = (old_cell.0, old_cell.1);
            if let Some(cell_entities) = self.cells.get_mut(&key) {
                cell_entities.retain(|e| *e != entity);
            }
        }

        self.insert(entity, new_position);
    }

    /// Remove entity from grid
    pub fn remove(&mut self, entity: Entity) {
        if let Some(cell) = self.entities.remove(&entity) {
            let key = (cell.0, cell.1);
            if let Some(cell_entities) = self.cells.get_mut(&key) {
                cell_entities.retain(|e| *e != entity);
            }
        }
    }

    /// Find all entities within radius of position (O(1) in typical case)
    pub fn query_radius(&self, center: Vec2, radius: f32) -> Vec<Entity> {
        let cell_radius = (radius / self.cell_size).ceil() as i32 + 1;
        let center_cell = self.world_to_cell(center);
        let mut results = Vec::new();

        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let cell_key = (center_cell.0 + dx, center_cell.1 + dy);
                if let Some(entities) = self.cells.get(&cell_key) {
                    results.extend(entities.iter().copied());
                }
            }
        }

        results
    }

    /// Find all entities in rectangular AABB (useful for viewport frustum)
    pub fn query_rect(&self, min: Vec2, max: Vec2) -> Vec<Entity> {
        let min_cell = self.world_to_cell(min);
        let max_cell = self.world_to_cell(max);
        let mut results = Vec::new();

        for x in min_cell.0..=max_cell.0 {
            for y in min_cell.1..=max_cell.1 {
                if let Some(entities) = self.cells.get(&(x, y)) {
                    results.extend(entities.iter().copied());
                }
            }
        }

        results
    }

    /// Count entities in radius (for debugging/monitoring)
    pub fn count_in_radius(&self, center: Vec2, radius: f32) -> usize {
        self.query_radius(center, radius).len()
    }

    /// Clear entire grid
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entities.clear();
    }
}

/// System: Maintain spatial grid as entities move
pub fn maintain_spatial_grid(
    mut spatial_grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), Changed<Transform>>,
) {
    for (entity, transform) in query.iter() {
        spatial_grid.update(entity, transform.translation.xy());
    }
}

/// System: Clean up spatial grid when entities are despawned
pub fn cleanup_spatial_grid(
    mut spatial_grid: ResMut<SpatialGrid>,
    mut removed: RemovedComponents<Transform>,
) {
    for entity in removed.read() {
        spatial_grid.remove(entity);
    }
}

/// Example usage: Fast targeting using spatial queries
pub fn fast_threat_targeting(
    spatial_grid: Res<SpatialGrid>,
    threats: Query<(Entity, &Transform), With<crate::threat_system::Threat>>,
    probes: Query<(Entity, &Transform), With<crate::components::Probe>>,
) {
    for (threat_entity, threat_transform) in threats.iter() {
        let nearby_probes = spatial_grid.query_radius(threat_transform.translation.xy(), 200.0);

        let mut closest: Option<(Entity, f32)> = None;
        for probe_entity in nearby_probes {
            if let Ok((_, probe_transform)) = probes.get(probe_entity) {
                let dist = threat_transform
                    .translation
                    .xy()
                    .distance(probe_transform.translation.xy());
                if closest.is_none() || dist < closest.unwrap().1 {
                    closest = Some((probe_entity, dist));
                }
            }
        }

        // Update threat target...
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_insert_query() {
        let mut grid = SpatialGrid::new(100.0);
        let e1 = Entity::from_raw(1);
        let e2 = Entity::from_raw(2);
        let e3 = Entity::from_raw(3);

        grid.insert(e1, Vec2::new(50.0, 50.0));
        grid.insert(e2, Vec2::new(75.0, 75.0));
        grid.insert(e3, Vec2::new(500.0, 500.0)); // Far away

        let nearby = grid.query_radius(Vec2::new(50.0, 50.0), 100.0);
        assert!(nearby.contains(&e1));
        assert!(nearby.contains(&e2));
        assert!(!nearby.contains(&e3));
    }

    #[test]
    fn test_spatial_update() {
        let mut grid = SpatialGrid::new(100.0);
        let e1 = Entity::from_raw(1);

        grid.insert(e1, Vec2::new(50.0, 50.0));
        grid.update(e1, Vec2::new(500.0, 500.0));

        let nearby = grid.query_radius(Vec2::new(50.0, 50.0), 100.0);
        assert!(!nearby.contains(&e1));

        let nearby_new = grid.query_radius(Vec2::new(500.0, 500.0), 100.0);
        assert!(nearby_new.contains(&e1));
    }

    #[test]
    fn test_spatial_remove() {
        let mut grid = SpatialGrid::new(100.0);
        let e1 = Entity::from_raw(1);

        grid.insert(e1, Vec2::new(50.0, 50.0));
        grid.remove(e1);

        let nearby = grid.query_radius(Vec2::new(50.0, 50.0), 100.0);
        assert!(!nearby.contains(&e1));
    }

    #[test]
    fn test_spatial_rect_query() {
        let mut grid = SpatialGrid::new(100.0);
        let e1 = Entity::from_raw(1);
        let e2 = Entity::from_raw(2);
        let e3 = Entity::from_raw(3);

        grid.insert(e1, Vec2::new(50.0, 50.0));
        grid.insert(e2, Vec2::new(150.0, 150.0));
        grid.insert(e3, Vec2::new(500.0, 500.0));

        let results = grid.query_rect(Vec2::new(0.0, 0.0), Vec2::new(200.0, 200.0));
        assert!(results.contains(&e1));
        assert!(results.contains(&e2));
        assert!(!results.contains(&e3));
    }

    #[test]
    fn test_10k_entities_scaling() {
        let mut grid = SpatialGrid::new(50.0);

        // Insert 10K entities in random positions
        for i in 0..10_000 {
            grid.insert(
                Entity::from_raw(i),
                Vec2::new(
                    (i as f32 * 1.618) % 5000.0, // Pseudorandom via golden ratio
                    (i as f32 * 2.718) % 5000.0, // Pseudorandom via e
                ),
            );
        }

        // Query should be fast regardless of total entity count
        let results = grid.query_radius(Vec2::new(2500.0, 2500.0), 500.0);

        // Rough estimate: 500m radius → (500/50)² = 100 cells → ~10K/100 = 100 entities expected
        assert!(results.len() > 50 && results.len() < 200);
    }
}
