use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Instant;
use bevy_ecs::prelude::{Entity, IntoSystemConfigs, Query, Res, ResMut, Resource, With, Without};
use eframe::{egui, emath};
use eframe::emath::{Pos2, Rect, Vec2};
use eframe::epaint::Color32;
use egui::{Frame, Key, Response, ScrollArea, Sense, Shape, Stroke, Ui};
use egui::epaint::CircleShape;
use egui::Shape::Circle;
use hex_brains_engine::core::{Food, Snake, Position, Solid};
use hex_brains_engine::simulation::{Simulation, EngineEvent, EngineCommand, EngineState, EngineEvents, Hex, HexType, SimulationConfig, Stats};
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
            tail_color: Stroke::new(1.0, Color32::LIGHT_RED),
            food_color: Stroke::new(1.0, Color32::YELLOW),
        };
        let simulation_config = create_simulation_config(config.columns, config.rows);
        let mut simulation = Simulation::new("Main".to_string(), engine_events_sender.clone(), Some(engine_commands_receiver), simulation_config);
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
        Box::new(MyEguiApp::new(cc, engine_commands_sender, engine_events_sender, engine_events_receiver, config))
    }));
}

fn create_simulation_config(columns: usize, rows: usize) -> SimulationConfig{
    SimulationConfig {
        rows,
        columns,
        starting_snakes: 10,
        starting_food: 100,
        food_per_step: 5,
        energy_per_segment: 100,
        wait_cost: 1,
        move_cost: 10,
        energy_to_grow: 200,
        size_to_split: 10,
    }
}
fn draw_simulation(mut engine_events: ResMut<EngineEvents>, positions: Query<&Position>, snakes: Query<(Entity, &Snake)>, solids: Query<(Entity, &Solid)>, food: Query<(Entity, &Food)>, stats: Res<Stats>) {
    puffin::profile_function!();
    let all_hexes: Vec<Hex> = solids.iter().map(|(solid, _)| {
        let position = positions.get(solid).unwrap();
        Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::SnakeTail }
    }).chain(snakes.iter().map(|(head, _)| {
        let position = positions.get(head).unwrap();
        Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::SnakeHead }
    })).chain(food.iter().map(|(food, _)| {
        let position = positions.get(food).unwrap();
        Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::Food }
        // transform_to_circle(&p, &to_screen, &response, &config, Color32::YELLOW)
    })).collect();
    engine_events.events.lock().unwrap().send(EngineEvent::DrawData { hexes: all_hexes, stats: stats.clone() });
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

#[derive(Resource, Clone, Copy)]
struct Config {
    rows: usize,
    columns: usize,
    bg_color: Stroke,
    snake_color: Stroke,
    food_color: Stroke,
    tail_color: Stroke,
}

struct MyEguiApp {
    text: String,
    total_frames: usize,
    last_frame: Instant,
    engine_commands_sender: Sender<EngineCommand>,
    engine_events_sender: Sender<EngineEvent>,
    engine_events_receiver: Receiver<EngineEvent>,
    can_draw_frame: bool,
    config: Config,
    hexes: Vec<Hex>,
    updates_last_second: u32,
    last_second: Instant,
    frames_last_second: u32,
    frames_per_second: u32,
    updates_per_second: u32,
    stats: Stats
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>, engine_commands_sender: Sender<EngineCommand>, engine_events_sender: Sender<EngineEvent>, engine_events_receiver: Receiver<EngineEvent>, config: Config) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            text: String::new(),
            total_frames: 0,
            updates_last_second: 0,
            frames_last_second: 0,
            frames_per_second: 0,
            updates_per_second: 0,
            last_frame: Instant::now(),
            last_second: Instant::now(),
            engine_commands_sender,
            engine_events_sender,
            engine_events_receiver,
            can_draw_frame: true,
            config,
            stats: Stats::default(),
            hexes: vec![],
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
            match result {
                EngineEvent::SimulationFinished { steps, name, duration } => {
                    self.text.push_str(&format!("\nSimulation {} finished in {} steps in {} ms", name, steps, duration));
                }
                EngineEvent::FrameDrawn { updates_left, updates_done } => {
                    self.text = format!("{:.1} updates left, {} updates done", updates_left, updates_done);
                    self.can_draw_frame = true;
                    self.total_frames += 1;
                    self.updates_last_second += updates_done;
                    self.frames_last_second += 1;
                }
                EngineEvent::DrawData { hexes, stats } => {
                    self.hexes = hexes;
                    self.stats = stats;
                }
            }
        });
        if self.last_second.elapsed().as_millis() > 1000 {
            self.last_second = Instant::now();
            self.updates_per_second = self.updates_last_second;
            self.frames_per_second = self.frames_last_second;
            self.updates_last_second = 0;
            self.frames_last_second = 0;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start profiling").clicked() {
                    puffin::set_scopes_on(true); // tell puffin to collect data
                }
                if ui.button("Simulate Batch").clicked() {
                    let simulations = (0..64)
                        .map(|i| {
                            let mut result = Simulation::new(format!("Simulation {}", i), self.engine_events_sender.clone(), None, create_simulation_config(self.config.columns, self.config.rows));
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
                if ui.button("Create Snakes").on_hover_text("Click to add 10 snakes. Press 's' to add one snake").clicked() {
                    self.engine_commands_sender.send(EngineCommand::CreateSnakes(10)).unwrap();
                }
                ui.label(format!("Total : {} ({:.1}ms/frame)", self.total_frames, (Instant::now().duration_since(self.last_frame)).as_millis()));
                ui.label(format!("FPS : {:.1}", self.frames_per_second));
                ui.label(format!("UPS : {}", self.updates_per_second));
                ui.label(format!("Speed : x{:.1}", self.updates_per_second as f32 / self.frames_per_second as f32));
                ui.label(format!("Oldest snake : {}", self.stats.oldest_snake));
                ui.label(format!("Total snakes/segments : {}/{}", self.stats.total_snakes, self.stats.total_solids));
                ui.label(format!("Total food : {}", self.stats.total_food));
            });
            draw_hexes(ui, &self.hexes, &self.config);
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
            if ctx.input(|i| i.key_pressed(Key::S)) {
                self.engine_commands_sender.send(EngineCommand::CreateSnakes(1)).unwrap();
            }
        });
        if self.can_draw_frame {
            ctx.request_repaint();
            self.can_draw_frame = false;
        }
        self.last_frame = Instant::now();
        self.engine_commands_sender.send(EngineCommand::RepaintRequested);
    }
}

fn draw_hexes(ui: &mut Ui, hexes: &Vec<Hex>, config: &Config) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            let (mut response, _) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

            let to_screen = emath::RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                response.rect,
            );
            // let from_screen = to_screen.inverse();

            let shapes: Vec<Shape> = hexes.iter().map(|hex| {
                let position = Pos2 { x: hex.x as f32, y: hex.y as f32 };
                let color = match hex.hex_type {
                    HexType::SnakeHead => Color32::RED,
                    HexType::SnakeTail => Color32::LIGHT_RED,
                    HexType::Food => Color32::YELLOW,
                };
                transform_to_circle(&position, &to_screen, &response, &config, color)
            }).collect();

            let positions: Vec<Pos2> = (0..config.columns)
                .flat_map(|x| (0..config.rows).map(move |y| Pos2 { x: x as f32, y: y as f32 }))
                .collect();

            let mut ground: Vec<Shape> = positions.iter().map(|position| {
                transform_to_circle(position, &to_screen, &response, &config, config.bg_color.color)
            }).collect();
            ground.extend(shapes);
            response.mark_changed();
            let painter = ui.painter();
            painter.extend(ground);
            response
        });

}