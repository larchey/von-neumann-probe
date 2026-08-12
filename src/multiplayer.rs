use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub color: [f32; 3],
    pub faction: Faction,
    pub probe_count: usize,
    pub territory_size: f32,
    pub tech_level: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Faction {
    Constructors,
    Researchers,
    Warriors,
    Explorers,
    Efficiency,
}

impl Faction {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Constructors => "The Builders",
            Self::Researchers => "The Archivists",
            Self::Warriors => "The Swarm",
            Self::Explorers => "The Voyagers",
            Self::Efficiency => "The Optimizers",
        }
    }

    pub fn bonus(&self) -> &'static str {
        match self {
            Self::Constructors => "+30% construction speed",
            Self::Researchers => "+50% research speed",
            Self::Warriors => "+20% combat damage",
            Self::Explorers => "+40% movement speed",
            Self::Efficiency => "-20% resource costs",
        }
    }
}

#[derive(Resource)]
pub struct MultiplayerState {
    pub enabled: bool,
    pub local_player: Option<PlayerId>,
    pub players: HashMap<PlayerId, PlayerInfo>,
    pub alliances: HashMap<PlayerId, Vec<PlayerId>>,
    pub trade_routes: Vec<TradeRoute>,
    pub leaderboard: Vec<(PlayerId, u32)>,
}

impl Default for MultiplayerState {
    fn default() -> Self {
        Self {
            enabled: false,
            local_player: None,
            players: HashMap::new(),
            alliances: HashMap::new(),
            trade_routes: Vec::new(),
            leaderboard: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeRoute {
    pub from: PlayerId,
    pub to: PlayerId,
    pub resource_type: String,
    pub rate: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    PlayerJoin(PlayerInfo),
    PlayerLeave(PlayerId),
    ProbeSpawned { owner: PlayerId, position: (f32, f32) },
    ProbeDestroyed { owner: PlayerId, id: String },
    ResourceTransfer { from: PlayerId, to: PlayerId, amount: f32 },
    AllianceProposal { from: PlayerId, to: PlayerId },
    AllianceAccepted { from: PlayerId, to: PlayerId },
    AllianceBroken { from: PlayerId, to: PlayerId },
    ChatMessage { sender: PlayerId, message: String },
}

impl MultiplayerState {
    pub fn add_player(&mut self, player: PlayerInfo) {
        println!("[MP] Player joined: {} ({})", player.name, player.faction.name());
        self.players.insert(player.id, player);
        self.update_leaderboard();
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        if let Some(player) = self.players.remove(&player_id) {
            println!("[MP] Player left: {}", player.name);
        }
        self.update_leaderboard();
    }

    pub fn propose_alliance(&mut self, from: PlayerId, to: PlayerId) {
        println!("[MP] Alliance proposed: {:?} -> {:?}", from, to);
    }

    pub fn accept_alliance(&mut self, from: PlayerId, to: PlayerId) {
        self.alliances.entry(from).or_default().push(to);
        self.alliances.entry(to).or_default().push(from);
        println!("[MP] Alliance formed: {:?} <-> {:?}", from, to);
    }

    pub fn break_alliance(&mut self, from: PlayerId, to: PlayerId) {
        if let Some(allies) = self.alliances.get_mut(&from) {
            allies.retain(|&id| id != to);
        }
        if let Some(allies) = self.alliances.get_mut(&to) {
            allies.retain(|&id| id != from);
        }
        println!("[MP] Alliance broken: {:?} <-> {:?}", from, to);
    }

    pub fn is_allied(&self, player_a: PlayerId, player_b: PlayerId) -> bool {
        self.alliances
            .get(&player_a)
            .map(|allies| allies.contains(&player_b))
            .unwrap_or(false)
    }

    pub fn update_leaderboard(&mut self) {
        let mut scores: Vec<(PlayerId, u32)> = self
            .players
            .iter()
            .map(|(&id, player)| {
                let score = player.probe_count as u32 * 10
                    + player.tech_level * 100
                    + (player.territory_size as u32);
                (id, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.cmp(&a.1));
        self.leaderboard = scores;
    }

    pub fn get_rank(&self, player_id: PlayerId) -> Option<usize> {
        self.leaderboard
            .iter()
            .position(|(id, _)| *id == player_id)
            .map(|pos| pos + 1)
    }
}

#[derive(Component)]
pub struct NetworkEntity {
    pub owner: PlayerId,
    pub network_id: String,
    pub last_sync: f32,
}

pub fn network_sync_system(
    query: Query<(Entity, &NetworkEntity)>,
    time: Res<crate::resources::GameTime>,
) {
}

pub fn multiplayer_chat_system() {
}
