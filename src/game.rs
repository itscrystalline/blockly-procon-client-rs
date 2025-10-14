use parking_lot::{RwLock, RwLockReadGuard};
use std::{
    ffi::OsStr,
    sync::{
        Arc,
        mpsc::{Receiver, Sender, TryRecvError, channel},
    },
    thread,
    time::Duration,
};

use crate::{
    client::Client,
    game_types::{Effect, GameData, Map, Side},
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
    pub map_size: (u32, u32),
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
    pub score: u32,
    pub side: Side,
}
pub struct ChaserGame {
    client: Client,
    state: Arc<RwLock<GameState>>,
}
pub struct ChaserHandle {
    state: Arc<RwLock<GameState>>,
    send: Sender<C2SPacket>,
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

        let players = if cool_name == name {
            Players {
                us: Player {
                    name: cool_name,
                    score: cool_score,
                    side: Side::Cold,
                },
                opponent: Player {
                    name: hot_name,
                    score: hot_score,
                    side: Side::Hot,
                },
            }
        } else if hot_name == name {
            Players {
                us: Player {
                    name: hot_name,
                    score: hot_score,
                    side: Side::Hot,
                },
                opponent: Player {
                    name: cool_name,
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

        let (c2s_send, c2s_recv) = channel::<C2SPacket>();
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

                            state.map = map_data;
                            state.turns_left = turn;
                            state.effect = effect;
                            state.players.assign_scores(cool_score, hot_score);

                            if let Some(Effect { player, .. }) = effect {
                                state.phase = GamePhase::Turn(player);
                                if player != state.players.us.side {
                                    game.client.send(C2SPacket::GetReady);
                                }
                            }
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
                let pkt = c2s_recv.try_recv();
                if !matches!(game.state.read().phase, GamePhase::Ended { .. }) {
                    match pkt {
                        Ok(p) if ready => {
                            game.client.send(p);
                            ready = false;
                        }
                        Err(TryRecvError::Disconnected) => panic!("channel disconnected"),
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
            send: c2s_send,
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
        self.send.send(packet).expect("channel closed");
    }
}
