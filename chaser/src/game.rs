use parking_lot::{Mutex, MutexGuard};
use std::{cmp::min, ffi::OsStr, sync::Arc, thread, time::Duration};

use crate::{
    client::{Client, SocketIo},
    game_types::{Direction, Effect, Element, GameData, Map, RecElement, Side},
    packets::{C2SPacket, S2CPacket},
    ui,
};
#[derive(Debug, Clone)]
pub enum GamePhase {
    Starting,
    Turn(Side),
    Ended { winner: Side, reason: String },
}
pub struct GameState {
    pub room: String,
    pub phase: GamePhase,
    pub map: Map,
    pub map_size: (usize, usize),
    pub effect: Option<Effect>,
    pub turns_left: u32,
    pub players: Players,
}
pub struct Players {
    #[cfg(feature = "fog_of_war")]
    pub us: OwnPlayer,
    #[cfg(not(feature = "fog_of_war"))]
    pub us: Player,
    pub opponent: Player,
}
impl Players {
    fn assign_scores(&mut self, cool: u32, hot: u32) {
        if let Side::Cold = self.us.side {
            self.us.score = cool;
            self.opponent.score = hot;
        } else {
            self.us.score = hot;
            self.opponent.score = cool;
        }
    }
}
pub struct Player {
    pub name: String,
    #[cfg(feature = "fog_of_war")]
    pub pos: Option<(usize, usize)>,
    #[cfg(not(feature = "fog_of_war"))]
    pub pos: (usize, usize),
    pub score: u32,
    pub side: Side,
}
#[cfg(feature = "fog_of_war")]
pub struct OwnPlayer {
    pub name: String,
    pub pos: (usize, usize),
    pub score: u32,
    pub side: Side,
}
#[cfg(feature = "fog_of_war")]
impl From<OwnPlayer> for Player {
    fn from(v: OwnPlayer) -> Self {
        Self {
            name: v.name,
            pos: Some(v.pos),
            score: v.score,
            side: v.side,
        }
    }
}

pub struct ChaserGame {
    client: Client,
    state: Arc<Mutex<GameState>>,
}
pub struct ChaserHandle {
    state: Arc<Mutex<GameState>>,
    send: Arc<Mutex<Option<C2SPacket>>>,
}
impl ChaserGame {
    pub fn join(name: impl ToString, map: impl ToString) -> ChaserHandle {
        Self::join_url("http://localhost:3000", name, map)
    }
    pub fn join_url(
        url: impl AsRef<OsStr>,
        name: impl ToString,
        map: impl ToString,
    ) -> ChaserHandle {
        let name = name.to_string();
        let map = map.to_string();
        let socketio = if url
            .as_ref()
            .to_str()
            .is_some_and(|url| url.starts_with("https://blockly.kbylabs.com"))
        {
            SocketIo::Four
        } else {
            SocketIo::Two
        };
        let mut client = Client::with_server(&url, socketio);

        client.send(C2SPacket::PlayerJoin {
            room_id: map.clone(),
            name: name.clone(),
        });

        let (x_size, y_size, cool_name, hot_name) = loop {
            if let Some(S2CPacket::JoinedRoom {
                x_size,
                y_size,
                cool_name,
                hot_name,
            }) = client.recv()
                && !(hot_name.contains("接続待機中") || cool_name.contains("接続待機中"))
            {
                break (x_size, y_size, cool_name, hot_name);
            }
        };

        let S2CPacket::NewBoard(GameData {
            map_data,
            cool_score,
            hot_score,
            turn,
            ..
        }) = (loop {
            if let Some(p) = client.recv() {
                break p;
            }
        })
        else {
            panic!("unexpected packet received while waiting for game data")
        };

        let cool_pos = map_data.find_player(Side::Cold);
        let hot_pos = map_data.find_player(Side::Hot);
        let players = if cool_name == name {
            Players {
                #[cfg(feature = "fog_of_war")]
                us: OwnPlayer {
                    name: cool_name,
                    pos: cool_pos.expect("!cool_pos"),
                    score: cool_score,
                    side: Side::Cold,
                },
                #[cfg(not(feature = "fog_of_war"))]
                us: Player {
                    name: cool_name,
                    pos: cool_pos.expect("!cool_pos"),
                    score: cool_score,
                    side: Side::Cold,
                },
                opponent: Player {
                    name: hot_name,
                    #[cfg(feature = "fog_of_war")]
                    pos: None,
                    #[cfg(not(feature = "fog_of_war"))]
                    pos: hot_pos.expect("!hot_pos"),
                    score: hot_score,
                    side: Side::Hot,
                },
            }
        } else if hot_name == name {
            Players {
                #[cfg(feature = "fog_of_war")]
                us: OwnPlayer {
                    name: hot_name,
                    pos: hot_pos.expect("!hot_pos"),
                    score: hot_score,
                    side: Side::Hot,
                },
                #[cfg(not(feature = "fog_of_war"))]
                us: Player {
                    name: hot_name,
                    pos: hot_pos.expect("!hot_pos"),
                    score: hot_score,
                    side: Side::Hot,
                },
                opponent: Player {
                    name: cool_name,
                    #[cfg(feature = "fog_of_war")]
                    pos: None,
                    #[cfg(not(feature = "fog_of_war"))]
                    pos: cool_pos.expect("!cool_pos"),
                    score: cool_score,
                    side: Side::Cold,
                },
            }
        } else {
            unreachable!()
        };

        let state = Arc::new(Mutex::new(GameState {
            room: map.clone(),
            #[cfg(feature = "fog_of_war")]
            map: Map::empty((x_size, y_size)),
            #[cfg(not(feature = "fog_of_war"))]
            map: map_data.clone(),
            map_size: (x_size, y_size),
            turns_left: turn,
            phase: GamePhase::Starting,
            effect: None,
            players,
        }));
        let state2 = Arc::clone(&state);
        let state3 = Arc::clone(&state);

        let game = ChaserGame { client, state };

        let c2s_arc1 = Arc::new(Mutex::new(None));
        let c2s_arc2 = Arc::clone(&c2s_arc1);

        ui::start_ui(state3);

        thread::spawn(move || {
            let mut game = game;
            game.client.send(C2SPacket::GetReady);
            let mut ready = false;
            let mut ended: Option<Side> = None;
            let our_side = game.state.lock().players.us.side;
            let mut last_search: Option<Direction> = None;
            loop {
                thread::sleep(Duration::from_millis(10));
                if ended.is_some() {
                    break;
                }
                if let Some(p) = game.client.recv() {
                    match p {
                        S2CPacket::GameResult { winner, info } => {
                            game.state.lock().phase = GamePhase::Ended {
                                winner,
                                reason: info,
                            }
                        }
                        S2CPacket::UpdateBoard(GameData {
                            map_data,
                            cool_score,
                            hot_score,
                            turn,
                            effect,
                        }) => {
                            let mut state = game.state.lock();

                            if let Some(Effect { player, .. }) = effect {
                                state.phase = GamePhase::Turn(player);

                                if player != state.players.us.side {
                                    game.client.send(C2SPacket::GetReady);
                                    ready = false;
                                }
                            }

                            #[cfg(not(feature = "fog_of_war"))]
                            {
                                let new_us = map_data.find_player(state.players.us.side);
                                let new_opp = map_data.find_player(state.players.opponent.side);
                                state.map = map_data;
                                if let Some(new_us) = new_us {
                                    state.players.us.pos = new_us;
                                }
                                if let Some(new_opp) = new_opp {
                                    state.players.opponent.pos = new_opp;
                                }
                            }
                            #[cfg(feature = "fog_of_war")]
                            {
                                let us = state.players.us.pos;
                                let us_side = state.players.us.side;
                                let new_us = map_data
                                    .find_player(state.players.us.side)
                                    .expect("player moved >2 blocks or was killed");
                                state.players.us.pos = new_us;
                                state.map.set(us.0, us.1, Element::Blank);
                                state.map.set(new_us.0, new_us.1, us_side.to_elem());
                            }
                            state.turns_left = turn;
                            state.effect = effect;
                            state.players.assign_scores(cool_score, hot_score);
                        }
                        S2CPacket::GetReadyRec { rec_data } => {
                            let mut state = game.state.lock();
                            let pos = state.players.us.pos;
                            let side = state.players.us.side;

                            #[cfg(feature = "fog_of_war")]
                            let mut opp = None;
                            for (i, elem) in rec_data.into_iter().enumerate() {
                                let x_offset = (i % 3) as isize - 1;
                                let y_offset = (i / 3) as isize - 1;
                                if let Some(x) = pos.0.checked_add_signed(x_offset)
                                    && let Some(y) = pos.1.checked_add_signed(y_offset)
                                {
                                    if x_offset == 0 && y_offset == 0 {
                                        _ = state.map.set(
                                            x,
                                            y,
                                            if matches!(elem, RecElement::Opponent) {
                                                Element::BothColdAndHot
                                            } else {
                                                side.to_elem()
                                            },
                                        );
                                    } else {
                                        _ = state.map.set(x, y, elem.into_elem(side));
                                        #[cfg(feature = "fog_of_war")]
                                        if matches!(elem, RecElement::Opponent) {
                                            _ = opp.insert((x, y));
                                        }
                                    }
                                }
                            }

                            #[cfg(feature = "fog_of_war")]
                            if let Some(opp) = opp {
                                if let Some((old_x, old_y)) =
                                    state.players.opponent.pos.replace(opp)
                                {
                                    state.map.set(old_x, old_y, Element::Blank);
                                }
                            } else if let Some(old) = state.players.opponent.pos
                                && state
                                    .map
                                    .around_8(pos, state.map_size)
                                    .iter()
                                    .any(|(_, pos)| *pos == old)
                            {
                                let (old_x, old_y) = state.players.opponent.pos.take().unwrap();
                                state.map.set(old_x, old_y, Element::Blank);
                            }

                            ready = true;
                        }
                        S2CPacket::MoveRec { rec_data }
                        | S2CPacket::PutRec { rec_data }
                        | S2CPacket::LookRec { rec_data } => {
                            let mut state = game.state.lock();
                            let pos = state.players.us.pos;
                            let side = state.players.us.side;

                            let offset = match last_search {
                                None => (0, 0),
                                Some(Direction::Top) => (0, -2),
                                Some(Direction::Bottom) => (0, 2),
                                Some(Direction::Left) => (-2, 0),
                                Some(Direction::Right) => (2, 0),
                            };

                            #[cfg(feature = "fog_of_war")]
                            let mut opp = None;
                            for (i, elem) in rec_data.into_iter().enumerate() {
                                let x_offset = (i % 3) as isize - 1 + offset.0;
                                let y_offset = (i / 3) as isize - 1 + offset.1;
                                if let Some(x) = pos.0.checked_add_signed(x_offset)
                                    && let Some(y) = pos.1.checked_add_signed(y_offset)
                                {
                                    if x_offset == 0 && y_offset == 0 {
                                        _ = state.map.set(
                                            x,
                                            y,
                                            if matches!(elem, RecElement::Opponent) {
                                                Element::BothColdAndHot
                                            } else {
                                                side.to_elem()
                                            },
                                        );
                                    } else {
                                        _ = state.map.set(x, y, elem.into_elem(side));
                                        #[cfg(feature = "fog_of_war")]
                                        if matches!(elem, RecElement::Opponent) {
                                            _ = opp.insert((x, y));
                                        }
                                    }
                                }
                            }

                            #[cfg(feature = "fog_of_war")]
                            if let Some(opp) = opp {
                                if let Some((old_x, old_y)) =
                                    state.players.opponent.pos.replace(opp)
                                {
                                    state.map.set(old_x, old_y, Element::Blank);
                                }
                            } else if let Some(old) = state.players.opponent.pos
                                && state
                                    .map
                                    .around_8(pos, state.map_size)
                                    .iter()
                                    .any(|(_, pos)| *pos == old)
                            {
                                let (old_x, old_y) = state.players.opponent.pos.take().unwrap();
                                state.map.set(old_x, old_y, Element::Blank);
                            }

                            _ = last_search.take();
                        }
                        S2CPacket::SearchRec { rec_data } => {
                            if let Some(dir) = last_search {
                                let mut state = game.state.lock();
                                let pos = state.players.us.pos;
                                let side = state.players.us.side;
                                let map_size = state.map_size;

                                let range: Vec<usize> = match dir {
                                    Direction::Top => (pos.1.saturating_sub(9)
                                        ..=pos.1.saturating_sub(1))
                                        .rev()
                                        .collect(),
                                    Direction::Bottom => (min(pos.1 + 1, map_size.1)
                                        ..=min(pos.1 + 9, map_size.1))
                                        .collect(),
                                    Direction::Left => (pos.0.saturating_sub(9)
                                        ..=pos.0.saturating_sub(1))
                                        .rev()
                                        .collect(),
                                    Direction::Right => (min(pos.0 + 1, map_size.0)
                                        ..=min(pos.0 + 9, map_size.0))
                                        .collect(),
                                };
                                let other_pos = match dir {
                                    Direction::Top | Direction::Bottom => pos.0,
                                    Direction::Left | Direction::Right => pos.1,
                                };

                                #[cfg(feature = "fog_of_war")]
                                let mut opp = None;
                                for (elem, pos) in rec_data.into_iter().zip(range) {
                                    let x = if matches!(dir, Direction::Top | Direction::Bottom) {
                                        other_pos
                                    } else {
                                        pos
                                    };
                                    let y = if matches!(dir, Direction::Left | Direction::Right) {
                                        other_pos
                                    } else {
                                        pos
                                    };
                                    _ = state.map.set(x, y, elem.into_elem(side));
                                    #[cfg(feature = "fog_of_war")]
                                    if matches!(elem, RecElement::Opponent) {
                                        _ = opp.insert((x, y));
                                    }
                                }

                                #[cfg(feature = "fog_of_war")]
                                if let Some(opp) = opp {
                                    if let Some((old_x, old_y)) =
                                        state.players.opponent.pos.replace(opp)
                                    {
                                        state.map.set(old_x, old_y, Element::Blank);
                                    }
                                } else if let Some(old) = state.players.opponent.pos
                                    && state
                                        .map
                                        .around_8(pos, map_size)
                                        .iter()
                                        .any(|(_, pos)| *pos == old)
                                {
                                    let (old_x, old_y) = state.players.opponent.pos.take().unwrap();
                                    state.map.set(old_x, old_y, Element::Blank);
                                }
                            }

                            _ = last_search.take();
                        }
                        _ => (),
                    }
                }
                // send any pending packet
                if let GamePhase::Ended { winner, reason } = &game.state.lock().phase {
                    println!("game over! {winner:?} won! ({reason})");
                    println!("we are {our_side:?}");
                    ended = Some(*winner);
                } else {
                    match c2s_arc1.lock().take() {
                        Some(p) if ready => {
                            if let C2SPacket::MovePlayer(dir) = p {
                                let old_pos = game.state.lock().players.us.pos;
                                game.state.lock().players.us.pos = match dir {
                                    Direction::Top => (old_pos.0, old_pos.1.saturating_sub(1)),
                                    Direction::Bottom => (old_pos.0, old_pos.1 + 1),
                                    Direction::Left => (old_pos.0.saturating_sub(1), old_pos.1),
                                    Direction::Right => (old_pos.0 + 1, old_pos.1),
                                }
                            }
                            if let C2SPacket::Look(dir) | C2SPacket::Search(dir) = p {
                                _ = last_search.insert(dir);
                            } else {
                                _ = last_search.take();
                            }
                            if let C2SPacket::PutWall(dir) = p {
                                let pos = {
                                    let state = game.state.lock();
                                    let size = state.map_size;
                                    let us = (
                                        state.players.us.pos.0 as isize,
                                        state.players.us.pos.1 as isize,
                                    );
                                    let shift = match dir {
                                        Direction::Top => (0, -1),
                                        Direction::Bottom => (0, 1),
                                        Direction::Left => (-1, 0),
                                        Direction::Right => (1, 0),
                                    };
                                    let new_wall = (us.0 + shift.0, us.1 + shift.1);
                                    if (0..size.0 as isize).contains(&new_wall.0)
                                        && (0..size.1 as isize).contains(&new_wall.1)
                                    {
                                        Some((new_wall.0 as usize, new_wall.1 as usize))
                                    } else {
                                        None
                                    }
                                };
                                if let Some((x, y)) = pos {
                                    game.state.lock().map.set(x, y, Element::Wall);
                                }
                            }
                            game.client.send(p);
                            ready = false;
                        }
                        _ => (),
                    }
                }
            }
        });

        ChaserHandle {
            state: state2,
            send: c2s_arc2,
        }
    }

    #[inline]
    pub fn run_loop(quit: bool, handle: ChaserHandle, mut f: impl FnMut(&ChaserHandle)) {
        loop {
            // wait a bit for sync
            thread::sleep(Duration::from_millis(50));
            if !matches!(handle.info().phase, GamePhase::Ended { .. }) {
                f(&handle)
            } else if quit {
                break;
            }
        }
    }
}
impl ChaserHandle {
    pub fn info(&self) -> MutexGuard<'_, GameState> {
        self.state.lock()
    }
    pub fn send(&self, packet: C2SPacket) {
        _ = self.send.lock().insert(packet);
    }
}
