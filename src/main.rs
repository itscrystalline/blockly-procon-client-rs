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
    let handle = ChaserGame::join("crystal", "Practice1");
    let mut current_dir = Direction::Top;
    handle.send(C2SPacket::MovePlayer(Direction::Left));
    handle.send(C2SPacket::MovePlayer(Direction::Left));
    loop {
        thread::sleep(Duration::from_millis(20));
        if !matches!(handle.info().phase, GamePhase::Ended { .. }) {
            let pos = handle.info().players.us.pos;
            let (x_b, y_b) = handle.info().map_size;
            println!("at pos: {pos:?} w/ size: ({x_b}, {y_b}) ");
            current_dir = match pos {
                (0, 0) => Direction::Right,
                (0, y) if y == y_b - 1 => Direction::Top,
                (x, 0) if x == x_b - 1 => Direction::Bottom,
                (x, y) if x == x_b - 1 && y == y_b - 1 => Direction::Left,
                _ => current_dir,
            };
            println!("moving {current_dir:?}");
            handle.send(C2SPacket::MovePlayer(current_dir));
        }
    }
}
