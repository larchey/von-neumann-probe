use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEffect {
    ProbeReplicate,
    MineAsteroid,
    CombatHit,
    ProbeDestroyed,
    ThreatDestroyed,
    TechResearched,
    CathedralExpand,
    FleetCommand,
    WarpJump,
    StructureBuilt,
    LowResources,
    VictoryAchieved,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicTrack {
    MenuAmbient,
    EarlyGameExploration,
    MidGameExpansion,
    LateGameDominance,
    CombatIntense,
    Victory,
}

pub struct AudioManager {
    sfx_enabled: bool,
    music_enabled: bool,
    sfx_volume: f32,
    music_volume: f32,
    current_track: Option<MusicTrack>,
    sfx_cooldowns: HashMap<SoundEffect, f32>,
}

impl Default for AudioManager {
    fn default() -> Self {
        Self {
            sfx_enabled: true,
            music_enabled: true,
            sfx_volume: 0.7,
            music_volume: 0.5,
            current_track: Some(MusicTrack::EarlyGameExploration),
            sfx_cooldowns: HashMap::new(),
        }
    }
}

impl AudioManager {
    pub fn play_sfx(&mut self, effect: SoundEffect) {
        if !self.sfx_enabled {
            return;
        }

        if let Some(&cooldown) = self.sfx_cooldowns.get(&effect) {
            if cooldown > 0.0 {
                return;
            }
        }

        self.sfx_cooldowns.insert(effect, self.get_cooldown(effect));

        println!("[AUDIO] Playing SFX: {:?}", effect);
    }

    pub fn play_music(&mut self, track: MusicTrack) {
        if !self.music_enabled {
            return;
        }

        if self.current_track == Some(track) {
            return;
        }

        self.current_track = Some(track);
        println!("[AUDIO] Switching to music track: {:?}", track);
    }

    pub fn update(&mut self, dt: f32) {
        for cooldown in self.sfx_cooldowns.values_mut() {
            *cooldown = (*cooldown - dt).max(0.0);
        }
    }

    fn get_cooldown(&self, effect: SoundEffect) -> f32 {
        match effect {
            SoundEffect::ProbeReplicate => 0.5,
            SoundEffect::MineAsteroid => 0.2,
            SoundEffect::CombatHit => 0.1,
            SoundEffect::ProbeDestroyed => 0.3,
            SoundEffect::ThreatDestroyed => 0.3,
            SoundEffect::TechResearched => 1.0,
            SoundEffect::CathedralExpand => 1.5,
            SoundEffect::FleetCommand => 0.4,
            SoundEffect::WarpJump => 0.3,
            SoundEffect::StructureBuilt => 0.8,
            SoundEffect::LowResources => 2.0,
            SoundEffect::VictoryAchieved => 5.0,
        }
    }

    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
    }

    pub fn toggle_sfx(&mut self) {
        self.sfx_enabled = !self.sfx_enabled;
    }

    pub fn toggle_music(&mut self) {
        self.music_enabled = !self.music_enabled;
    }
}

pub fn audio_update_system(mut audio: ResMut<AudioManager>, time: Res<crate::resources::GameTime>) {
    audio.update(time.delta_secs);
}

pub fn dynamic_music_system(
    mut audio: ResMut<AudioManager>,
    game_state: Res<crate::resources::GameState>,
) {
    let new_track = if game_state.probe_count < 10 {
        MusicTrack::EarlyGameExploration
    } else if game_state.probe_count < 100 {
        MusicTrack::MidGameExpansion
    } else if game_state.threat_level > 5.0 {
        MusicTrack::CombatIntense
    } else {
        MusicTrack::LateGameDominance
    };

    audio.play_music(new_track);
}
