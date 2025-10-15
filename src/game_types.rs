use std::{cmp::min, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Deserialize, Clone, Copy)]
#[allow(dead_code)]
pub struct Effect {
    #[serde(rename = "t")]
    search: SearchType,
    #[serde(rename = "p")]
    pub player: Side,
    #[serde(rename = "d")]
    direction: Option<Direction>,
}

#[repr(transparent)]
#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct Map(Vec<Vec<Element>>);
impl Map {
    pub fn at(&self, x: usize, y: usize) -> Element {
        self.0[y][x]
    }
    pub fn find_player(&self, side: Side) -> Option<(usize, usize)> {
        let to_find = match side {
            Side::Hot => Element::Hot,
            Side::Cold => Element::Cold,
        };
        for (i, row) in self.0.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                if val == to_find {
                    return Some((j, i));
                }
            }
        }
        None
    }
    pub fn find_player_around(
        &self,
        side: Side,
        old_pos: (usize, usize),
        size: (usize, usize),
    ) -> Option<(usize, usize)> {
        let to_find = match side {
            Side::Hot => Element::Hot,
            Side::Cold => Element::Cold,
        };
        let min_x = old_pos.0.saturating_sub(1);
        let max_x = min(old_pos.0 + 1, size.0 - 1);
        let min_y = old_pos.1.saturating_sub(1);
        let max_y = min(old_pos.1 + 1, size.1 - 1);

        for i in min_x..=max_x {
            for j in min_y..=max_y {
                if self.at(i, j) == to_find {
                    return Some((i, j));
                }
            }
        }
        None
    }
}
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(into = "String")]
#[serde(from = "String")]
pub enum SearchType {
    AroundCurrent,
    AroundSide,
    Direction,
}
impl From<SearchType> for String {
    fn from(value: SearchType) -> Self {
        match value {
            SearchType::AroundCurrent => "r",
            SearchType::AroundSide => "l",
            SearchType::Direction => "s",
        }
        .to_string()
    }
}
impl From<String> for SearchType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "r" => SearchType::AroundCurrent,
            "l" => SearchType::AroundSide,
            "s" => SearchType::Direction,
            _ => unreachable!(),
        }
    }
}
