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
        rec_data: Option<Vec<RecElement>>,
    },
    MoveRec {
        rec_data: Vec<RecElement>,
    },
    LookRec {
        rec_data: Vec<RecElement>,
    },
    SearchRec {
        rec_data: Vec<RecElement>,
    },
    PutRec {
        rec_data: Vec<RecElement>,
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
