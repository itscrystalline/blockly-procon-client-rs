#![feature(iter_map_windows)]

use std::{cmp::min, ops::RangeInclusive};

use chaser::{
    game::{ChaserGame, ChaserHandle},
    game_types::{Direction, Element, Map},
    packets::C2SPacket,
};
use pathfinding::prelude::astar;

const CHARGE: u32 = 40;
const OPP_RANGE: usize = 5;

type Point = (usize, usize);

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

enum TargetState {
    Searching,
    Wandering(Point),
    FixDeadlock(Point),
    Heart(Point),
    Opponent(Point),
}

fn dist(s: Point, t: Point) -> usize {
    s.0.abs_diff(t.0) + s.1.abs_diff(t.1)
}
fn bounds_ranges(
    around: Point,
    size: Point,
    radius: usize,
) -> (RangeInclusive<usize>, RangeInclusive<usize>) {
    (
        around.0.saturating_sub(radius)..=min(size.0 - 1, around.0 + radius),
        around.1.saturating_sub(radius)..=min(size.1 - 1, around.1 + radius),
    )
}
fn pathfind_astar(handle: ChaserHandle) {
    let mut walls = vec![];
    let mut state = TargetState::Searching;
    ChaserGame::run_loop(handle, |handle| {
        let (us, opp, opp_elem, size, turns_left, map) = {
            let i = handle.info();
            (
                i.players.us.pos,
                i.players.opponent.pos,
                i.players.opponent.side.to_elem(),
                i.map_size,
                i.turns_left,
                i.map.clone(),
            )
        };

        if map.deadlocked() {
            println!("deadlocked");
            state = TargetState::FixDeadlock(loop {
                let around = bounds_ranges(us, size, 2);
                let x = fastrand::usize(around.0);
                let y = fastrand::usize(around.1);
                if map.at(x, y) != Element::Wall && (x, y) != us {
                    println!("going to {x}, {y}");
                    break (x, y);
                }
            });
        }
        if let Some((_, _, dir)) = map
            .around_4(us, size)
            .iter()
            .find(|(elem, _, _)| *elem == opp_elem)
        {
            handle.send(C2SPacket::PutWall(*dir));
            return;
        }

        let hearts = viable_hearts(&map, size, map.hearts_near(us));
        match state {
            TargetState::Searching => {
                if turns_left < CHARGE || hearts.is_empty() || dist(us, opp) < OPP_RANGE {
                    println!("running to opp");
                    state = TargetState::Opponent(opp);
                } else if !hearts.is_empty() {
                    println!("running to heart");
                    state = TargetState::Heart(*hearts.first().unwrap());
                } else {
                    let res = loop {
                        let x = fastrand::usize(..size.0);
                        let y = fastrand::usize(..size.1);
                        if map.at(x, y) != Element::Wall && (x, y) != us {
                            println!("going to {x}, {y}");
                            break (x, y);
                        }
                    };
                    state = TargetState::Wandering(res);
                }
            }
            TargetState::Wandering(pos) | TargetState::Heart(pos) | TargetState::Opponent(pos) => {
                if us == pos {
                    state = TargetState::Searching
                }
                if turns_left < CHARGE || dist(us, opp) < OPP_RANGE {
                    println!("running to opp");
                    state = TargetState::Opponent(opp);
                }
            }
            TargetState::FixDeadlock(pos) => {
                if us == pos {
                    state = TargetState::Searching
                }
            }
        }

        if let TargetState::Wandering(target)
        | TargetState::Heart(target)
        | TargetState::Opponent(target)
        | TargetState::FixDeadlock(target) = state
        {
            let mut directions = run_astar(&map, us, target, size, &walls);

            if let Some(dir) = directions.pop() {
                if matches!(state, TargetState::Opponent(_)) {
                    if directions.is_empty() {
                        handle.send(C2SPacket::PutWall(dir));
                    } else {
                        handle.send(C2SPacket::MovePlayer(dir));
                    }
                } else {
                    if directions.is_empty() && matches!(state, TargetState::Heart(_)) {
                        walls.push(us);
                    }
                    handle.send(C2SPacket::MovePlayer(dir));
                }
            } else {
                state = TargetState::Searching;
            }
        }
    });
}

fn run_astar(
    map: &Map,
    src: Point,
    dest: Point,
    size: Point,
    blacklisted: &[Point],
) -> Vec<Direction> {
    let res = astar(
        &src,
        |&(x, y)| {
            let mut options = map.around_4((x, y), size);
            options.retain(|(elem, pos, _)| *elem != Element::Wall && !blacklisted.contains(pos));

            options
                .into_iter()
                .map(|(_, pos, _)| (pos, 1))
                .collect::<Vec<_>>()
        },
        |&(x, y)| dest.0.abs_diff(x) + dest.1.abs_diff(y),
        |&p| p == dest,
    );
    if let Some((astar_path, _)) = res {
        into_directions(astar_path)
    } else {
        vec![]
    }
}

fn into_directions(path: Vec<Point>) -> Vec<Direction> {
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

fn viable_hearts(map: &Map, size: Point, mut hearts: Vec<Point>) -> Vec<Point> {
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
