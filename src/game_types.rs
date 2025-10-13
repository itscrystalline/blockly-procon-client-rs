use serde::{Deserialize, Serialize};

use crate::Side;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "String")]
#[serde(from = "String")]
pub enum Direction {
    Top,
    Bottom,
    Left,
    Right,
}
impl From<Direction> for String {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Top => "top",
            Direction::Bottom => "bottom",
            Direction::Left => "left",
            Direction::Right => "right",
        }
        .to_string()
    }
}
impl From<String> for Direction {
    fn from(value: String) -> Self {
        match value.as_str() {
            "top" => Direction::Top,
            "bottom" => Direction::Bottom,
            "left" => Direction::Left,
            "right" => Direction::Right,
            _ => unreachable!(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(from = "u8")]
pub enum Element {
    Blank,
    Wall,
    Heart,
    Cold,
    Hot,
}
impl From<u8> for Element {
    fn from(value: u8) -> Self {
        match value {
            0 => Element::Blank,
            1 => Element::Wall,
            2 => Element::Heart,
            3 => Element::Cold,
            4 => Element::Hot,
            _ => unreachable!(),
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct GameData {
    pub map_data: Map,
    pub cool_score: u32,
    pub hot_score: u32,
    pub turn: u32,
    #[serde(default)]
    pub effect: Option<Effect>,
}

#[derive(Debug, Deserialize)]
pub struct Effect {
    #[serde(rename = "t")]
    _t: String,
    #[serde(rename = "p")]
    pub player: Side,
    #[serde(rename = "d")]
    _direction: Option<Direction>,
}

#[repr(transparent)]
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Map(Vec<Vec<Element>>);
impl Map {
    pub fn at(&self, x: usize, y: usize) -> Element {
        self.0[y][x]
    }
}
