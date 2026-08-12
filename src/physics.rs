use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct SpatialGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<Entity>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, position: Vec2, entity: Entity) {
        let cell = self.world_to_cell(position);
        self.cells.entry(cell).or_insert_with(Vec::new).push(entity);
    }

    pub fn query_neighbors(&self, position: Vec2, radius: f32) -> Vec<Entity> {
        let cell = self.world_to_cell(position);
        let search_radius = (radius / self.cell_size).ceil() as i32;

        let mut results = Vec::new();

        for dx in -search_radius..=search_radius {
            for dy in -search_radius..=search_radius {
                let neighbor_cell = (cell.0 + dx, cell.1 + dy);
                if let Some(entities) = self.cells.get(&neighbor_cell) {
                    results.extend(entities);
                }
            }
        }

        results
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    fn world_to_cell(&self, position: Vec2) -> (i32, i32) {
        (
            (position.x / self.cell_size).floor() as i32,
            (position.y / self.cell_size).floor() as i32,
        )
    }
}

pub struct BoundingBox {
    pub min: Vec2,
    pub max: Vec2,
}

impl BoundingBox {
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.min.x && point.x <= self.max.x
            && point.y >= self.min.y && point.y <= self.max.y
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    pub fn distance_to_point(&self, point: Vec2) -> f32 {
        let dx = (self.min.x - point.x).max(0.0).max(point.x - self.max.x);
        let dy = (self.min.y - point.y).max(0.0).max(point.y - self.max.y);
        (dx * dx + dy * dy).sqrt()
    }
}

pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    pub fn contains_point(&self, point: Vec2) -> bool {
        self.center.distance(point) <= self.radius
    }

    pub fn intersects_circle(&self, other: &Circle) -> bool {
        self.center.distance(other.center) <= self.radius + other.radius
    }

    pub fn intersects_box(&self, bbox: &BoundingBox) -> bool {
        bbox.distance_to_point(self.center) <= self.radius
    }
}

#[derive(Default)]
pub struct Quadtree {
    root: Option<Box<QuadtreeNode>>,
}

struct QuadtreeNode {
    bounds: BoundingBox,
    entities: Vec<Entity>,
    children: Option<[Box<QuadtreeNode>; 4]>,
    max_entities: usize,
    max_depth: usize,
    current_depth: usize,
}

impl Quadtree {
    pub fn new(bounds: BoundingBox, max_entities: usize, max_depth: usize) -> Self {
        Self {
            root: Some(Box::new(QuadtreeNode {
                bounds,
                entities: Vec::new(),
                children: None,
                max_entities,
                max_depth,
                current_depth: 0,
            })),
        }
    }

    pub fn insert(&mut self, position: Vec2, entity: Entity) {
        if let Some(ref mut root) = self.root {
            root.insert(position, entity);
        }
    }

    pub fn query_range(&self, bounds: &BoundingBox) -> Vec<Entity> {
        if let Some(ref root) = self.root {
            root.query_range(bounds)
        } else {
            Vec::new()
        }
    }

    pub fn clear(&mut self) {
        self.root = None;
    }
}

impl QuadtreeNode {
    fn insert(&mut self, position: Vec2, entity: Entity) {
        if !self.bounds.contains_point(position) {
            return;
        }

        if self.children.is_none() && self.entities.len() >= self.max_entities
            && self.current_depth < self.max_depth
        {
            self.subdivide();
        }

        if let Some(ref mut children) = self.children {
            let index = self.get_child_index(position);
            children[index].insert(position, entity);
        } else {
            self.entities.push(entity);
        }
    }

    fn query_range(&self, bounds: &BoundingBox) -> Vec<Entity> {
        let mut results = Vec::new();

        if !self.bounds.intersects(bounds) {
            return results;
        }

        results.extend(self.entities.iter());

        if let Some(ref children) = self.children {
            for child in children.iter() {
                results.extend(child.query_range(bounds));
            }
        }

        results
    }

    fn subdivide(&mut self) {
        let width = (self.bounds.max.x - self.bounds.min.x) / 2.0;
        let height = (self.bounds.max.y - self.bounds.min.y) / 2.0;
        let x = self.bounds.min.x;
        let y = self.bounds.min.y;

        let quads = [
            BoundingBox {
                min: Vec2::new(x, y),
                max: Vec2::new(x + width, y + height),
            },
            BoundingBox {
                min: Vec2::new(x + width, y),
                max: Vec2::new(x + 2.0 * width, y + height),
            },
            BoundingBox {
                min: Vec2::new(x, y + height),
                max: Vec2::new(x + width, y + 2.0 * height),
            },
            BoundingBox {
                min: Vec2::new(x + width, y + height),
                max: Vec2::new(x + 2.0 * width, y + 2.0 * height),
            },
        ];

        self.children = Some([
            Box::new(QuadtreeNode {
                bounds: quads[0].clone(),
                entities: Vec::new(),
                children: None,
                max_entities: self.max_entities,
                max_depth: self.max_depth,
                current_depth: self.current_depth + 1,
            }),
            Box::new(QuadtreeNode {
                bounds: quads[1].clone(),
                entities: Vec::new(),
                children: None,
                max_entities: self.max_entities,
                max_depth: self.max_depth,
                current_depth: self.current_depth + 1,
            }),
            Box::new(QuadtreeNode {
                bounds: quads[2].clone(),
                entities: Vec::new(),
                children: None,
                max_entities: self.max_entities,
                max_depth: self.max_depth,
                current_depth: self.current_depth + 1,
            }),
            Box::new(QuadtreeNode {
                bounds: quads[3].clone(),
                entities: Vec::new(),
                children: None,
                max_entities: self.max_entities,
                max_depth: self.max_depth,
                current_depth: self.current_depth + 1,
            }),
        ]);
    }

    fn get_child_index(&self, position: Vec2) -> usize {
        let mid_x = (self.bounds.min.x + self.bounds.max.x) / 2.0;
        let mid_y = (self.bounds.min.y + self.bounds.max.y) / 2.0;

        if position.x < mid_x {
            if position.y < mid_y { 0 } else { 2 }
        } else if position.y < mid_y {
            1
        } else {
            3
        }
    }
}

impl Clone for BoundingBox {
    fn clone(&self) -> Self {
        Self {
            min: self.min,
            max: self.max,
        }
    }
}
