#![feature(iter_map_windows)]

use chaser::{
    game::{ChaserGame, ChaserHandle},
    game_types::{Direction, Element},
    packets::C2SPacket,
};
use pathfinding::prelude::astar;

fn pathfind_astar(handle: ChaserHandle) {
    let mut walls = vec![];
    let mut current_target = None;
    ChaserGame::run_loop(handle, |handle| {
        let us = handle.info().players.us.pos;
        let opp = handle.info().players.opponent.pos;
        let opp_side = handle.info().players.opponent.side;
        let size = handle.info().map_size;
        let turns_left = handle.info().turns_left;
        let map = handle.info().map.clone();

        fn dist(s: (usize, usize), t: (usize, usize)) -> usize {
            s.0.abs_diff(t.0) + s.1.abs_diff(t.1)
        }

        if dist(us, opp) <= 5 && !map.deadlocked() {
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

        let target = if let Some(t) = current_target {
            t
        } else if turns_left < 80 {
            _ = current_target.insert(opp);
            opp
        } else if let hearts = map.hearts_near(us)
            && !hearts.is_empty()
        {
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
                            if walls.contains(&pos) {
                                continue;
                            }
                            if !matches!(map.at(pos.0, pos.1), Element::Wall) {
                                options.push(((n_x as usize, n_y as usize), 1));
                            }
                        }
                    }
                    options
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

        let mut surrounding = vec![];
        for x in [(-1, Direction::Left), (1, Direction::Right)] {
            if let Some(r_x) = us.0.checked_add_signed(x.0) {
                surrounding.push((map.at(r_x, us.1), x.1));
            }
        }
        for y in [(-1, Direction::Top), (1, Direction::Bottom)] {
            if let Some(r_y) = us.1.checked_add_signed(y.0) {
                surrounding.push((map.at(us.0, r_y), y.1));
            }
        }
        for (elem, dir) in surrounding {
            if let Element::Hot | Element::Cold = elem {
                let elem_side = elem.to_side();
                if elem_side == opp_side {
                    handle.send(C2SPacket::PutWall(dir));
                    return;
                }
            }
        }

        if let Some(dir) = directions.pop() {
            if turns_left < 80 || dist(us, opp) < 5 {
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
