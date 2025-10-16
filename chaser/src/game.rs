use parking_lot::{Mutex, MutexGuard};
use std::{
    cmp::min,
    ffi::OsStr,
    sync::{
        Arc,
        mpsc::{Receiver, channel},
    },
    thread,
    time::Duration,
};

use crate::{
    client::Client,
    game_types::{Direction, Effect, Element, GameData, Map, Side},
    packets::{C2SPacket, S2CPacket},
    ui,
};
pub enum GamePhase {
    Starting,
    Turn(Side),
    Ended { winner: Side },
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
    pub pos: (usize, usize),
    pub score: u32,
    pub side: Side,
}
pub struct ChaserGame {
    client: Client,
    state: Arc<Mutex<GameState>>,
}
pub struct ChaserHandle {
    state: Arc<Mutex<GameState>>,
    send: Arc<Mutex<Option<C2SPacket>>>,
    ready: Receiver<()>,
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
        let mut client = Client::with_server(&url);
        client.send(C2SPacket::PlayerJoin {
            room_id: map.clone(),
            name: name.clone(),
        });

        let S2CPacket::JoinedRoom {
            x_size,
            y_size,
            cool_name,
            hot_name,
        } = (loop {
            if let Some(p) = client.recv() {
                break p;
            }
        })
        else {
            panic!("unexpected packet received while waiting for game data")
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
                us: Player {
                    name: cool_name,
                    pos: cool_pos.expect("!cool_pos"),
                    score: cool_score,
                    side: Side::Cold,
                },
                opponent: Player {
                    name: hot_name,
                    pos: hot_pos.expect("!hot_pos"),
                    score: hot_score,
                    side: Side::Hot,
                },
            }
        } else if hot_name == name {
            Players {
                us: Player {
                    name: hot_name,
                    pos: hot_pos.expect("!hot_pos"),
                    score: hot_score,
                    side: Side::Hot,
                },
                opponent: Player {
                    name: cool_name,
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
            map: if cfg!(feature = "fog_of_war") {
                Map::empty((x_size, y_size))
            } else {
                map_data.clone()
            },
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
        let (ready_send, ready_recv) = channel::<()>();

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
                    continue;
                }
                if let Some(p) = game.client.recv() {
                    match p {
                        S2CPacket::GameResult { winner, .. } => {
                            game.state.lock().phase = GamePhase::Ended { winner }
                        }
                        S2CPacket::UpdateBoard(GameData {
                            mut map_data,
                            cool_score,
                            hot_score,
                            turn,
                            effect,
                        }) => {
                            let mut state = game.state.lock();
                            let size = state.map_size;

                            if let Some(Effect { player, .. }) = effect {
                                state.phase = GamePhase::Turn(player);

                                let p = if player == state.players.us.side {
                                    &mut state.players.us
                                } else {
                                    &mut state.players.opponent
                                };
                                let old_pos = p.pos;
                                if let Some(pos) =
                                    map_data.find_player_around(player, old_pos, size)
                                {
                                    p.pos = pos;
                                    if cfg!(feature = "fog_of_war") {
                                        map_data.set(
                                            pos.0,
                                            pos.1,
                                            match p.side {
                                                Side::Hot => Element::Hot,
                                                Side::Cold => Element::Cold,
                                            },
                                        );
                                    }
                                }

                                if player != state.players.us.side {
                                    game.client.send(C2SPacket::GetReady);
                                    ready = false;
                                }
                            }

                            if cfg!(not(feature = "fog_of_war")) {
                                state.map = map_data;
                            } else {
                                let us = state.players.us.pos;
                                let us_side = state.players.us.side;
                                let opp = state.players.opponent.pos;
                                let opp_side = state.players.opponent.side;
                                state.map.set(
                                    us.0,
                                    us.1,
                                    match us_side {
                                        Side::Hot => Element::Hot,
                                        Side::Cold => Element::Cold,
                                    },
                                );
                                state.map.set(
                                    opp.0,
                                    opp.1,
                                    match opp_side {
                                        Side::Hot => Element::Hot,
                                        Side::Cold => Element::Cold,
                                    },
                                );
                            }
                            state.turns_left = turn;
                            state.effect = effect;
                            state.players.assign_scores(cool_score, hot_score);
                        }
                        S2CPacket::GetReadyRec { .. } => {
                            ready = true;
                            ready_send.send(()).expect("channel closed");
                        }
                        S2CPacket::MoveRec { rec_data }
                        | S2CPacket::PutRec { rec_data }
                        | S2CPacket::LookRec { rec_data } => {
                            if cfg!(feature = "fog_of_war") {
                                let mut state = game.state.lock();
                                let pos = state.players.us.pos;
                                let side = state.players.us.side;
                                let map = &mut state.map;

                                let offset = match last_search {
                                    None => (0, 0),
                                    Some(Direction::Top) => (0, -1),
                                    Some(Direction::Bottom) => (0, 1),
                                    Some(Direction::Left) => (-1, 0),
                                    Some(Direction::Right) => (1, 0),
                                };

                                for (i, elem) in rec_data.into_iter().enumerate() {
                                    let x_offset = (i % 3) as isize - 1 + offset.0;
                                    let y_offset = (i / 3) as isize - 1 + offset.1;
                                    if let Some(x) = pos.0.checked_add_signed(x_offset)
                                        && let Some(y) = pos.1.checked_add_signed(y_offset)
                                    {
                                        if x_offset == 0 && y_offset == 0 {
                                            _ = map.set(
                                                x,
                                                y,
                                                match side {
                                                    Side::Hot => Element::Hot,
                                                    Side::Cold => Element::Cold,
                                                },
                                            );
                                        } else {
                                            _ = map.set(x, y, elem.into_elem(side));
                                        }
                                    }
                                }

                                _ = last_search.take();
                            }
                        }
                        S2CPacket::SearchRec { rec_data } if cfg!(feature = "fog_of_war") => {
                            if let Some(dir) = last_search {
                                let mut state = game.state.lock();
                                let pos = state.players.us.pos;
                                let side = state.players.us.side;
                                let map_size = state.map_size;
                                let map = &mut state.map;

                                let range = match dir {
                                    Direction::Top => {
                                        pos.1.saturating_sub(9)..=pos.1.saturating_sub(1)
                                    }
                                    Direction::Bottom => {
                                        min(pos.1 + 1, map_size.1)..=min(pos.1 + 9, map_size.1)
                                    }
                                    Direction::Left => {
                                        pos.0.saturating_sub(9)..=pos.0.saturating_sub(1)
                                    }
                                    Direction::Right => {
                                        min(pos.0 + 1, map_size.0)..=min(pos.0 + 9, map_size.0)
                                    }
                                };
                                let other_pos = match dir {
                                    Direction::Top | Direction::Bottom => pos.0,
                                    Direction::Left | Direction::Right => pos.1,
                                };

                                for (i, (elem, pos)) in rec_data.into_iter().zip(range).enumerate()
                                {
                                    if i == 5 {
                                        continue;
                                    }
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
                                    _ = map.set(x, y, elem.into_elem(side));
                                }
                            }
                            _ = last_search.take();
                        }
                        _ => (),
                    }
                }
                // send any pending packet
                if let GamePhase::Ended { winner } = game.state.lock().phase {
                    println!("game over! {winner:?} won!");
                    println!("we are {our_side:?}");
                    ended = Some(winner);
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
            ready: ready_recv,
        }
    }

    #[inline]
    pub fn run_loop(handle: ChaserHandle, mut f: impl FnMut(&ChaserHandle)) {
        loop {
            // wait a bit for sync
            thread::sleep(Duration::from_millis(50));
            if !matches!(handle.info().phase, GamePhase::Ended { .. }) {
                f(&handle)
            }
        }
    }
}
impl ChaserHandle {
    pub fn info(&self) -> MutexGuard<'_, GameState> {
        self.state.lock()
    }
    pub fn send(&self, packet: C2SPacket) {
        self.ready.recv().expect("channel closed");
        _ = self.send.lock().insert(packet);
    }
}
