mod client;
mod game;
mod game_types;
mod packets;
mod ui;

use std::{thread, time::Duration};

use crate::{
    game::{ChaserGame, GamePhase},
    game_types::Direction,
    packets::C2SPacket,
};

fn main() {
    let handle = ChaserGame::join("crystal", "Practice2");
    loop {
        if !matches!(handle.info().phase, GamePhase::Ended { .. }) {
            handle.send(C2SPacket::MovePlayer(Direction::Top));
        }
        thread::sleep(Duration::from_millis(200));
    }
}
