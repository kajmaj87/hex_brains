use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use bevy_ecs::prelude::{Entity, IntoSystemConfigs, Query, Res, ResMut, Resource, With, Without};
use eframe::{egui, emath};
use eframe::emath::{Pos2, Rect, Vec2};
use eframe::epaint::Color32;
use egui::{Frame, Key, Response, ScrollArea, Sense, Shape, Stroke};
use egui::epaint::CircleShape;
use egui::Shape::Circle;
use hex_brains_engine::core::{Food, Head, Position, Solid, Tail};
use hex_brains_engine::simulation::{Simulation, EngineEvent, EngineCommand, EngineState};
use hex_brains_engine::simulation_manager::simulate_batch;

fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2 { x: 1200.0, y: 1200.0 });
    let (engine_commands_sender, engine_commands_receiver) = std::sync::mpsc::channel();
    let (engine_events_sender, engine_events_receiver) = std::sync::mpsc::channel();
    eframe::run_native("My egui App", native_options, Box::new(|cc| {
        let context = cc.egui_ctx.clone();
        let config = Config {
            rows: 100,
            columns: 100,
            bg_color: Stroke::new(1.0, Color32::LIGHT_GREEN),
            snake_color: Stroke::new(1.0, Color32::RED),
        };
        let mut simulation = Simulation::new("Main".to_string(), engine_events_sender.clone(), Some(engine_commands_receiver), config.rows, config.columns);
        let egui_context = EguiEcsContext {
            context,
        };
        simulation.insert_resource(egui_context);
        simulation.insert_resource(config);
        simulation.insert_resource(EngineState {
            repaint_needed: false,
            speed_limit: Some(0.1),
            running: true,
            frames_left: 0.0,
            frames: 0,
            updates_done: 0,
        });
        simulation.add_system(draw_simulation.run_if(should_draw_simulation));
        thread::spawn(move || {
            simulation.run();
        });
        Box::new(MyEguiApp::new(cc, engine_commands_sender, engine_events_sender, engine_events_receiver))
    }));
}

fn draw_simulation(context: Res<EguiEcsContext>, mut config: ResMut<Config>, positions: Query<&Position>, heads: Query<(Entity, &Head)>, tails: Query<(Entity, &Tail)>, solids: Query<(Entity, &Solid)>, food: Query<(Entity, &Food)>) {
    puffin::profile_function!();
    egui::Window::new("Main Simulation").default_size(Vec2 { x: 1200.0, y: 1200.0 }).show(&context.context, |ui| {
        egui::stroke_ui(ui, &mut config.bg_color, "Background Color");
        egui::stroke_ui(ui, &mut config.snake_color, "Snake Color");
        Frame::canvas(ui.style()).show(ui, |ui| {
            let (mut response, _) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

            let to_screen = emath::RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                response.rect,
            );
            // let from_screen = to_screen.inverse();

            let heads: Vec<Shape> = heads.iter().map(|(head, _)| {
                let position = positions.get(head).unwrap();
                let p = Pos2 { x: position.x as f32, y: position.y as f32 };
                transform_to_circle(&p, &to_screen, &response, &config, config.snake_color.color)
            }).collect();
            let tails: Vec<Shape> = tails.iter().map(|(tail, _)| {
                let position = positions.get(tail).unwrap();
                let p = Pos2 { x: position.x as f32, y: position.y as f32 };
                transform_to_circle(&p, &to_screen, &response, &config, Color32::LIGHT_BLUE)
            }).collect();
            let solids: Vec<Shape> = solids.iter().map(|(solid, _)| {
                let position = positions.get(solid).unwrap();
                let p = Pos2 { x: position.x as f32, y: position.y as f32 };
                transform_to_circle(&p, &to_screen, &response, &config, Color32::BLACK)
            }).collect();
            let food: Vec<Shape> = food.iter().map(|(food, _)| {
                let position = positions.get(food).unwrap();
                let p = Pos2 { x: position.x as f32, y: position.y as f32 };
                transform_to_circle(&p, &to_screen, &response, &config, Color32::YELLOW)
            }).collect();

            let positions: Vec<Pos2> = (0..config.columns)
                .flat_map(|x| (0..config.rows).map(move |y| Pos2 { x: x as f32, y: y as f32 }))
                .collect();

            let mut ground: Vec<Shape> = positions.iter().map(|position| {
                transform_to_circle(position, &to_screen, &response, &config, config.bg_color.color)
            }).collect();
            ground.extend(food);
            ground.extend(solids);
            ground.extend(tails);
            ground.extend(heads);

            response.mark_changed();
            let painter = ui.painter();
            painter.extend(ground);
            // ground.iter().for_each(|shape| {
            //     painter.add(shape.clone());
            // });

            response
        });
    });
}

fn transform_to_circle(game_position: &Pos2, to_screen: &emath::RectTransform, response: &Response, config: &Config, color: Color32) -> Shape {
    // Radius is based on window's dimensions and the desired number of circles.
    let radius = 1.0 / (2.0 * config.rows as f32);

    // Offset every second row
    let offset = if game_position.y as i32 % 2 == 0 { radius } else { 0.0 };

    // Normalize the game position
    let normalized_position = Pos2 {
        x: game_position.x / config.columns as f32 + offset + radius,
        y: game_position.y / config.rows as f32 + radius,
    };

    // Convert normalized position to screen position
    let screen_position = to_screen * normalized_position;

    Circle(CircleShape {
        center: screen_position,
        radius: radius * response.rect.height(), // Using the normalized radius for the screen
        fill: color,
        stroke: Default::default(),
    })
}

fn should_draw_simulation(engine_state: Res<EngineState>) -> bool {
    engine_state.repaint_needed
}

#[derive(Resource)]
struct EguiEcsContext {
    context: egui::Context,
}

#[derive(Resource)]
struct Config {
    rows: usize,
    columns: usize,
    bg_color: Stroke,
    snake_color: Stroke,
}

struct MyEguiApp {
    text: String,
    total_finished: usize,
    engine_commands_sender: Sender<EngineCommand>,
    engine_events_sender: Sender<EngineEvent>,
    engine_events_receiver: Receiver<EngineEvent>,
    can_draw_frame: bool
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
            can_draw_frame: true,
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
                EngineEvent::FrameDrawn { updates_left, updates_done } => {
                    self.text = format!("{:.1} updates left, {} updates done", updates_left, updates_done);
                    self.can_draw_frame = true;
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
                        let mut result = Simulation::new(format!("Simulation {}", i), self.engine_events_sender.clone(), None, 200, 200);
                        result.insert_resource(EngineState {
                            repaint_needed: false,
                            speed_limit: None,
                            running: true,
                            frames_left: 0.0,
                            frames: 0,
                            updates_done: 0,
                        });
                        result
                    })
                    .collect();
                thread::spawn(move || {
                    simulate_batch(simulations);
                });
            }
            if ui.button("Create Snakes").clicked() {
                self.engine_commands_sender.send(EngineCommand::CreateSnakes(10)).unwrap();
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
        });
        if self.can_draw_frame {
            ctx.request_repaint();
            self.can_draw_frame = false;
        }
        self.engine_commands_sender.send(EngineCommand::RepaintRequested);
    }
}