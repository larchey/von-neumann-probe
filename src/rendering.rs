use crate::generation::{CathedralLayout, CathedralType};

#[derive(Clone, Copy)]
pub struct RGB {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

pub struct CathedralRenderer;

impl CathedralRenderer {
    pub fn get_core_color(cathedral_type: CathedralType) -> RGB {
        match cathedral_type {
            CathedralType::Computational => RGB { r: 0.2, g: 0.8, b: 1.0 },
            CathedralType::Manufacturing => RGB { r: 1.0, g: 0.6, b: 0.2 },
            CathedralType::Agricultural => RGB { r: 0.2, g: 1.0, b: 0.2 },
            CathedralType::Military => RGB { r: 1.0, g: 0.2, b: 0.2 },
            CathedralType::Scientific => RGB { r: 0.8, g: 0.2, b: 1.0 },
            CathedralType::Hybrid => RGB { r: 1.0, g: 1.0, b: 0.2 },
        }
    }

    pub fn get_ring_color() -> RGB {
        RGB { r: 0.4, g: 0.4, b: 0.4 }
    }

    pub fn get_spire_color() -> RGB {
        RGB { r: 0.8, g: 0.8, b: 0.0 }
    }

    pub fn get_structure_color(struct_type: &crate::components::StructureType) -> RGB {
        match struct_type {
            crate::components::StructureType::Refinery => RGB { r: 0.8, g: 0.6, b: 0.2 },
            crate::components::StructureType::Foundry => RGB { r: 0.9, g: 0.3, b: 0.1 },
            crate::components::StructureType::Laboratory => RGB { r: 0.3, g: 0.7, b: 0.9 },
            crate::components::StructureType::PowerPlant => RGB { r: 1.0, g: 0.8, b: 0.0 },
            crate::components::StructureType::StorageBay => RGB { r: 0.6, g: 0.6, b: 0.6 },
            crate::components::StructureType::FactoryCathedral => RGB { r: 0.8, g: 0.2, b: 0.8 },
            crate::components::StructureType::DefenseTurret => RGB { r: 1.0, g: 0.0, b: 0.0 },
            crate::components::StructureType::ComputeNode => RGB { r: 0.2, g: 0.8, b: 0.8 },
            crate::components::StructureType::TransmissionArray => RGB { r: 0.4, g: 0.9, b: 0.4 },
        }
    }
}

pub struct MinimalistTheme {
    pub background: RGB,
    pub grid: RGB,
    pub accent: RGB,
    pub danger: RGB,
    pub success: RGB,
}

impl Default for MinimalistTheme {
    fn default() -> Self {
        Self {
            background: RGB { r: 0.05, g: 0.05, b: 0.08 },
            grid: RGB { r: 0.1, g: 0.1, b: 0.15 },
            accent: RGB { r: 0.2, g: 0.8, b: 1.0 },
            danger: RGB { r: 1.0, g: 0.2, b: 0.2 },
            success: RGB { r: 0.2, g: 1.0, b: 0.2 },
        }
    }
}

pub struct ParticleEffect {
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub lifetime: f32,
    pub color: RGB,
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
