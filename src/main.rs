mod client;
mod game;
mod game_types;
mod packets;
mod ui;

use std::{ops::Deref, sync::mpsc::channel, thread, time::Duration};

use crate::{
    game::{ChaserGame, ChaserHandle, GamePhase, GameState},
    game_types::Direction,
    packets::C2SPacket,
};

fn main() {
    let handle = ChaserGame::join("crystal", "Practice1");
    loop {
        if !matches!(handle.info().phase, GamePhase::Ended { .. }) {
            handle.send(C2SPacket::MovePlayer(Direction::Top));
            thread::sleep(Duration::from_millis(200));
        } else {
            break;
        }
    }
}
