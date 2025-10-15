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
