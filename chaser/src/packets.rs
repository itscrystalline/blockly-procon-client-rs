use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

use crate::game_types::{Direction, GameData, RecElement, Side};

#[derive(Debug, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case", tag = "packet", content = "data")]
pub enum C2SPacket {
    PlayerJoin {
        room_id: String,
        name: String,
    },
    GetReady,
    MovePlayer(Direction),
    /// Looks in a 3x3 grid next to the player, shifted in the specified direction.
    Look(Direction),
    /// Looks in a 9 cell line starting next to the player, extending into the specified direction.
    Search(Direction),
    PutWall(Direction),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case", tag = "packet", content = "data")]
pub enum S2CPacket {
    Error(String),
    ConnectError(String),
    JoinedRoom {
        x_size: usize,
        y_size: usize,
        cool_name: String,
        hot_name: String,
    },
    GameResult {
        #[serde(rename = "winer")]
        winner: Side,
        info: String,
    },
    NewBoard(GameData),
    #[serde(rename = "updata_board")]
    UpdateBoard(GameData),
    GetReadyRec {
        #[serde(default)]
        rec_data: Vec<RecElement>,
    },
    MoveRec {
        #[serde(default)]
        rec_data: Vec<RecElement>,
    },
    LookRec {
        #[serde(default)]
        rec_data: Vec<RecElement>,
    },
    SearchRec {
        #[serde(default)]
        rec_data: Vec<RecElement>,
    },
    PutRec {
        #[serde(default)]
        rec_data: Vec<RecElement>,
    },
}

impl Display for C2SPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            C2SPacket::PlayerJoin { .. } => "PlayerJoin".to_string(),
            C2SPacket::GetReady => "GetReady".to_string(),
            C2SPacket::MovePlayer(dir) => format!("MovePlayer: {dir:?}"),
            C2SPacket::Look(dir) => format!("Look: {dir:?}"),
            C2SPacket::Search(dir) => format!("Search: {dir:?}"),
            C2SPacket::PutWall(dir) => format!("PutWall: {dir:?}"),
        };
        write!(f, "{name}")
    }
}

impl fmt::Display for S2CPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            S2CPacket::JoinedRoom { .. } => "JoinedRoom".to_string(),
            S2CPacket::GameResult { .. } => "GameResult".to_string(),
            S2CPacket::NewBoard(_) => "NewBoard".to_string(),
            S2CPacket::UpdateBoard(_) => "UpdateBoard".to_string(),
            S2CPacket::GetReadyRec { .. } => "GetReadyRec".to_string(),
            S2CPacket::MoveRec { .. } => "MoveRec".to_string(),
            S2CPacket::LookRec { .. } => "LookRec".to_string(),
            S2CPacket::SearchRec { .. } => "SearchRec".to_string(),
            S2CPacket::PutRec { .. } => "PutRec".to_string(),
            S2CPacket::Error(info) => format!("Error: {info}"),
            S2CPacket::ConnectError(info) => format!("ConnectError: {info}"),
        };
        write!(f, "{name}")
    }
}
