#![feature(iter_map_windows)]

use chaser::{
    game::ChaserGame,
    game_types::{Direction, Element},
    packets::C2SPacket,
};
use pathfinding::prelude::astar;

fn main() {
    let handle = ChaserGame::join("crystal", "game_server_14");
    // handle.send(C2SPacket::MovePlayer(Direction::Left));
    // handle.send(C2SPacket::MovePlayer(Direction::Left));

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

// fn astar(map: &Map, size: (usize, usize), src: Point, dest: Point) -> Vec<Direction> {
//     println!("running!");
//     if src == dest {
//         return vec![];
//     }
//
//     #[derive(Debug, Clone, Copy)]
//     struct Cell {
//         parent: Option<(usize, usize)>,
//         f: usize,
//         g: usize,
//         h: usize,
//     }
//
//     fn blocked_at(map: &Map, p: Point) -> bool {
//         matches!(map.at(p.0, p.1), Element::Wall)
//     }
//
//     fn heuristic(a: Point, b: Point) -> usize {
//         usize::abs_diff(a.0, b.0) + usize::abs_diff(a.1, b.1)
//     }
//
//     fn reconstruct_path(cells: &Vec<Vec<Cell>>, mut cur: Point) -> Vec<Direction> {
//         let mut pts = Vec::new();
//         // trace back until a cell points to itself (the source)
//         while cells[cur.1][cur.0].parent != Some(cur) {
//             pts.push(cur);
//             cur = cells[cur.1][cur.0].parent.unwrap();
//         }
//         pts.push(cur);
//         pts.reverse();
//
//         let mut dirs = Vec::new();
//         for w in pts.windows(2) {
//             let a = w[0];
//             let b = w[1];
//             if b.0 == a.0 + 1 && b.1 == a.1 {
//                 dirs.push(Direction::Right);
//             } else if b.0 + 1 == a.0 && b.1 == a.1 {
//                 dirs.push(Direction::Left);
//             } else if b.1 == a.1 + 1 && b.0 == a.0 {
//                 dirs.push(Direction::Bottom);
//             } else if b.1 + 1 == a.1 && b.0 == a.0 {
//                 dirs.push(Direction::Top);
//             } else {
//                 panic!("diagonal or non-adjacent step in path");
//             }
//         }
//         dirs
//     }
//
//     let (width, height) = size;
//
//     let mut closed = vec![vec![false; width]; height];
//     let mut cells = vec![
//         vec![
//             Cell {
//                 parent: None,
//                 f: usize::MAX,
//                 g: usize::MAX,
//                 h: usize::MAX,
//             };
//             width
//         ];
//         height
//     ];
//
//     let (sx, sy) = src;
//     cells[sy][sx].g = 0;
//     cells[sy][sx].h = 0;
//     cells[sy][sx].f = 0;
//     cells[sy][sx].parent = Some(src);
//
//     // min-heap by f: store (Reverse(f), x, y)
//     let mut heap: BinaryHeap<(Reverse<usize>, usize, usize)> = BinaryHeap::new();
//     heap.push((Reverse(0), sx, sy));
//
//     while let Some((Reverse(cur_f), x, y)) = heap.pop() {
//         // stale entry guard: skip if this doesn't match cell's current f
//         if closed[y][x] {
//             continue;
//         }
//         if cur_f != cells[y][x].f {
//             // outdated entry
//             continue;
//         }
//
//         // mark current closed
//         closed[y][x] = true;
//
//         // try 4 neighbours
//         let neighbors = [
//             (x.wrapping_sub(1), y), // left (guard via checked below)
//             (x + 1, y),             // right
//             (x, y.wrapping_sub(1)), // up
//             (x, y + 1),             // down
//         ];
//
//         for &(nx, ny) in &neighbors {
//             // bounds check using checked arithmetic
//             if nx >= width || ny >= height {
//                 continue;
//             }
//             if (nx, ny) == dest {
//                 // set parent so reconstruct_path works
//                 cells[ny][nx].parent = Some((x, y));
//                 return reconstruct_path(&cells, dest);
//             }
//
//             if closed[ny][nx] || blocked_at(map, (nx, ny)) {
//                 continue;
//             }
//
//             // compute tentative scores
//             let g_new = cells[y][x].g.saturating_add(1); // cells[y][x].g is valid here
//             let h_new = heuristic((nx, ny), dest);
//             let f_new = g_new.saturating_add(h_new);
//
//             // if we found a better path, update cell and push to heap
//             if f_new < cells[ny][nx].f {
//                 cells[ny][nx].f = f_new;
//                 cells[ny][nx].g = g_new;
//                 cells[ny][nx].h = h_new;
//                 cells[ny][nx].parent = Some((x, y));
//                 heap.push((Reverse(f_new), nx, ny));
//             }
//         }
//     }
//
//     // no path
//     vec![]
// }
