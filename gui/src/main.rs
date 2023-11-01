use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use bevy_ecs::prelude::{IntoSystemConfigs, Query, Res, Resource};
use eframe::egui;
use egui::{Key, ScrollArea};
use hex_brains_engine::simulation::{Position, Simulation, EngineEvent, EngineCommand, EngineState};
use hex_brains_engine::simulation_manager::simulate_batch;

fn main() {
    let native_options = eframe::NativeOptions::default();
    let (engine_commands_sender, engine_commands_receiver) = std::sync::mpsc::channel();
    let (engine_events_sender, engine_events_receiver) = std::sync::mpsc::channel();
    eframe::run_native("My egui App", native_options, Box::new(|cc| {
        let context = cc.egui_ctx.clone();
        let mut simulation = Simulation::new("Main".to_string(), engine_events_sender.clone(), Some(engine_commands_receiver));
        let egui_context = EguiEcsContext {
            context,
        };
        simulation.insert_resource(egui_context);
        simulation.insert_resource(EngineState {
            repaint_needed: false,
            speed_limit: Some(0.1),
            running: true,
            frames_left: 0.0,
            frames: 0
        });
        simulation.add_system(draw_simulation.run_if(should_draw_simulation));
        thread::spawn(move || {
            simulation.run();
        });
        Box::new(MyEguiApp::new(cc, engine_commands_sender, engine_events_sender, engine_events_receiver))
    }));
}

fn draw_simulation(context: Res<EguiEcsContext>, positions: Query<&Position>) {
    egui::Window::new("Main Simulation").show(&context.context, |ui| {
        ui.heading("Positions: ");
        positions.for_each(|position| {
            ui.label(format!("Position: ({}, {})", position.x, position.y));
        });
    });
}

fn should_draw_simulation(engine_state: Res<EngineState>) -> bool {
    engine_state.repaint_needed
}

#[derive(Resource)]
struct EguiEcsContext {
    context: egui::Context,
}

struct MyEguiApp {
    text: String,
    total_finished: usize,
    engine_commands_sender: Sender<EngineCommand>,
    engine_events_sender: Sender<EngineEvent>,
    engine_events_receiver: Receiver<EngineEvent>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>, engine_commands_sender: Sender<EngineCommand>, engine_events_sender: Sender<EngineEvent>, engine_events_receiver: Receiver<EngineEvent>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            text: String::new(),
            total_finished: 0,
            engine_commands_sender,
            engine_events_sender,
            engine_events_receiver,
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        puffin::profile_scope!("gui::update");
        if puffin::are_scopes_on() {
            puffin_egui::profiler_window(ctx);
            puffin::GlobalProfiler::lock().new_frame();
        }
        self.engine_events_receiver.try_iter().for_each(|result| {
            self.total_finished += 1;
            match result {
                EngineEvent::SimulationFinished { steps, name, duration } => {
                    self.text.push_str(&format!("\nSimulation {} finished in {} steps in {} ms", name, steps, duration));
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            ui.heading("Press/Hold/Release example. Press A to test.");
            if ui.button("Start profiling").clicked() {
                puffin::set_scopes_on(true); // tell puffin to collect data
            }
            if ui.button("Simulate Batch").clicked() {
                let simulations = (0..64)
                    .map(|i| {
                        let mut result = Simulation::new(format!("Simulation {}", i), self.engine_events_sender.clone(), None);
                        result.insert_resource(EngineState {
                            repaint_needed: false,
                            speed_limit: None,
                            running: true,
                            frames_left: 0.0,
                            frames: 0
                        });
                        result
                    })
                    .collect();
                thread::spawn(move || {
                    simulate_batch(simulations);
                });
            }
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.label(&self.text);
                });

            if ctx.input(|i| i.key_pressed(Key::PlusEquals)) {
                self.engine_commands_sender.send(EngineCommand::IncreaseSpeed).unwrap();
            }
            if ctx.input(|i| i.key_pressed(Key::Minus)) {
                self.engine_commands_sender.send(EngineCommand::DecreaseSpeed).unwrap();
            }
            if ctx.input(|i| i.key_pressed(Key::Num0)) {
                self.engine_commands_sender.send(EngineCommand::RemoveSpeedLimit).unwrap();
            }
            if ctx.input(|i| i.key_pressed(Key::Space)) {
                self.engine_commands_sender.send(EngineCommand::FlipRunningState).unwrap();
            }
            if ctx.input(|i| i.key_down(Key::A)) {
                self.text.push_str("\nHeld");
                ui.ctx().request_repaint(); // make sure we note the holding.
            }
            if ctx.input(|i| i.key_released(Key::A)) {
                self.text.push_str("\nReleased");
            }
        });
        ctx.request_repaint();
        self.engine_commands_sender.send(EngineCommand::RepaintRequested);
    }
}