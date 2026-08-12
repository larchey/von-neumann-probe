use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::components::{ColonyStructure, StructureType, Resources};

#[derive(Resource)]
pub struct CathedralGenerator {
    seed: u64,
}

impl Default for CathedralGenerator {
    fn default() -> Self {
        Self {
            seed: rand::random(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CathedralLayout {
    pub core: CathedralCore,
    pub rings: Vec<CathedralRing>,
    pub spires: Vec<Spire>,
    pub total_area: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CathedralCore {
    pub radius: f32,
    pub center: Vec2,
    pub cathedral_type: CathedralType,
    pub stability: f32,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CathedralType {
    Computational,
    Manufacturing,
    Agricultural,
    Military,
    Scientific,
    Hybrid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CathedralRing {
    pub radius: f32,
    pub ring_index: usize,
    pub structures: Vec<RingStructure>,
    pub total_slots: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RingStructure {
    pub position: Vec2,
    pub structure_type: StructureType,
    pub radial_angle: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spire {
    pub position: Vec2,
    pub height: f32,
    pub power_level: f32,
}

impl CathedralGenerator {
    pub fn generate_initial_colony(&self) -> CathedralLayout {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);

        let cathedral_types = [
            CathedralType::Computational,
            CathedralType::Manufacturing,
            CathedralType::Scientific,
        ];
        let cathedral_type = cathedral_types[rng.gen_range(0..3)];

        let core = CathedralCore {
            radius: 40.0,
            center: Vec2::ZERO,
            cathedral_type,
            stability: 1.0,
        };

        let mut rings = Vec::new();

        for ring_idx in 0..3 {
            let radius = 40.0 + (ring_idx as f32 + 1.0) * 50.0;
            let slots = 4 + (ring_idx as usize * 2);

            let mut structures = Vec::new();

            for slot in 0..slots {
                let angle = (slot as f32 / slots as f32) * std::f32::consts::TAU;
                let x = radius * angle.cos();
                let y = radius * angle.sin();

                let struct_type = match ring_idx {
                    0 => StructureType::StorageBay,
                    1 => StructureType::Foundry,
                    2 => StructureType::PowerPlant,
                    _ => StructureType::ComputeNode,
                };

                structures.push(RingStructure {
                    position: Vec2::new(x, y),
                    structure_type: struct_type,
                    radial_angle: angle,
                });
            }

            rings.push(CathedralRing {
                radius,
                ring_index: ring_idx,
                structures,
                total_slots: slots,
            });
        }

        let mut spires = Vec::new();
        for i in 0..4 {
            let angle = (i as f32 / 4.0) * std::f32::consts::TAU;
            let distance = 150.0;
            spires.push(Spire {
                position: Vec2::new(distance * angle.cos(), distance * angle.sin()),
                height: 80.0 + rng.gen::<f32>() * 40.0,
                power_level: 0.8,
            });
        }

        let total_area = std::f32::consts::PI * (core.radius.powi(2));

        CathedralLayout {
            core,
            rings,
            spires,
            total_area,
        }
    }

    pub fn expand_cathedral(&self, layout: &CathedralLayout) -> CathedralLayout {
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed.wrapping_add(1));

        let mut new_rings = layout.rings.clone();

        let new_radius = layout.rings.last().unwrap().radius + 60.0;
        let new_ring_idx = layout.rings.len();
        let slots = 6 + (new_ring_idx as usize * 3);

        let mut structures = Vec::new();
        for slot in 0..slots {
            let angle = (slot as f32 / slots as f32) * std::f32::consts::TAU;
            let x = new_radius * angle.cos();
            let y = new_radius * angle.sin();

            let struct_type = if rng.gen_bool(0.7) {
                StructureType::FactoryCathedral
            } else {
                StructureType::DefenseTurret
            };

            structures.push(RingStructure {
                position: Vec2::new(x, y),
                structure_type: struct_type,
                radial_angle: angle,
            });
        }

        new_rings.push(CathedralRing {
            radius: new_radius,
            ring_index: new_ring_idx,
            structures,
            total_slots: slots,
        });

        let mut spires = layout.spires.clone();
        let angle = (spires.len() as f32 / 8.0) * std::f32::consts::TAU;
        let distance = 150.0 + (new_ring_idx as f32 * 30.0);
        spires.push(Spire {
            position: Vec2::new(distance * angle.cos(), distance * angle.sin()),
            height: 100.0 + rng.gen::<f32>() * 60.0,
            power_level: 0.9,
        });

        let total_area = std::f32::consts::PI * (new_radius.powi(2));

        CathedralLayout {
            core: layout.core.clone(),
            rings: new_rings,
            spires,
            total_area,
        }
    }

    pub fn generate_sector(&self, sector_x: i32, sector_y: i32) -> SectorLayout {
        let mut rng = rand::rngs::StdRng::seed_from_u64(
            self.seed.wrapping_add((sector_x as u64) << 32 | (sector_y as u64)),
        );

        let cathedral_count = rng.gen_range(0..3);
        let mut cathedrals = Vec::new();

        for _ in 0..cathedral_count {
            let x = (sector_x as f32) * 1000.0 + rng.gen::<f32>() * 800.0;
            let y = (sector_y as f32) * 1000.0 + rng.gen::<f32>() * 800.0;

            let cathedral = CathedralCore {
                radius: 30.0 + rng.gen::<f32>() * 40.0,
                center: Vec2::new(x, y),
                cathedral_type: match rng.gen_range(0..6) {
                    0 => CathedralType::Computational,
                    1 => CathedralType::Manufacturing,
                    2 => CathedralType::Agricultural,
                    3 => CathedralType::Military,
                    4 => CathedralType::Scientific,
                    _ => CathedralType::Hybrid,
                },
                stability: 0.5 + rng.gen::<f32>() * 0.5,
            };

            cathedrals.push(cathedral);
        }

        SectorLayout {
            sector_x,
            sector_y,
            cathedrals,
            asteroid_fields: Vec::new(),
            anomalies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectorLayout {
    pub sector_x: i32,
    pub sector_y: i32,
    pub cathedrals: Vec<CathedralCore>,
    pub asteroid_fields: Vec<AsteroidFieldData>,
    pub anomalies: Vec<SectorAnomaly>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AsteroidFieldData {
    pub position: Vec2,
    pub density: f32,
    pub mineral_value: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SectorAnomaly {
    Nebula { position: Vec2, intensity: f32 },
    BlackHole { position: Vec2, radius: f32 },
    EnergyStorm { position: Vec2, power: f32 },
}
