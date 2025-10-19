#[cfg(feature = "ui")]
mod ui_enabled {
    use eframe::EventLoopBuilderHook;
    use eframe::egui;
    use egui::FontId;
    use egui::{Color32, RichText, Stroke};
    use egui::{FontFamily::Proportional, TextStyle};

    use parking_lot::Mutex;
    use std::cmp::{max, min};
    use std::ops::RangeInclusive;
    use std::sync::Arc;
    use std::thread;

    use winit::platform::wayland::EventLoopBuilderExtWayland;

    use crate::game::GameState;
    use crate::game_types::{Direction, Effect, Element, Map, SearchType, Side};

    struct ChaserMonitor(Arc<Mutex<GameState>>);

    pub fn start_ui(state: Arc<Mutex<GameState>>) {
        let env = std::env::var("NO_UI");
        if env.is_err() {
            thread::spawn(move || {
                let (room, name, opp_name) = {
                    let info = state.lock();
                    (
                        info.room.clone(),
                        info.players.us.name.clone(),
                        info.players.opponent.name.clone(),
                    )
                };
                let event_loop_builder: Option<EventLoopBuilderHook> =
                    Some(Box::new(|event_loop_builder| {
                        event_loop_builder.with_any_thread(true);
                    }));
                let native_options = eframe::NativeOptions {
                    event_loop_builder,
                    ..Default::default()
                };
                eframe::run_native(
                    &format!("Chaser Room '{room}': {name} vs {opp_name}"),
                    native_options,
                    Box::new(|_ctx| Ok(Box::new(ChaserMonitor(state)))),
                )
                .expect("egui crashed!");
            });
        }
    }

    fn highlight_at(
        pos: (usize, usize),
        map_size: (usize, usize),
        map: &Map,
        effect: &Effect,
    ) -> Option<Color32> {
        fn range(
            r_x: RangeInclusive<usize>,
            r_y: RangeInclusive<usize>,
        ) -> impl Iterator<Item = (usize, usize)> {
            (r_x).flat_map(move |x| (r_y.clone()).map(move |y| (x, y)))
        }
        macro_rules! r {
            ($pos: expr, $map_size: expr) => {
                $pos.saturating_sub(1)..=min($pos + 1, $map_size - 1)
            };
        }

        let Effect {
            search,
            player,
            direction,
        } = effect;
        let expected = match player {
            Side::Hot => Element::Hot,
            Side::Cold => Element::Cold,
        };

        let mut check_range = match search {
            SearchType::AroundCurrent => range(r!(pos.0, map_size.0), r!(pos.1, map_size.1)),
            SearchType::AroundSide => match direction {
                Some(Direction::Top) => range(r!(pos.0, map_size.0), r!(pos.1 + 2, map_size.1)),
                Some(Direction::Bottom) => range(
                    r!(pos.0, map_size.0),
                    r!(pos.1.saturating_sub(2), map_size.1),
                ),
                Some(Direction::Left) => range(r!(pos.0 + 2, map_size.0), r!(pos.1, map_size.1)),
                Some(Direction::Right) => range(
                    r!(pos.0.saturating_sub(2), map_size.0),
                    r!(pos.1, map_size.1),
                ),

                None => unreachable!(),
            },
            SearchType::Direction => match direction {
                Some(Direction::Top) => {
                    range(pos.0..=pos.0, pos.1 + 1..=min(pos.1 + 9, map_size.1 - 1))
                }
                Some(Direction::Bottom) => range(
                    pos.0..=pos.0,
                    max(0, pos.1.saturating_sub(9))..=pos.1.saturating_sub(1),
                ),
                Some(Direction::Left) => {
                    range(pos.0 + 1..=min(pos.0 + 9, map_size.0 - 1), pos.1..=pos.1)
                }
                Some(Direction::Right) => range(
                    max(0, pos.0.saturating_sub(9))..=pos.0.saturating_sub(1),
                    pos.1..=pos.1,
                ),

                None => unreachable!(),
            },
        };

        check_range
            .any(|(x, y)| map.at(x, y) == expected)
            .then_some(match search {
                SearchType::AroundCurrent => Color32::from_rgb(55, 66, 47),
                SearchType::AroundSide | SearchType::Direction => Color32::from_rgb(22, 62, 91),
            })
    }

    impl eframe::App for ChaserMonitor {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut style = (*ctx.style()).clone();
                style.text_styles = [(TextStyle::Button, FontId::new(24.0, Proportional))].into();
                ui.style_mut().text_styles = style.text_styles;

                let info = self.0.lock();
                let (map, (cols, rows), effect) = (&info.map, info.map_size, &info.effect);

                let available = ui.available_size();

                // Compute uniform cell sizes.
                let cell_w = available.x / cols as f32;
                let cell_h = available.y / rows as f32;

                // Now allocate that full area so egui knows we're using it.
                let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());

                // Draw your grid within that rect.

                for row in 0..rows {
                    for col in 0..cols {
                        let x = rect.left() + col as f32 * cell_w;
                        let y = rect.top() + row as f32 * cell_h;
                        let cell_rect = egui::Rect::from_min_size(
                            egui::pos2(x + 3.0, y + 3.0),
                            egui::vec2(cell_w - 3.0, cell_h - 3.0),
                        );

                        let elem = map.at(col, row);
                        let color = match elem {
                            Element::Blank => Color32::TRANSPARENT,
                            Element::Wall => Color32::WHITE,
                            Element::Heart => Color32::from_rgb(230, 69, 83),
                            Element::Cold => Color32::from_rgb(4, 165, 229),
                            Element::Hot => Color32::from_rgb(210, 15, 57),
                            Element::BothColdAndHot => Color32::WHITE,
                        };
                        let border_color = match elem {
                            Element::Blank => Color32::TRANSPARENT,
                            Element::Wall => Color32::TRANSPARENT,
                            Element::Heart => Color32::TRANSPARENT,
                            Element::Cold => Color32::from_rgb(4, 165, 229),
                            Element::Hot => Color32::from_rgb(210, 15, 57),
                            Element::BothColdAndHot => Color32::TRANSPARENT,
                        };
                        let mut btn =
                            egui::Button::new(RichText::new(elem.to_string()).color(color))
                                .stroke(Stroke::new(1.0, border_color));
                        if let Some(effect) = effect
                            && let Some(color) = highlight_at((col, row), (cols, rows), map, effect)
                        {
                            btn = btn.fill(color);
                        }
                        ui.put(cell_rect, btn);
                    }
                }
                ui.ctx().request_repaint();
            });
        }
    }
}

#[cfg(not(feature = "ui"))]
mod ui_disabled {
    use crate::game::GameState;
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[allow(dead_code)]
    pub fn start_ui(_state: Arc<Mutex<GameState>>) {}
}

#[cfg(not(feature = "ui"))]
pub use ui_disabled::*;
#[cfg(feature = "ui")]
pub use ui_enabled::*;
