#![feature(iter_map_windows)]

use std::{cmp::min, ops::RangeInclusive};

use argh::FromArgs;
use chaser::{
    game::{ChaserGame, ChaserHandle},
    game_types::{Direction, Element, Map},
    packets::C2SPacket,
};
use pathfinding::prelude::astar;

const CHARGE: u32 = 50;
const OPP_RANGE: usize = 5;

type Point = (usize, usize);

#[derive(FromArgs)]
/// Options for the client.
struct Options {
    /// room name
    #[argh(option)]
    room: Option<String>,
    /// player name
    #[argh(option)]
    name: Option<String>,
    /// server url
    #[argh(option)]
    server: Option<String>,
}

fn main() {
    let Options { room, name, server } = argh::from_env();

    let handle = if let Some(server) = server {
        ChaserGame::join_url(
            server,
            name.unwrap_or("crystal".to_string()),
            room.unwrap_or("Tornament2".to_string()),
        )
    } else {
        ChaserGame::join(
            name.unwrap_or("crystal".to_string()),
            room.unwrap_or("Tornament2".to_string()),
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
    let mut stuck_counter = 0;
    let mut skip_counter = 0;
    ChaserGame::run_loop(true, handle, |handle| {
        let (us, opp, opp_elem, size, mut turns_left, map) = {
            let i = handle.info();
            #[cfg(feature = "fow")]
            {
                (
                    i.players.us.pos,
                    i.players.opponent.pos,
                    i.players.opponent.side.to_elem(),
                    i.map_size,
                    i.turns_left,
                    i.map.clone(),
                )
            }
            #[cfg(not(feature = "fow"))]
            {
                (
                    i.players.us.pos,
                    Some(i.players.opponent.pos),
                    i.players.opponent.side.to_elem(),
                    i.map_size,
                    i.turns_left,
                    i.map.clone(),
                )
            }
        };

        fn go_for_opp(turns_left: u32, us: Point, opp: Point) -> bool {
            turns_left < CHARGE || dist(us, opp) < OPP_RANGE
        }

        if map.deadlocked() || opp.is_some_and(|opp| opp == us) || stuck_counter > 5 {
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
            stuck_counter = 0;
        }
        if let Some((_, pos, dir)) = map
            .around_4(us, size)
            .iter()
            .find(|(elem, _, _)| *elem == opp_elem)
        {
            println!("placing block on opp at {pos:?} ({dir:?})");
            handle.send(C2SPacket::PutWall(*dir));
            return;
        }

        let scan_chance = if turns_left < CHARGE { 75 } else { 50 };
        if fastrand::usize(0..100) < scan_chance {
            random_scan(handle, size, us);
            turns_left -= 1;
        }

        let hearts = viable_hearts(&map, size, map.hearts_near(us));
        match state {
            TargetState::Searching => {
                if let Some(opp) = opp
                    && (hearts.is_empty() || go_for_opp(turns_left, us, opp))
                {
                    println!("running to opp {opp:?}");
                    state = TargetState::Opponent(opp);
                } else if !hearts.is_empty()
                    && (cfg!(feature = "fow") || fastrand::usize(0..10) > 3)
                {
                    let heart = *hearts.first().unwrap();
                    state = TargetState::Heart(heart);
                    println!("running to heart {heart:?}");
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
                    println!("reached destination");
                    state = TargetState::Searching
                }
                if let Some(opp) = opp
                    && go_for_opp(turns_left, us, opp)
                {
                    println!("running to opp");
                    state = TargetState::Opponent(opp);
                }
            }
            TargetState::FixDeadlock(pos) => {
                if us == pos {
                    println!("reached destination");
                    state = TargetState::Searching
                }
            }
        }

        if let TargetState::Wandering(target)
        | TargetState::Heart(target)
        | TargetState::Opponent(target)
        | TargetState::FixDeadlock(target) = state
        {
            let mut directions =
                if matches!(state, TargetState::Heart(_) | TargetState::Wandering(_)) {
                    run_astar(&map, us, target, size, &walls, |pos| {
                        if let Some(opp) = opp {
                            (size.0 + size.1) - dist(pos, opp)
                        } else {
                            1
                        }
                    })
                } else {
                    run_astar(&map, us, target, size, &walls, |_| 1)
                };
            // println!("{directions:?}");

            if let Some(dir) = directions.pop() {
                if matches!(state, TargetState::Opponent(_)) {
                    if directions.is_empty() {
                        handle.send(C2SPacket::PutWall(dir));
                    } else if directions.len() == 1 && skip_counter < 3 {
                        println!("skipping");
                        skip_counter += 1;
                        handle.send(C2SPacket::Search(Direction::Top));
                    } else {
                        handle.send(C2SPacket::MovePlayer(dir));
                        skip_counter = 0;
                    }
                } else {
                    if directions.is_empty() && matches!(state, TargetState::Heart(_)) {
                        walls.push(us);
                    }
                    handle.send(C2SPacket::MovePlayer(dir));
                }
            } else {
                println!("reached or cannot go, searching");
                state = TargetState::Searching;
                stuck_counter += 1;
            }
        }
    });
}

fn random_scan(handle: &ChaserHandle, size: Point, pos: Point) {
    let (half_x, half_y) = (size.0 / 2, size.1 / 2);
    let area_or_line = fastrand::bool();
    let dir1_or_dir2 = fastrand::bool();
    // let (odd_x, odd_y) = (size.0 % 2 == 0, size.1 % 2 == 0);
    if (0..half_x).contains(&pos.0) {
        if (0..half_y).contains(&pos.1) {
            // top left
            let dir = if dir1_or_dir2 {
                Direction::Bottom
            } else {
                Direction::Right
            };
            handle.send(if area_or_line {
                C2SPacket::Look(dir)
            } else {
                C2SPacket::Search(dir)
            });
        } else {
            // top right
            let dir = if dir1_or_dir2 {
                Direction::Bottom
            } else {
                Direction::Left
            };
            handle.send(if area_or_line {
                C2SPacket::Look(dir)
            } else {
                C2SPacket::Search(dir)
            });
        }
    } else if (0..half_y).contains(&pos.1) {
        // bottom left
        let dir = if dir1_or_dir2 {
            Direction::Top
        } else {
            Direction::Right
        };
        handle.send(if area_or_line {
            C2SPacket::Look(dir)
        } else {
            C2SPacket::Search(dir)
        });
    } else {
        // bottom right
        let dir = if dir1_or_dir2 {
            Direction::Top
        } else {
            Direction::Left
        };
        handle.send(if area_or_line {
            C2SPacket::Look(dir)
        } else {
            C2SPacket::Search(dir)
        });
    }
}

fn run_astar(
    map: &Map,
    src: Point,
    dest: Point,
    size: Point,
    blacklisted: &[Point],
    cost_fn: impl Fn(Point) -> usize,
) -> Vec<Direction> {
    let res = astar(
        &src,
        |&(x, y)| {
            let mut options = map.around_4((x, y), size);
            options.retain(|(elem, pos, _)| *elem != Element::Wall && !blacklisted.contains(pos));

            options
                .into_iter()
                .map(|(_, pos, _)| (pos, cost_fn(pos)))
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
