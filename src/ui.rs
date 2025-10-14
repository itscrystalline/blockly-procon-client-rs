use egui::{Color32, RichText};
use egui::{FontFamily::Proportional, TextStyle};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;

use eframe::EventLoopBuilderHook;
use eframe::egui;
use egui::FontId;
use winit::platform::wayland::EventLoopBuilderExtWayland;

use crate::game::GameState;
use crate::game_types::Element;

struct ChaserMonitor(Arc<RwLock<GameState>>);

pub fn start_ui(state: Arc<RwLock<GameState>>) {
    thread::spawn(move || {
        let (room, name, opp_name) = {
            let info = state.read();
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

impl eframe::App for ChaserMonitor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut style = (*ctx.style()).clone();
            style.text_styles = [(TextStyle::Button, FontId::new(24.0, Proportional))].into();
            ui.style_mut().text_styles = style.text_styles;

            let info = self.0.read();
            let (map, (cols, rows)) = (&info.map, info.map_size);

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

                    let elem = map.at(col as usize, row as usize);
                    ui.put(
                        cell_rect,
                        egui::Button::new(RichText::new(elem.to_string()).color(match elem {
                            Element::Blank => Color32::PLACEHOLDER,
                            Element::Wall => Color32::WHITE,
                            Element::Heart => Color32::from_rgb(230, 69, 83),
                            Element::Cold => Color32::from_rgb(4, 165, 229),
                            Element::Hot => Color32::from_rgb(210, 15, 57),
                        })),
                    );
                }
            }
            ui.ctx().request_repaint();
        });
    }
}
