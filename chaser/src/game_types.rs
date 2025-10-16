use std::{cmp::min, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
impl Direction {
    pub const fn flip(self) -> Self {
        match self {
            Direction::Top => Direction::Bottom,
            Direction::Bottom => Direction::Top,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
    pub const fn left(self) -> Self {
        match self {
            Direction::Top => Direction::Left,
            Direction::Bottom => Direction::Right,
            Direction::Left => Direction::Bottom,
            Direction::Right => Direction::Top,
        }
    }
    pub const fn right(self) -> Self {
        match self {
            Direction::Top => Direction::Right,
            Direction::Bottom => Direction::Left,
            Direction::Left => Direction::Top,
            Direction::Right => Direction::Bottom,
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
    BothColdAndHot,
}
impl From<u8> for Element {
    fn from(value: u8) -> Self {
        match value {
            0 => Element::Blank,
            1 => Element::Wall,
            2 => Element::Heart,
            3 => Element::Cold,
            4 => Element::Hot,
            34 | 43 => Element::BothColdAndHot,
            n => panic!("unwhown Element {n}"),
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
            Element::BothColdAndHot => "❄🔥",
        })
    }
}

/// WHY NOT USE THE SAME ONE??
#[repr(u8)]
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(from = "u8")]
pub enum RecElement {
    Blank,
    Opponent,
    Wall,
    Heart,
}
impl From<u8> for RecElement {
    fn from(value: u8) -> Self {
        match value {
            0 => RecElement::Blank,
            1 => RecElement::Opponent,
            2 => RecElement::Wall,
            3 => RecElement::Heart,
            _ => unreachable!(),
        }
    }
}
impl RecElement {
    pub fn into_elem(self, our_side: Side) -> Element {
        match self {
            RecElement::Blank => Element::Blank,
            RecElement::Opponent => match our_side {
                Side::Hot => Element::Cold,
                Side::Cold => Element::Hot,
            },
            RecElement::Wall => Element::Wall,
            RecElement::Heart => Element::Heart,
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

#[derive(Debug, Deserialize, Clone, Copy)]
#[allow(dead_code)]
pub struct Effect {
    #[serde(rename = "t")]
    pub search: SearchType,
    #[serde(rename = "p")]
    pub player: Side,
    #[serde(rename = "d")]
    pub direction: Option<Direction>,
}

#[repr(transparent)]
#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct Map(Vec<Vec<Element>>);
impl Map {
    pub fn empty(size: (usize, usize)) -> Map {
        Map(vec![vec![Element::Blank; size.0]; size.1])
    }

    pub fn at(&self, x: usize, y: usize) -> Element {
        self.0[y][x]
    }
    pub fn set(&mut self, x: usize, y: usize, elem: Element) -> bool {
        let e = self.0.get_mut(y).and_then(|row| row.get_mut(x));
        if let Some(e) = e {
            *e = elem;
            true
        } else {
            false
        }
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
