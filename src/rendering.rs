use bevy::prelude::*;
use crate::generation::{CathedralLayout, CathedralType};

pub struct CathedralRenderer;

impl CathedralRenderer {
    pub fn get_core_color(cathedral_type: CathedralType) -> Color {
        match cathedral_type {
            CathedralType::Computational => Color::srgb(0.2, 0.8, 1.0),
            CathedralType::Manufacturing => Color::srgb(1.0, 0.6, 0.2),
            CathedralType::Agricultural => Color::srgb(0.2, 1.0, 0.2),
            CathedralType::Military => Color::srgb(1.0, 0.2, 0.2),
            CathedralType::Scientific => Color::srgb(0.8, 0.2, 1.0),
            CathedralType::Hybrid => Color::srgb(1.0, 1.0, 0.2),
        }
    }

    pub fn get_ring_color() -> Color {
        Color::srgb(0.4, 0.4, 0.4)
    }

    pub fn get_spire_color() -> Color {
        Color::srgb(0.8, 0.8, 0.0)
    }

    pub fn get_structure_color(struct_type: &crate::components::StructureType) -> Color {
        match struct_type {
            crate::components::StructureType::Refinery => Color::srgb(0.8, 0.6, 0.2),
            crate::components::StructureType::Foundry => Color::srgb(0.9, 0.3, 0.1),
            crate::components::StructureType::Laboratory => Color::srgb(0.3, 0.7, 0.9),
            crate::components::StructureType::PowerPlant => Color::srgb(1.0, 0.8, 0.0),
            crate::components::StructureType::StorageBay => Color::srgb(0.6, 0.6, 0.6),
            crate::components::StructureType::FactoryCathedral => Color::srgb(0.8, 0.2, 0.8),
            crate::components::StructureType::DefenseTurret => Color::srgb(1.0, 0.0, 0.0),
            crate::components::StructureType::ComputeNode => Color::srgb(0.2, 0.8, 0.8),
            crate::components::StructureType::TransmissionArray => Color::srgb(0.4, 0.9, 0.4),
        }
    }
}

pub struct MinimalistTheme {
    pub background_color: Color,
    pub grid_color: Color,
    pub accent_color: Color,
    pub danger_color: Color,
    pub success_color: Color,
}

impl Default for MinimalistTheme {
    fn default() -> Self {
        Self {
            background_color: Color::srgb(0.05, 0.05, 0.08),
            grid_color: Color::srgb(0.1, 0.1, 0.15),
            accent_color: Color::srgb(0.2, 0.8, 1.0),
            danger_color: Color::srgb(1.0, 0.2, 0.2),
            success_color: Color::srgb(0.2, 1.0, 0.2),
        }
    }
}

pub fn render_background(mut clear_color: ResMut<ClearColor>, theme: Res<MinimalistTheme>) {
    clear_color.0 = theme.background_color;
}

pub struct ParticleEffect {
    pub position: Vec2,
    pub velocity: Vec2,
    pub lifetime: f32,
    pub color: Color,
    pub size: f32,
}

#[derive(Component)]
pub struct Particle {
    pub effect: ParticleEffect,
    pub elapsed: f32,
}

pub fn update_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Particle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.elapsed += time.delta_secs();

        if particle.elapsed >= particle.effect.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += particle.effect.velocity.extend(0.0) * time.delta_secs();
    }
}
