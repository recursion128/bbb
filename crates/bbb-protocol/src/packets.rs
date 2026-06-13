pub mod chat;
pub mod chunks;
pub mod client_audio;
pub mod client_common;
pub mod client_features;
pub mod client_state;
pub mod client_ui;
pub mod command_suggestions;
pub mod commands;
pub mod connection;
mod data_components;
pub mod debug_game;
pub mod entities;
mod exports;
pub mod inventory;
pub mod maps;
pub mod merchant;
pub mod movement;
pub mod play_clientbound;
pub mod player_actions;
pub mod player_info;
pub mod scoreboard;
pub mod server_presentation;
pub mod serverbound;
pub mod tags;
pub mod waypoints;
mod wire;
pub mod world_border;
pub mod world_effects;

pub use exports::*;
pub(crate) use wire::*;

#[cfg(test)]
mod tests;
