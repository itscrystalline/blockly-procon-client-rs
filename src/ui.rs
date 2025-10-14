use std::thread;

use crate::game::{ChaserHandle, GameState};

struct ChaserMonitor(ChaserHandle);

fn start_ui(handle: ChaserHandle) {
    let (room, name, opponent_name) = {
        let info = handle.info();
        (
            info.room.clone(),
            info.name.clone(),
            info.opponent_name.clone(),
        )
    };
    thread::spawn(move || {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default(),
            ..Default::default()
        };
        eframe::run_native(
            &format!("Chaser Room '{room}': {name} vs {opponent_name}"),
            native_options,
            Box::new(|_ctx| Ok(Box::new(ChaserMonitor(handle)))),
        )
        .expect("egui crashed!");
    });
}

impl eframe::App for ChaserMonitor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("game_board")
                .min_col_width(50.0)
                .min_row_height(50.0)
                .spacing((3.0, 3.0))
                .show(ui, |ui| {
                    let info = self.0.info();
                    let map = &info.map;
                    let (x, y) = info.map_size;
                    for y in 0..y {
                        for x in 0..x {
                            let elem = map.at(x as usize, y as usize);
                            _ = ui.button("").clicked();
                        }
                        ui.end_row();
                    }
                });
        });
    }
}
