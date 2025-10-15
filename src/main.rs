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
    let mut state = State::Current;
    // handle.send(C2SPacket::MovePlayer(Direction::Left));
    // handle.send(C2SPacket::MovePlayer(Direction::Left));

    enum State {
        Current,
        Turn,
    }

    ChaserGame::run_loop(handle, |handle| {
        let y = handle.info().players.us.pos.1;
        let y_b = handle.info().map_size.1;

        match &state {
            State::Current => {
                if y == 0 {
                    current_dir = current_dir.right();
                } else if y == y_b - 1 {
                    current_dir = current_dir.left();
                }
                state = State::Turn;
            }
            State::Turn => {
                if y == 0 {
                    current_dir = current_dir.right();
                } else if y == y_b - 1 {
                    current_dir = current_dir.left();
                }
                state = State::Current;
            }
        }

        handle.send(C2SPacket::MovePlayer(current_dir));
    });
}
