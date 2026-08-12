use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use crate::generation::{CathedralGenerator, SectorLayout};
use crate::components::AsteroidField;

#[derive(Resource)]
pub struct SectorManager {
    pub loaded_sectors: HashMap<(i32, i32), SectorLayout>,
    pub active_sectors: HashSet<(i32, i32)>,
    pub sector_size: f32,
}

impl Default for SectorManager {
    fn default() -> Self {
        Self {
            loaded_sectors: HashMap::new(),
            active_sectors: HashSet::new(),
            sector_size: 1000.0,
        }
    }
}

impl SectorManager {
    pub fn world_to_sector(&self, position: Vec2) -> (i32, i32) {
        (
            (position.x / self.sector_size).floor() as i32,
            (position.y / self.sector_size).floor() as i32,
        )
    }

    pub fn sector_to_world_center(&self, sector_x: i32, sector_y: i32) -> Vec2 {
        Vec2::new(
            (sector_x as f32 + 0.5) * self.sector_size,
            (sector_y as f32 + 0.5) * self.sector_size,
        )
    }

    pub fn get_visible_sectors(&self, center: Vec2, view_radius: f32) -> Vec<(i32, i32)> {
        let center_sector = self.world_to_sector(center);
        let sector_radius = (view_radius / self.sector_size).ceil() as i32 + 1;

        let mut sectors = Vec::new();
        for dx in -sector_radius..=sector_radius {
            for dy in -sector_radius..=sector_radius {
                sectors.push((center_sector.0 + dx, center_sector.1 + dy));
            }
        }

        sectors
    }
}

pub fn stream_sectors(
    mut commands: Commands,
    mut sector_manager: ResMut<SectorManager>,
    generator: Res<CathedralGenerator>,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    let camera_pos = camera_query
        .get_single()
        .map(|t| t.translation.xy())
        .unwrap_or(Vec2::ZERO);

    let visible_sectors = sector_manager.get_visible_sectors(camera_pos, 2000.0);
    let visible_set: HashSet<(i32, i32)> = visible_sectors.iter().copied().collect();

    for sector_coords in visible_sectors.iter() {
        if !sector_manager.loaded_sectors.contains_key(sector_coords) {
            let layout = generator.generate_sector(sector_coords.0, sector_coords.1);

            for asteroid_data in layout.asteroid_fields.iter() {
                commands.spawn((
                    Transform::from_xyz(asteroid_data.position.x, asteroid_data.position.y, 0.0),
                    AsteroidField {
                        position: asteroid_data.position,
                        size: asteroid_data.density * 50.0,
                        mineral_density: asteroid_data.mineral_value,
                        resources: crate::components::Resources {
                            minerals: asteroid_data.mineral_value * 100.0,
                            computronium: asteroid_data.mineral_value * 20.0,
                            exotic_matter: 0.0,
                        },
                    },
                    Sprite::from_color(
                        Color::srgb(0.5, 0.5, 0.5),
                        Vec2::splat(asteroid_data.density * 50.0),
                    ),
                ));
            }

            sector_manager.loaded_sectors.insert(*sector_coords, layout);
        }
    }

    sector_manager.active_sectors = visible_set;
}

pub fn update_sector_entities(
    mut commands: Commands,
    sector_manager: Res<SectorManager>,
    asteroid_query: Query<(Entity, &Transform), With<AsteroidField>>,
) {
    for (entity, transform) in asteroid_query.iter() {
        let sector = sector_manager.world_to_sector(transform.translation.xy());

        if !sector_manager.active_sectors.contains(&sector) {
            commands.entity(entity).despawn();
        }
    }
}
