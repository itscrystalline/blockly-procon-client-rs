use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::game::Side;

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
impl Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Element::Blank => " ",
            Element::Wall => "⬛",
            Element::Heart => "❤",
            Element::Cold => "❄",
            Element::Hot => "🔥",
        })
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
#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct Map(Vec<Vec<Element>>);
impl Map {
    pub fn at(&self, x: usize, y: usize) -> Element {
        self.0[y][x]
    }
    pub fn empty(size: (u32, u32)) -> Map {
        Map(Vec::with_capacity(size.1 as usize))
    }
}
