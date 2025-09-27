use eframe::emath::Vec2;
use tracing::Level;
use tracing_subscriber::fmt;

mod app;
mod components;
mod config;
mod drawing;
mod tests;
mod ui_helpers;
mod ui_state;
mod windows;

fn main() {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2 {
            x: app::INITIAL_WINDOW_WIDTH,
            y: app::INITIAL_WINDOW_HEIGHT,
        }),
        ..Default::default()
    };
    fmt().with_max_level(Level::INFO).init();
    let (engine_commands_sender, engine_commands_receiver) = std::sync::mpsc::channel();
    let (engine_events_sender, engine_events_receiver) = std::sync::mpsc::channel();
    let _ = eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| {
            Box::new(app::MyEguiApp::new(
                cc,
                engine_commands_sender,
                engine_events_sender,
                engine_events_receiver,
                engine_commands_receiver,
            ))
        }),
    );
}
