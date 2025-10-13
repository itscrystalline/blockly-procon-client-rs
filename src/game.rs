use std::{
    ffi::OsStr,
    sync::{
        Arc, RwLock, RwLockReadGuard,
        mpsc::{Receiver, Sender, TryRecvError, channel},
    },
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{
    client::Client,
    game_types::{Effect, GameData, Map},
    packets::{C2SPacket, S2CPacket},
};
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(into = "String")]
#[serde(from = "String")]
pub enum Side {
    Hot,
    Cold,
}
impl From<Side> for String {
    fn from(value: Side) -> Self {
        match value {
            Side::Hot => "hot",
            Side::Cold => "cold",
        }
        .to_string()
    }
}
impl From<String> for Side {
    fn from(value: String) -> Self {
        match value.as_str() {
            "hot" => Side::Hot,
            "cool" => Side::Cold,
            _ => unreachable!(),
        }
    }
}
pub enum GamePhase {
    Starting,
    Turn(Side),
    Ended { winner: Side },
}
pub struct GameState {
    pub room: String,
    pub name: String,
    pub opponent_name: String,
    pub phase: GamePhase,
    pub map: Map,
    pub map_size: (u32, u32),
    pub score: u32,
    pub opponent_score: u32,
    pub turns_left: u32,
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
            name,
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

        let state = Arc::new(RwLock::new(GameState {
            room: map,
            name: cool_name,
            opponent_name: hot_name,
            map: map_data,
            map_size: (x_size, y_size),
            score: cool_score,
            opponent_score: hot_score,
            turns_left: turn,
            phase: GamePhase::Starting,
        }));
        let state2 = Arc::clone(&state);

        let game = ChaserGame { client, state };

        let (c2s_send, c2s_recv) = channel::<C2SPacket>();
        let (ready_send, ready_recv) = channel::<()>();

        thread::spawn(move || {
            let mut game = game;
            game.client.send(C2SPacket::GetReady);
            let mut ready = false;
            loop {
                thread::sleep(Duration::from_millis(20));
                if let Some(p) = game.client.recv() {
                    match p {
                        S2CPacket::GameResult { winner, .. } => {
                            game.state.write().unwrap().phase = GamePhase::Ended { winner }
                        }
                        S2CPacket::UpdateBoard(GameData {
                            map_data,
                            cool_score,
                            hot_score,
                            turn,
                            effect,
                        }) => {
                            let mut state = game.state.write().unwrap();

                            state.map = map_data;
                            state.score = cool_score;
                            state.opponent_score = hot_score;
                            state.turns_left = turn;
                            if let Some(Effect { player, .. }) = effect {
                                state.phase = GamePhase::Turn(player);
                                if let Side::Hot = player {
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
                if !matches!(game.state.read().unwrap().phase, GamePhase::Ended { .. }) {
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
        self.state.read().unwrap()
    }
    pub fn send(&self, packet: C2SPacket) {
        self.ready.recv().expect("channel closed");
        self.send.send(packet).expect("channel closed");
    }
}
