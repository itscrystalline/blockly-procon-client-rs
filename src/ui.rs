use egui::FontFamily::Proportional;
use egui::TextStyle::Button;
use std::{sync::mpsc::Receiver, thread};

use eframe::EventLoopBuilderHook;
use eframe::egui;
use egui::FontId;
use winit::platform::wayland::EventLoopBuilderExtWayland;

use crate::game_types::Map;

struct ChaserMonitor {
    recv: Receiver<Map>,
    size: (u32, u32),
    map: Map,
}

pub fn start_ui(
    room: String,
    name: String,
    opp_name: String,
    size: (u32, u32),
    map: Map,
    map_update: Receiver<Map>,
) {
    thread::spawn(move || {
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
            Box::new(|_ctx| {
                Ok(Box::new(ChaserMonitor {
                    recv: map_update,
                    size,
                    map,
                }))
            }),
        )
        .expect("egui crashed!");
    });
}

impl eframe::App for ChaserMonitor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(map) = self.recv.try_recv() {
            self.map = map;
            println!("got map")
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut style = (*ctx.style()).clone();
            style.text_styles = [(Button, FontId::new(24.0, Proportional))].into();
            ui.style_mut().text_styles = style.text_styles;
            egui::Grid::new("game_board")
                .min_col_width(50.0)
                .min_row_height(50.0)
                .spacing((3.0, 3.0))
                .show(ui, |ui| {
                    let map = &self.map;
                    let (x, y) = self.size;
                    for y in 0..y {
                        for x in 0..x {
                            let elem = map.at(x as usize, y as usize);
                            ui.add_sized([50., 50.], egui::Button::new(elem.to_string()));
                        }
                        ui.end_row();
                    }
                });
            ui.allocate_space(ui.available_size());
        });
    }
}
