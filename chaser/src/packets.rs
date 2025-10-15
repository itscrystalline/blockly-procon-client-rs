use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

use crate::game_types::{Direction, Element, GameData, Side};

#[derive(Debug, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case", tag = "packet", content = "data")]
pub enum C2SPacket {
    PlayerJoin { room_id: String, name: String },
    GetReady,
    MovePlayer(Direction),
    Look(Direction),
    Search(Direction),
    PutWall(Direction),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case", tag = "packet", content = "data")]
pub enum S2CPacket {
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
        rec_data: Option<Vec<Element>>,
    },
    MoveRec {
        rec_data: Vec<Element>,
    },
    LookRec {
        rec_data: Vec<Element>,
    },
    SearchRec {
        rec_data: Vec<Element>,
    },
    PutRec {
        rec_data: Vec<Element>,
    },
}

impl Display for C2SPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            C2SPacket::PlayerJoin { .. } => "PlayerJoin",
            C2SPacket::GetReady => "GetReady",
            C2SPacket::MovePlayer(_) => "MovePlayer",
            C2SPacket::Look(_) => "Look",
            C2SPacket::Search(_) => "Search",
            C2SPacket::PutWall(_) => "PutWall",
        };
        write!(f, "{name}")
    }
}

impl fmt::Display for S2CPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            S2CPacket::JoinedRoom { .. } => "JoinedRoom",
            S2CPacket::GameResult { .. } => "GameResult",
            S2CPacket::NewBoard(_) => "NewBoard",
            S2CPacket::UpdateBoard(_) => "UpdateBoard",
            S2CPacket::GetReadyRec { .. } => "GetReadyRec",
            S2CPacket::MoveRec { .. } => "MoveRec",
            S2CPacket::LookRec { .. } => "LookRec",
            S2CPacket::SearchRec { .. } => "SearchRec",
            S2CPacket::PutRec { .. } => "PutRec",
        };
        write!(f, "{name}")
    }
}
