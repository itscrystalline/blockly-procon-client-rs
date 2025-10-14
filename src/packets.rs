use serde::{Deserialize, Serialize};

use crate::game_types::{Direction, GameData, Side};

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
        x_size: u32,
        y_size: u32,
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
        rec_data: Option<Vec<u8>>,
    },
    MoveRec {
        rec_data: Vec<u8>,
    },
    LookRec {
        rec_data: Vec<u8>,
    },
    SearchRec {
        rec_data: Vec<u8>,
    },
    PutRec {
        rec_data: Vec<u8>,
    },
}
