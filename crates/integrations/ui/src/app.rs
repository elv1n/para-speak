use eframe::egui;
#[cfg(target_os = "macos")]
use crate::macos;

pub struct OverlayApp {
    visible: bool,
}

impl Default for OverlayApp {
    fn default() -> Self {
        Self { visible: true }
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        {
            use std::sync::Once;
            static SETUP: Once = Once::new();
            SETUP.call_once(|| {
                log::info!("Setting up macOS window for all spaces");
                macos::setup_all_app_windows_for_spaces();
            });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if self.visible {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(300.0, 60.0),
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            let rect = ui.available_rect_before_wrap();
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(8),
                                egui::Color32::from_rgba_premultiplied(60, 60, 60, 180),
                            );
                            ui.label("Para-Speak");
                        },
                    );
                }

                if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                    self.visible = !self.visible;
                }
            });
    }
}

pub fn create_overlay_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false)
            .with_inner_size([300.0, 60.0])
            .with_position(egui::Pos2::new(
                (1920.0 - 300.0) / 2.0, // Center horizontally (assuming 1920px width)
                1080.0 - 100.0,         // Near bottom (assuming 1080px height)
            )),
        ..Default::default()
    }
}
