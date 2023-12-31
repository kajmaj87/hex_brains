use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Instant;
use bevy_ecs::prelude::*;
use eframe::{egui, emath};
use eframe::emath::{Pos2, Rect, Vec2};
use eframe::epaint::{Color32, Fonts};
use egui::{Align2, FontDefinitions, FontFamily, FontId, Frame, Key, Response, ScrollArea, Sense, Shape, Stroke, Ui};
use egui::epaint::CircleShape;
use egui::Shape::Circle;
use tracing::{info, Level};
use tracing_subscriber::fmt;
use hex_brains_engine::core::{Food, Snake, Position, Solid, ScentMap, Scent};
use hex_brains_engine::dna::SegmentType;
use hex_brains_engine::neural;
use hex_brains_engine::neural::{ConnectionGene, NodeGene, NodeType};
use hex_brains_engine::simulation::{Simulation, EngineEvent, EngineCommand, EngineState, EngineEvents, Hex, HexType, SimulationConfig, Stats, MutationConfig};
use hex_brains_engine::simulation_manager::simulate_batch;

fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(Vec2 { x: 1200.0, y: 1200.0 });
    fmt()
        .with_max_level(Level::INFO)
        .init();
    let (engine_commands_sender, engine_commands_receiver) = std::sync::mpsc::channel();
    let (engine_events_sender, engine_events_receiver) = std::sync::mpsc::channel();
    eframe::run_native("My egui App", native_options, Box::new(|cc| {
        Box::new(MyEguiApp::new(cc, engine_commands_sender, engine_events_sender, engine_events_receiver, engine_commands_receiver))
    }));
}

fn create_simulation_config(columns: usize, rows: usize, add_walls: bool) -> SimulationConfig {
    SimulationConfig {
        rows,
        columns,
        add_walls,
        create_scents: false,
        scent_diffusion_rate: 0.25,
        scent_dispersion_per_step: 150.0,
        starting_snakes: 10,
        starting_food: 100,
        food_per_step: 2,
        plant_matter_per_segment: 100.0,
        wait_cost: 1.0,
        move_cost: 10.0,
        new_segment_cost: 100.0,
        size_to_split: 10,
        species_threshold: 0.2,
        mutation: MutationConfig::default(),
        snake_max_age: 2_000,
        meat_energy_content: 5.0,
        plant_energy_content: 1.0,
    }
}

fn start_simulation(engine_events_sender: &Sender<EngineEvent>, engine_commands_receiver: Arc<Mutex<Receiver<EngineCommand>>>, context: egui::Context, config: Config) {
    let simulation_config = create_simulation_config(config.columns, config.rows, config.add_walls);
    let mut simulation = Simulation::new("Main".to_string(), engine_events_sender.clone(), Some(Arc::clone(&engine_commands_receiver)), simulation_config);
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
        ignore_speed_limit: false,
        finished: false,
    });
    simulation.add_system(draw_simulation.run_if(should_draw_simulation));
    thread::spawn(move || {
        simulation.run();
    });
}

fn draw_simulation(mut engine_events: ResMut<EngineEvents>, positions: Query<&Position>, scents: Query<(Entity, &Scent)>, scent_map: Res<ScentMap>, heads: Query<(Entity, &Snake)>, solids: Query<(Entity, &Solid), Without<SegmentType>>, segments: Query<(Entity, &SegmentType), With<SegmentType>>, food: Query<(Entity, &Food)>, stats: Res<Stats>) {
    puffin::profile_function!();
    let all_hexes: Vec<Hex> = solids.iter().map(|(solid, _)| {
        let position = positions.get(solid).unwrap();
        Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::SnakeTail }
    }).chain(food.iter().map(|(food_id, food)| {
        let position = positions.get(food_id).unwrap();
        if food.is_meat() {
            Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::Meat }
        } else {
            Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::Food }
        }
    })).chain(heads.iter().map(|(head, snake)| {
        let position = positions.get(head).unwrap();
        Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::SnakeHead { specie: snake.species.unwrap_or(0) } }
    })).chain(segments.iter().map(|(segment_id, segment_type)| {
        let position = positions.get(segment_id).unwrap();
        Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::Segment { segment_type: segment_type.clone() } }
    })).chain(scents.iter().map(|(scent, _)| {
        let position = positions.get(scent).unwrap();
        let value = scent_map.map.get(position);
        Hex { x: position.x as usize, y: position.y as usize, hex_type: HexType::Scent { value: *value } }
    })).collect();
    engine_events.events.lock().unwrap().send(EngineEvent::DrawData { hexes: all_hexes, stats: stats.clone() });
}

fn draw_neural_network(ui: &mut Ui, fonts: &Fonts, specie_id: u32, nodes: &Vec<&NodeGene>, connections: &Vec<&ConnectionGene>) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let (mut response, _) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
            response.rect,
        );

        let input_nodes = nodes.iter().filter(|node| node.node_type == neural::NodeType::Input).collect::<Vec<_>>();
        let output_nodes = nodes.iter().filter(|node| node.node_type == neural::NodeType::Output).collect::<Vec<_>>();

        let specie_marker = Circle(CircleShape {
            center: to_screen * Pos2 { x: 0.05, y: 0.05 },
            radius: 0.02 * response.rect.height(), // Using the normalized radius for the screen
            fill: u32_to_color(specie_id),
            stroke: Default::default(),
        });

        let input_colors = vec![Color32::LIGHT_GRAY, Color32::DARK_GRAY, Color32::KHAKI, Color32::KHAKI, Color32::KHAKI, Color32::YELLOW, Color32::YELLOW, Color32::YELLOW, Color32::RED, Color32::RED, Color32::RED, Color32::LIGHT_RED, Color32::LIGHT_RED, Color32::LIGHT_RED, Color32::YELLOW, Color32::RED, Color32::BLUE, Color32::GRAY];

        let input_node_shapes: Vec<Shape> = input_nodes.iter().enumerate().map(|(index, node)| {
            let position = get_node_position(index, NodeType::Input);
            let screen_position = to_screen * position;
            // let text = Shape::text(&fonts, screen_position, Align2::LEFT_CENTER, "Hello worlds", FontId::new(26.0, FontFamily::Monospace), Color32::WHITE);
            let circle = Circle(CircleShape {
                center: screen_position,
                radius: 0.02 * response.rect.height(), // Using the normalized radius for the screen
                fill: input_colors[index],
                stroke: Default::default(),
            });
            circle
        }).collect();
        let output_node_shapes: Vec<Shape> = output_nodes.iter().enumerate().map(|(index, node)| {
            let position = get_node_position(index, NodeType::Output);
            let screen_position = to_screen * position;

            Circle(CircleShape {
                center: screen_position,
                radius: 0.02 * response.rect.height(), // Using the normalized radius for the screen
                fill: Color32::LIGHT_RED,
                stroke: Default::default(),
            })
        }).collect();
        let connection_shapes: Vec<Shape> = connections.iter().map(|connection| {
            let from_node = connection.in_node;
            let to_node = connection.out_node - input_nodes.len();
            let from_position = get_node_position(from_node, NodeType::Input);
            let to_position = get_node_position(to_node, NodeType::Output);
            let from_screen_position = to_screen * from_position;
            let to_screen_position = to_screen * to_position;
            let color = if connection.weight > 0.0 {
                Color32::LIGHT_GREEN
            } else {
                Color32::LIGHT_RED
            };
            Shape::line_segment(
                [from_screen_position, to_screen_position],
                Stroke::new(connection.weight.abs() / 30.0 * response.rect.height(), color),
            )
        }).collect();
        let painter = ui.painter();
        let input_node_names = vec![
            "bias",
            "chaos",
            "scent front",
            "scent left",
            "scent right",
            "plant v. front",
            "plant v. left",
            "plant v. right",
            "meat v. front",
            "meat v. left",
            "meat v. right",
            "solid v. front",
            "solid v. left",
            "solid v. right",
            "plant food level",
            "meat food level",
            "energy level",
            "age level"];
        let output_node_names = vec!["move forward", "move left", "move right", "wait"];
        painter.extend(vec![specie_marker]);
        painter.extend(connection_shapes);
        painter.extend(input_node_shapes);
        painter.extend(output_node_shapes);
        input_node_names.iter().enumerate().for_each(|(i, name)| {
            painter.text(to_screen * (get_node_position(i, NodeType::Input) - Vec2{ x: 0.05, y: 0.0 }), Align2::RIGHT_CENTER, name, FontId::new(12.0, FontFamily::Monospace), Color32::WHITE);
        });
        output_node_names.iter().enumerate().for_each(|(i, name)| {
            painter.text(to_screen * (get_node_position(i, NodeType::Output) + Vec2{ x: 0.05, y: 0.0 }), Align2::LEFT_CENTER, name, FontId::new(12.0, FontFamily::Monospace), Color32::WHITE);
        });
        response
    });
}

fn get_node_position(index: usize, node_type: NodeType) -> Pos2 {
    match node_type {
        NodeType::Input => {
            Pos2 { x: 0.25, y: 0.1 + index as f32 * 0.075 }
        }
        NodeType::Hidden => {
            Pos2 { x: 0.5, y: 0.1 + index as f32 * 0.075 }
        }
        NodeType::Output => {
            Pos2 { x: 0.85, y: 0.1 + index as f32 * 0.4 }
        }
    }
}

fn draw_hexes(ui: &mut Ui, hexes: &Vec<Hex>, config: &Config) {
    Frame::canvas(ui.style()).fill(config.bg_color.color).show(ui, |ui| {
        let (mut response, _) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
            response.rect,
        );

        // let from_screen = to_screen.inverse();
        let segment_alpha = 0.8;
        let muscle_color = with_alpha(Color32::LIGHT_RED, segment_alpha);
        let solid_color = with_alpha(Color32::BROWN, segment_alpha);
        let solar_color = with_alpha(Color32::LIGHT_BLUE, segment_alpha);
        let stomach_color = with_alpha(Color32::LIGHT_GREEN, segment_alpha);

        let shapes: Vec<Shape> = hexes.iter().map(|hex| {
            let position = Pos2 { x: hex.x as f32, y: hex.y as f32 };
            let color = match &hex.hex_type {
                HexType::SnakeHead { specie } => u32_to_color(*specie),
                HexType::SnakeTail => config.tail_color.color,
                HexType::Food => config.food_color.color,
                HexType::Meat => Color32::RED,
                HexType::Scent { value } => with_alpha(config.scent_color.color, config.scent_color.color.a() as f32 * value),
                HexType::Segment { segment_type } => {
                    match &segment_type {
                        SegmentType::Muscle(_) => muscle_color,
                        SegmentType::Solid(_) => solid_color,
                        SegmentType::Solar(_) => solar_color,
                        SegmentType::Stomach(_) => stomach_color,
                    }
                }
            };
            transform_to_circle(&position, &to_screen, &response, &config, color)
        }).collect();

        // let positions: Vec<Pos2> = (0..config.columns)
        //     .flat_map(|x| (0..config.rows).map(move |y| Pos2 { x: x as f32, y: y as f32 }))
        //     .collect();
        let positions = vec![];

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

fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (alpha * 256.0) as u8)
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
    scent_color: Stroke,
    food_color: Stroke,
    tail_color: Stroke,
    add_walls: bool,
}

struct MyEguiApp {
    text: String,
    total_frames: usize,
    last_frame: Instant,
    engine_commands_sender: Sender<EngineCommand>,
    engine_events_sender: Sender<EngineEvent>,
    engine_events_receiver: Receiver<EngineEvent>,
    engine_commands_receiver: Arc<Mutex<Receiver<EngineCommand>>>,
    can_draw_frame: bool,
    config: Config,
    hexes: Vec<Hex>,
    updates_last_second: u32,
    last_second: Instant,
    frames_last_second: u32,
    frames_per_second: u32,
    updates_per_second: u32,
    stats: Stats,
    show_simulation_settings: bool,
    show_mutation_settings: bool,
    show_species: bool,
    show_info: bool,
    simulation_config: SimulationConfig,
    simulation_running: bool,
    show_networks: bool,
    selected_network: u32,
    fonts: Fonts,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>, engine_commands_sender: Sender<EngineCommand>, engine_events_sender: Sender<EngineEvent>, engine_events_receiver: Receiver<EngineEvent>, engine_commands_receiver: Receiver<EngineCommand>) -> Self {
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
            engine_commands_receiver: Arc::new(Mutex::new(engine_commands_receiver)),
            config: Config {
                rows: 100,
                columns: 100,
                bg_color: Stroke::new(1.0, Color32::LIGHT_GREEN),
                scent_color: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xAD, 0xD8, 0xE6, 50)),
                tail_color: Stroke::new(1.0, Color32::LIGHT_RED),
                food_color: Stroke::new(1.0, Color32::YELLOW),
                add_walls: false,
            },
            simulation_config: SimulationConfig {
                rows: 100,
                columns: 100,
                create_scents: false,
                scent_diffusion_rate: 0.2,
                scent_dispersion_per_step: 30.0,
                starting_snakes: 0,
                starting_food: 0,
                food_per_step: 2,
                plant_matter_per_segment: 100.0,
                wait_cost: 1.0,
                move_cost: 10.0,
                new_segment_cost: 100.0,
                size_to_split: 12,
                species_threshold: 0.2,
                add_walls: false,
                mutation: MutationConfig::default(),
                snake_max_age: 2_000,
                meat_energy_content: 5.0,
                plant_energy_content: 1.0,
            },
            can_draw_frame: true,
            stats: Stats::default(),
            hexes: vec![],
            show_simulation_settings: false,
            show_mutation_settings: false,
            show_species: false,
            show_networks: false,
            show_info: false,
            simulation_running: false,
            selected_network: 0,
            fonts: Fonts::new(1.0, 2 * 1024, FontDefinitions::default()),
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
        egui::Window::new("Environment Settings").open(&mut self.show_simulation_settings).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Size");
                ui.add_enabled(!self.simulation_running, egui::DragValue::new(&mut self.config.columns).speed(1.0));
                self.config.rows = self.config.columns;
                self.simulation_config.rows = self.config.rows;
                self.simulation_config.columns = self.config.columns;
            });
            ui.horizontal(|ui| {
                ui.add_enabled(!self.simulation_running, egui::Checkbox::new(&mut self.config.add_walls, "Add walls"));
            });
            ui.horizontal(|ui| {
                ui.label("Food per step");
                ui.add(egui::DragValue::new(&mut self.simulation_config.food_per_step).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Energy per segment");
                ui.add(egui::DragValue::new(&mut self.simulation_config.plant_matter_per_segment).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Wait cost");
                ui.add(egui::DragValue::new(&mut self.simulation_config.wait_cost).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Move cost");
                ui.add(egui::DragValue::new(&mut self.simulation_config.move_cost).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("New segment energy cost");
                ui.add(egui::DragValue::new(&mut self.simulation_config.new_segment_cost).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Size to split");
                ui.add(egui::DragValue::new(&mut self.simulation_config.size_to_split).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Aging starts at");
                ui.add(egui::DragValue::new(&mut self.simulation_config.snake_max_age).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Species coloring threshold");
                ui.add(egui::DragValue::new(&mut self.simulation_config.species_threshold).speed(1.0));
            });
            ui.add(egui::Checkbox::new(&mut self.simulation_config.create_scents, "Create smell (low performance, memory leaks)"));
            ui.horizontal(|ui| {
                ui.label("Smell diffusion rate");
                ui.add(egui::DragValue::new(&mut self.simulation_config.scent_diffusion_rate).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Smell dispersion rate per step");
                ui.add(egui::DragValue::new(&mut self.simulation_config.scent_dispersion_per_step).speed(1.0));
            });
        });
        egui::Window::new("Mutation Settings").open(&mut self.show_mutation_settings).show(ctx, |ui| {
            ui.label("Senses:");
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.simulation_config.mutation.chaos_input_enabled, "Chaos gene");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.simulation_config.mutation.scent_sensing_enabled, "Food smelling");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.simulation_config.mutation.plant_vision_enabled, "Plant vision");
            });
            ui.horizontal(|ui| {
                ui.label("Front range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.plant_vision_front_range).speed(1.0));
                ui.label("Left range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.plant_vision_left_range).speed(1.0));
                ui.label("Right range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.plant_vision_right_range).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.simulation_config.mutation.meat_vision_enabled, "Meat vision");
            });
            ui.horizontal(|ui| {
                ui.label("Front range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.meat_vision_front_range).speed(1.0));
                ui.label("Left range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.meat_vision_left_range).speed(1.0));
                ui.label("Right range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.meat_vision_right_range).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.simulation_config.mutation.obstacle_vision_enabled, "Obstacle vision");
            });
            ui.horizontal(|ui| {
                ui.label("Front range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.obstacle_vision_front_range).speed(1.0));
                ui.label("Left range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.obstacle_vision_left_range).speed(1.0));
                ui.label("Right range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.obstacle_vision_right_range).speed(1.0));
            });
            ui.label("Mutation settings:");
            ui.horizontal(|ui| {
                ui.label("Weights perturbation chance");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.weight_perturbation_chance).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Weights perturbation range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.weight_perturbation_range).speed(1.0));
            });
            ui.add(egui::Checkbox::new(&mut self.simulation_config.mutation.perturb_disabled_connections, "Perturb disabled connections"));
            ui.horizontal(|ui| {
                ui.label("Weights reset chance");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.weight_reset_chance).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Weights reset range");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.weight_reset_range).speed(1.0));
            });
            ui.add(egui::Checkbox::new(&mut self.simulation_config.mutation.perturb_disabled_connections, "Perturb reset connections"));
            ui.horizontal(|ui| {
                ui.label("Connection flip chance");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.connection_flip_chance).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Dna mutation chance");
                ui.add(egui::DragValue::new(&mut self.simulation_config.mutation.dna_mutation_chance).speed(1.0));
            });
        });
        egui::Window::new("Species").open(&mut self.show_species).show(ctx, |ui| {});
        egui::Window::new("Networks").open(&mut self.show_networks).show(ctx, |ui| {
            let specie_ids = &self.stats.species.species.iter().map(|specie| specie.id).collect::<Vec<u32>>();
            if specie_ids.len() == 0 {
                ui.label("No networks yet");
                return;
            }
            let selected_specie_in_list = specie_ids.contains(&self.selected_network);
            if !selected_specie_in_list {
                self.selected_network = specie_ids[0];
            }
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Specie")
                    .selected_text(format!("{:?}", self.selected_network))
                    .show_ui(ui, |ui| {
                        for specie_id in specie_ids {
                            ui.selectable_value(&mut self.selected_network, *specie_id, format!("{:?}", specie_id));
                        }
                    });
                if ui.button("Next").clicked() {
                    self.selected_network = specie_ids[(specie_ids.iter().position(|id| *id == self.selected_network).unwrap() + 1) % specie_ids.len()];
                }
                if ui.button("Previous").clicked() {
                    self.selected_network = specie_ids[(specie_ids.iter().position(|id| *id == self.selected_network).unwrap() + specie_ids.len() - 1) % specie_ids.len()];
                }
            });
            ui.collapsing("Information", |ui| {
                ui.label("Green connections mean that the weight is positive, red connections mean that the weight is negative. The thicker the connection, the higher the weight.");
                ui.label("Positive weight means the snake wants to do the given action if it encounters this sensory input.");
                ui.label("Bias is a constant value of 1.0, chaos is a random number from range 0.0 .. 1.0 generated each tick");
                ui.label("Network cost is the energy it takes each turn to 'think'");

                ui.horizontal(|ui| {
                    ui.label(
                        r#"Input Nodes:
                    "#);
                    ui.label(
                        r#"Output Nodes
                    Move Forward
                    Move Left
                    Move Right
                    Wait"#);
                });
            });
            if let Some(selected_specie) = self.stats.species.species.iter().find(|specie| specie.id == self.selected_network) {
                ui.label(format!("Network run cost: {}", selected_specie.leader_network.run_cost()));
                draw_neural_network(ui, &self.fonts, selected_specie.id, &selected_specie.leader_network.get_nodes(), &selected_specie.leader_network.get_active_connections());
            }
        });
        egui::Window::new("Info").open(&mut self.show_info).show(ctx, |ui| {
            ui.label("Press 's' to add one snake");
            ui.label("Press 'a' to stop simulation and advance one frame (useful for debug)");
            ui.label("Press '+' to increase speed");
            ui.label("Press '-' to decrease speed");
            ui.label("Press 'tab' to ignore speed limit");
            ui.label("Press 'p' to pause/resume");
            ui.label("All enabled settings take effect immediately");
            ui.label("To change disabled settings, stop the simulation first");
        });
        self.engine_commands_sender.send(EngineCommand::UpdateSimulationConfig(self.simulation_config.clone())).unwrap();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start profiling").clicked() {
                    puffin::set_scopes_on(true); // tell puffin to collect data
                }
                if ui.button("Simulate Batch").clicked() {
                    let simulations = (0..64)
                        .map(|i| {
                            let mut result = Simulation::new(format!("Simulation {}", i), self.engine_events_sender.clone(), None, create_simulation_config(self.config.columns, self.config.rows, false));
                            result.insert_resource(EngineState {
                                repaint_needed: false,
                                speed_limit: None,
                                running: true,
                                frames_left: 0.0,
                                frames: 0,
                                updates_done: 0,
                                finished: false,
                                ignore_speed_limit: false,
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
                ui.label(format!("Max generation : {}", self.stats.max_generation));
                ui.label(format!("Max mutations : {}", self.stats.max_mutations));
                ui.label(format!("Snakes/segments : {}/{}", self.stats.total_snakes, self.stats.total_segments));
                ui.label(format!("Food : {}", self.stats.total_food));
                ui.label(format!("Species : {}", self.stats.species.species.len()));
                ui.label(format!("Scents : {}", self.stats.total_scents));
                ui.label(format!("Entities : {}", self.stats.total_entities));
                ui.label(format!("Plants/Meat : {}/{}", self.stats.total_plants, self.stats.total_meat));
                ui.label(format!("Stomachs: P/M: {}/{}", self.stats.total_plants_in_stomachs, self.stats.total_meat_in_stomachs));
                ui.label(format!("Total snake energy : {}", self.stats.total_snake_energy));
                ui.label(format!("Total energy : {}", self.stats.total_energy));
            });
            ui.horizontal(|ui| {
                egui::stroke_ui(ui, &mut self.config.bg_color, "Background Color");
                egui::stroke_ui(ui, &mut self.config.scent_color, "Scent Color");
                egui::stroke_ui(ui, &mut self.config.tail_color, "Tail Color");
                egui::stroke_ui(ui, &mut self.config.food_color, "Food Color");
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.simulation_running, egui::Button::new("Start simulation")).clicked() {
                    start_simulation(&self.engine_events_sender, Arc::clone(&self.engine_commands_receiver), ctx.clone(), self.config);
                    self.simulation_running = true;
                }
                if ui.button("Stop simulation").clicked() {
                    self.engine_commands_sender.send(EngineCommand::StopSimulation).unwrap();
                    self.simulation_running = false;
                }
                if ui.button("Environment").clicked() {
                    self.show_simulation_settings = !self.show_simulation_settings;
                }
                if ui.button("Mutations").clicked() {
                    self.show_mutation_settings = !self.show_mutation_settings;
                }
                if ui.button("Species").clicked() {
                    self.show_species = !self.show_species;
                }
                if ui.button("Networks").clicked() {
                    self.show_networks = !self.show_networks;
                }
                if ui.button("Info").clicked() {
                    self.show_info = !self.show_info;
                }
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
            if ctx.input(|i| i.key_pressed(Key::Tab)) {
                self.engine_commands_sender.send(EngineCommand::IgnoreSpeedLimit).unwrap();
            }
            if ctx.input(|i| i.key_pressed(Key::P)) {
                self.engine_commands_sender.send(EngineCommand::FlipRunningState).unwrap();
            }
            if ctx.input(|i| i.key_pressed(Key::S)) {
                self.engine_commands_sender.send(EngineCommand::CreateSnakes(1)).unwrap();
            }
            if ctx.input(|i| i.key_pressed(Key::A)) {
                self.engine_commands_sender.send(EngineCommand::AdvanceOneFrame).unwrap();
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


fn u32_to_color(u: u32) -> Color32 {
    let mut hasher = DefaultHasher::new();
    u.hash(&mut hasher);
    let hash = hasher.finish();

    let r = (hash >> 16) as u8;
    let g = (hash >> 8) as u8;
    let b = hash as u8;

    Color32::from_rgb(r, g, b)
}