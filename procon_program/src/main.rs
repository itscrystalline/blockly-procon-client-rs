#![feature(iter_map_windows)]

use chaser::{
    game::{ChaserGame, ChaserHandle},
    game_types::{Direction, Element},
    packets::C2SPacket,
};
use pathfinding::prelude::astar;

fn pathfind_astar(handle: ChaserHandle) {
    ChaserGame::run_loop(handle, |handle| {
        let us = handle.info().players.us.pos;
        let opp = handle.info().players.opponent.pos;
        let size = handle.info().map_size;
        let map = handle.info().map.clone();
        let directions = {
            let res = astar(
                &us,
                |&(x, y)| {
                    let mut options = vec![];
                    let (x_i, y_i) = (x as isize, y as isize);
                    for (n_x, n_y) in [
                        (x_i - 1, y_i),
                        (x_i + 1, y_i),
                        (x_i, y_i - 1),
                        (x_i, y_i + 1),
                    ] {
                        if (0..size.0 as isize).contains(&n_x)
                            && (0..size.1 as isize).contains(&n_y)
                        {
                            let pos = (n_x as usize, n_y as usize);
                            if !matches!(map.at(pos.0, pos.1), Element::Wall) {
                                options.push(((n_x as usize, n_y as usize), 1));
                            }
                        }
                    }
                    options
                },
                |&(x, y)| opp.0.abs_diff(x) + opp.1.abs_diff(y),
                |&p| p == opp,
            );
            if let Some((astar_path, _)) = res {
                let directions = into_directions(astar_path);
                Some(directions)
            } else {
                None
            }
        };

        if let Some(mut dir_v) = directions
            && let Some(dir) = dir_v.pop()
        {
            if dir_v.is_empty() {
                handle.send(C2SPacket::PutWall(dir));
            } else {
                handle.send(C2SPacket::MovePlayer(dir));
            }
        }
    });
}

fn main() {
    let mut args = std::env::args();
    _ = args.next();
    let room = args.next();
    let server = args.next();
    let handle = if let Some(server) = server {
        ChaserGame::join_url(server, "crystal", room.unwrap_or("Practice1".to_string()))
    } else {
        ChaserGame::join("crystal", room.unwrap_or("Practice1".to_string()))
    };
    // handle.send(C2SPacket::MovePlayer(Direction::Left));
    // handle.send(C2SPacket::MovePlayer(Direction::Left));
    //
    pathfind_astar(handle);
    // search_test(handle);
}

fn into_directions(path: Vec<(usize, usize)>) -> Vec<Direction> {
    let mut res = path
        .iter()
        .map_windows(|&[&(a_x, a_y), &b]| {
            if a_x > 0 && b == (a_x - 1, a_y) {
                Direction::Left
            } else if a_y > 0 && b == (a_x, a_y - 1) {
                Direction::Top
            } else if b == (a_x + 1, a_y) {
                Direction::Right
            } else if b == (a_x, a_y + 1) {
                Direction::Bottom
            } else {
                panic!("shouldn't move diagonally!")
            }
        })
        .collect::<Vec<_>>();
    res.reverse();
    res
}
