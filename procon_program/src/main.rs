#![feature(iter_map_windows)]

use chaser::{
    game::{ChaserGame, ChaserHandle},
    game_types::{Direction, Element, Map},
    packets::C2SPacket,
};
use pathfinding::prelude::astar;

const CHARGE: u32 = 40;
const OPP_RANGE: usize = 5;

fn pathfind_astar(handle: ChaserHandle) {
    let mut walls = vec![];
    let mut current_target = None;
    ChaserGame::run_loop(handle, |handle| {
        let us = handle.info().players.us.pos;
        let opp = handle.info().players.opponent.pos;
        let opp_elem = handle.info().players.opponent.side.to_elem();
        let size = handle.info().map_size;
        let turns_left = handle.info().turns_left;
        let map = handle.info().map.clone();

        fn dist(s: (usize, usize), t: (usize, usize)) -> usize {
            s.0.abs_diff(t.0) + s.1.abs_diff(t.1)
        }

        if dist(us, opp) <= OPP_RANGE && !map.deadlocked() {
            _ = current_target.insert(opp);
        }

        if map.deadlocked() {
            println!("deadlocked");
            _ = current_target.insert(loop {
                let x = fastrand::usize(..size.0);
                let y = fastrand::usize(..size.1);
                if map.at(x, y) != Element::Wall {
                    println!("going to {x}, {y}");
                    break (x, y);
                }
            });
        }

        let hearts = viable_hearts(&map, size, map.hearts_near(us));
        let target = if let Some(t) = current_target {
            t
        } else if turns_left < CHARGE || hearts.is_empty() {
            _ = current_target.insert(opp);
            opp
        } else if !hearts.is_empty() {
            _ = current_target.insert(*hearts.first().unwrap());
            *hearts.first().unwrap()
        } else {
            let res = loop {
                let x = fastrand::usize(..size.0);
                let y = fastrand::usize(..size.1);
                if map.at(x, y) != Element::Wall {
                    break (x, y);
                }
            };
            _ = current_target.insert(res);
            res
        };

        let mut directions = {
            let res = astar(
                &us,
                |&(x, y)| {
                    let mut options = map.around_4((x, y), size);
                    options.retain(|(elem, pos, _)| *elem != Element::Wall && !walls.contains(pos));

                    options
                        .into_iter()
                        .map(|(_, pos, _)| (pos, 1))
                        .collect::<Vec<_>>()
                },
                |&(x, y)| target.0.abs_diff(x) + target.1.abs_diff(y),
                |&p| p == target,
            );
            if let Some((astar_path, _)) = res {
                into_directions(astar_path)
            } else {
                vec![]
            }
        };

        if let Some((_, _, dir)) = map
            .around_4(us, size)
            .iter()
            .find(|(elem, _, _)| *elem == opp_elem)
        {
            handle.send(C2SPacket::PutWall(*dir));
            return;
        }
        if let Some(dir) = directions.pop() {
            if turns_left < CHARGE || dist(us, opp) < OPP_RANGE {
                if directions.is_empty() {
                    handle.send(C2SPacket::PutWall(dir));
                } else {
                    handle.send(C2SPacket::MovePlayer(dir));
                }
            } else {
                if directions.is_empty() {
                    walls.push(us);
                }
                handle.send(C2SPacket::MovePlayer(dir));
            }
        } else {
            _ = current_target.take();
        }
    });
}

fn main() {
    let mut args = std::env::args();
    _ = args.next();
    let room = args.next();
    let name = args.next();
    let server = args.next();
    let handle = if let Some(server) = server {
        ChaserGame::join_url(
            server,
            name.unwrap_or("crystal".to_string()),
            room.unwrap_or("Practice1".to_string()),
        )
    } else {
        ChaserGame::join(
            name.unwrap_or("crystal".to_string()),
            room.unwrap_or("Practice1".to_string()),
        )
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

fn viable_hearts(
    map: &Map,
    size: (usize, usize),
    mut hearts: Vec<(usize, usize)>,
) -> Vec<(usize, usize)> {
    hearts.retain(|pos| {
        let around = map.around_4(*pos, size);
        let self_gaps = around.iter().fold(0, |acc, (elem, _, _)| {
            if *elem == Element::Blank || *elem == Element::Heart {
                acc + 1
            } else {
                acc
            }
        }) > 1;
        let around_gaps = around
            .iter()
            .flat_map(|(_, pos, _)| map.around_4(*pos, size))
            .fold(0, |acc, (elem, _, _)| {
                if elem == Element::Blank || elem == Element::Heart {
                    acc + 1
                } else {
                    acc
                }
            })
            > 1;
        self_gaps && around_gaps
    });
    hearts
}
