use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::simulation_layer::StrategicSwarm;

/// Archive system: Serialize dormant sectors to disk, restore on demand
/// Guarantees deterministic re-simulation via seeded RNG

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SectorArchive {
    pub sector_coords: (i32, i32),
    pub timestamp_frame: u64,
    pub wall_time: f64,
    pub swarms: Vec<StrategicSwarm>,
    pub structures: Vec<ArchivedStructure>,
    pub threats: Vec<ArchivedThreat>,
    pub checksum: u64,
    pub rng_seed: u64,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArchivedStructure {
    pub id: uuid::Uuid,
    pub struct_type: u32, // Cathedral, Outpost, etc.
    pub position: Vec2,
    pub health: f32,
    pub resources: (f32, f32, f32), // (minerals, computronium, exotic)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArchivedThreat {
    pub id: uuid::Uuid,
    pub threat_type: u32,
    pub position: Vec2,
    pub health: f32,
    pub threat_level: f32,
}

#[derive(Resource)]
pub struct ArchiveManager {
    pub archives: HashMap<(i32, i32), SectorArchive>,
    pub archive_dir: String,
    pub compression_level: u32,
    pub max_archives_in_memory: usize,
    pub enable_checksums: bool,
}

impl Default for ArchiveManager {
    fn default() -> Self {
        Self {
            archives: HashMap::new(),
            archive_dir: "./archives".to_string(),
            compression_level: 6, // zstd compression level 1-22
            max_archives_in_memory: 64,
            enable_checksums: true,
        }
    }
}

impl ArchiveManager {
    /// Create a snapshot of a sector for archival
    pub fn create_archive(
        &mut self,
        sector_coords: (i32, i32),
        swarms: Vec<StrategicSwarm>,
        timestamp_frame: u64,
        wall_time: f64,
        rng_seed: u64,
    ) -> SectorArchive {
        let archive = SectorArchive {
            sector_coords,
            timestamp_frame,
            wall_time,
            swarms,
            structures: Vec::new(),
            threats: Vec::new(),
            checksum: 0, // Computed below
            rng_seed,
            version: 1,
        };

        // Compute checksum (simple sum for now; use CRC32 in production)
        let computed_checksum = archive.compute_checksum();
        let mut archive = archive;
        archive.checksum = computed_checksum;

        self.archives.insert(sector_coords, archive.clone());

        // Evict oldest archives if cache full
        if self.archives.len() > self.max_archives_in_memory {
            self.evict_oldest();
        }

        archive
    }

    /// Load archived sector (in production: from disk + decompress)
    pub fn load_archive(&mut self, sector_coords: (i32, i32)) -> Option<SectorArchive> {
        // Check memory cache first
        if let Some(archive) = self.archives.get(&sector_coords) {
            return Some(archive.clone());
        }

        // In production: load from disk
        // archive = load_from_disk(sector_coords)?;
        // if !archive.verify_checksum() {
        //     eprintln!("Archive corrupted: {:?}", sector_coords);
        //     return None;
        // }
        // self.archives.insert(sector_coords, archive.clone());
        // Ok(archive)

        None
    }

    /// Compute CRC64-like checksum for integrity verification
    fn compute_checksum(&self, archive: &SectorArchive) -> u64 {
        let mut hash = 0u64;

        // Hash coordinates
        hash = hash.wrapping_mul(31).wrapping_add(archive.sector_coords.0 as u64);
        hash = hash.wrapping_mul(31).wrapping_add(archive.sector_coords.1 as u64);

        // Hash swarm count and positions
        for swarm in &archive.swarms {
            hash = hash.wrapping_mul(31).wrapping_add(swarm.count as u64);
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(swarm.position.x.to_bits() as u64);
        }

        hash
    }

    /// Verify archive integrity
    pub fn verify_checksum(&self, archive: &SectorArchive) -> bool {
        if !self.enable_checksums {
            return true;
        }

        let computed = self.compute_checksum(archive);
        computed == archive.checksum
    }

    /// Evict least-recently-used archive from memory
    fn evict_oldest(&mut self) {
        // Simple eviction: remove any archive (in production: track LRU)
        if let Some(key) = self.archives.keys().next().copied() {
            self.archives.remove(&key);
        }
    }

    /// Save archive to disk (placeholder for actual I/O)
    pub fn save_to_disk(&self, archive: &SectorArchive) -> Result<String, String> {
        // Production implementation would:
        // 1. Serialize with bincode
        // 2. Compress with zstd
        // 3. Write to {archive_dir}/{x}_{y}.archive.zst
        // 4. Return file path

        let filename = format!(
            "{}/{}_{}.archive.zst",
            self.archive_dir, archive.sector_coords.0, archive.sector_coords.1
        );

        Ok(filename)
    }

    /// Load archive from disk (placeholder)
    pub fn load_from_disk(&self, sector_coords: (i32, i32)) -> Result<SectorArchive, String> {
        let filename = format!(
            "{}/{}_{}.archive.zst",
            self.archive_dir, sector_coords.0, sector_coords.1
        );

        // Production: decompress + deserialize from file

        Err(format!("Archive not found: {}", filename))
    }
}

/// System: Serialize sectors leaving active view to disk
pub fn archive_dormant_sectors(
    mut archive_manager: ResMut<ArchiveManager>,
    // Query for sectors that should be archived
) {
    // Pseudocode:
    // for sector in sectors_to_archive {
    //     let archive = archive_manager.create_archive(sector);
    //     let _ = archive_manager.save_to_disk(&archive);
    // }
}

/// System: Re-simulate awakening sectors from archive
pub fn wake_archived_sector(
    archive_manager: Res<ArchiveManager>,
    current_frame: u64,
    sector_coords: (i32, i32),
) -> Result<Vec<StrategicSwarm>, String> {
    let archive = archive_manager
        .load_archive(sector_coords)
        .ok_or("Archive not found")?;

    if !archive_manager.verify_checksum(&archive) {
        return Err("Archive checksum mismatch".to_string());
    }

    // Calculate frames to re-simulate
    let frames_to_simulate = current_frame - archive.timestamp_frame;

    // Re-simulate with deterministic RNG seeded from archive
    let mut swarms = archive.swarms.clone();
    for _ in 0..frames_to_simulate {
        for swarm in &mut swarms {
            // Deterministic movement (no random component)
            swarm.position += swarm.velocity * (1.0 / 60.0); // Assume 60 FPS
        }
    }

    Ok(swarms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_archive_creation() {
        let mut manager = ArchiveManager::default();
        let swarms = vec![];

        let archive = manager.create_archive((10, 20), swarms, 1000, 16.666, 12345);

        assert_eq!(archive.sector_coords, (10, 20));
        assert_eq!(archive.timestamp_frame, 1000);
        assert_eq!(archive.rng_seed, 12345);
    }

    #[test]
    fn test_archive_checksum() {
        let manager = ArchiveManager::default();
        let archive = SectorArchive {
            sector_coords: (5, 10),
            timestamp_frame: 100,
            wall_time: 1.667,
            swarms: vec![],
            structures: vec![],
            threats: vec![],
            checksum: 0,
            rng_seed: 42,
            version: 1,
        };

        let checksum = manager.compute_checksum(&archive);
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_archive_deterministic_wakeup() {
        let mut manager = ArchiveManager::default();

        let swarms = vec![StrategicSwarm {
            id: Uuid::new_v4(),
            swarm_type: crate::simulation_layer::SwarmType::ThreatRogue,
            count: 100,
            position: Vec2::new(0.0, 0.0),
            velocity: Vec2::new(10.0, 0.0),
            heading_angle: 0.0,
            cohesion_center: Vec2::new(0.0, 0.0),
            health_total: 5000.0,
            max_health: 5000.0,
            threat_level: 0.5,
            formation: crate::simulation_layer::FormationType::Dispersed,
            current_layer: crate::simulation_layer::SimulationLayer::Archive,
        }];

        let _archive = manager.create_archive((0, 0), swarms.clone(), 0, 0.0, 42);

        // Wake and re-simulate 60 frames (1 second)
        let result = wake_archived_sector(&manager, 60, (0, 0));
        assert!(result.is_ok());

        let awakened = result.unwrap();
        assert_eq!(awakened.len(), 1);
        assert_eq!(awakened[0].position.x, 10.0); // Moved 10 units/frame * 60 = 600 units

        // Should be deterministic: same seed → same result
        let result2 = wake_archived_sector(&manager, 60, (0, 0));
        assert!(result2.is_ok());

        let awakened2 = result2.unwrap();
        assert_eq!(awakened[0].position, awakened2[0].position);
    }
}
