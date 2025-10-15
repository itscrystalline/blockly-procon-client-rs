use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use std::{
    ffi::OsStr,
    ops::Deref,
    sync::{
        Arc,
        mpsc::{Receiver, Sender, TryRecvError, channel},
    },
    thread,
    time::Duration,
};

use crate::{
    client::Client,
    game_types::{Direction, Effect, GameData, Map, Side},
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
    state: Arc<RwLock<GameState>>,
}
pub struct ChaserHandle {
    state: Arc<RwLock<GameState>>,
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

        let state = Arc::new(RwLock::new(GameState {
            room: map.clone(),
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
        let (ready_send, ready_recv) = channel::<()>();

        ui::start_ui(state3);

        thread::spawn(move || {
            let mut game = game;
            game.client.send(C2SPacket::GetReady);
            let mut ready = false;
            let mut ended = false;
            loop {
                thread::sleep(Duration::from_millis(20));
                if ended {
                    continue;
                }
                if let Some(p) = game.client.recv() {
                    match p {
                        S2CPacket::GameResult { winner, .. } => {
                            game.state.write().phase = GamePhase::Ended { winner }
                        }
                        S2CPacket::UpdateBoard(GameData {
                            map_data,
                            cool_score,
                            hot_score,
                            turn,
                            effect,
                        }) => {
                            let mut state = game.state.write();
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
                                }
                                println!("hey");

                                if player != state.players.us.side {
                                    game.client.send(C2SPacket::GetReady);
                                    ready = false;
                                }
                            }

                            state.map = map_data;
                            state.turns_left = turn;
                            state.effect = effect;
                            state.players.assign_scores(cool_score, hot_score);
                        }
                        S2CPacket::GetReadyRec { .. } => {
                            ready = true;
                            ready_send.send(()).expect("channel closed");
                        }
                        S2CPacket::MoveRec { .. }
                        | S2CPacket::LookRec { .. }
                        | S2CPacket::SearchRec { .. }
                        | S2CPacket::PutRec { .. } => (),
                        _ => (),
                    }
                }
                // send any pending packet
                if !matches!(game.state.read().phase, GamePhase::Ended { .. }) {
                    match c2s_arc1.lock().take() {
                        Some(p) if ready => {
                            if let C2SPacket::MovePlayer(dir) = p {
                                let old_pos = game.state.read().players.us.pos;
                                game.state.write().players.us.pos = match dir {
                                    Direction::Top => (old_pos.0, old_pos.1.saturating_sub(1)),
                                    Direction::Bottom => (old_pos.0, old_pos.1 + 1),
                                    Direction::Left => (old_pos.0.saturating_sub(1), old_pos.1),
                                    Direction::Right => (old_pos.0 + 1, old_pos.1),
                                }
                            }
                            game.client.send(p);
                            ready = false;
                        }
                        _ => (),
                    }
                } else {
                    println!("game over!");
                    ended = true;
                }
            }
        });

        ChaserHandle {
            state: state2,
            send: c2s_arc2,
            ready: ready_recv,
        }
    }
}
impl ChaserHandle {
    pub fn info(&self) -> RwLockReadGuard<'_, GameState> {
        self.state.read()
    }
    pub fn send(&self, packet: C2SPacket) {
        self.ready.recv().expect("channel closed");
        _ = self.send.lock().insert(packet);
    }
}
