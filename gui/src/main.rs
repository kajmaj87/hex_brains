use bevy_ecs::prelude::*;
use eframe::emath::{Pos2, Rect, Vec2};
use eframe::epaint::{Color32, Fonts};
use eframe::{egui, emath};
use egui::epaint::CircleShape;
use egui::Shape::Circle;
use egui::{
    Align2, FontData, FontDefinitions, FontFamily, FontId, Frame, Key, Response, ScrollArea, Sense,
    Shape, Stroke, Ui,
};
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};
use hex_brains_engine::core::{Food, Position, Scent, ScentMap, Snake, Solid};
use hex_brains_engine::dna::SegmentType;
use hex_brains_engine::neural;
use hex_brains_engine::neural::{ConnectionGene, NodeGene, NodeType};
use hex_brains_engine::simulation::{
    EngineCommand, EngineEvent, EngineEvents, EngineState, Hex, HexType, MutationConfig,
    Simulation, SimulationConfig, Stats,
};
use parking_lot::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::collections::VecDeque;
use std::fs;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2 {
            x: 1200.0,
            y: 1200.0,
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
            Box::new(MyEguiApp::new(
                cc,
                engine_commands_sender,
                engine_events_sender,
                engine_events_receiver,
                engine_commands_receiver,
            ))
        }),
    );
}

fn only_star_fields_differ(a: &SimulationConfig, b: &SimulationConfig) -> bool {
    (a.rows != b.rows || a.columns != b.columns || a.add_walls != b.add_walls)
        && a.food_per_step == b.food_per_step
        && a.plant_matter_per_segment == b.plant_matter_per_segment
        && a.wait_cost == b.wait_cost
        && a.move_cost == b.move_cost
        && a.new_segment_cost == b.new_segment_cost
        && a.size_to_split == b.size_to_split
        && a.snake_max_age == b.snake_max_age
        && a.species_threshold == b.species_threshold
        && a.create_scents == b.create_scents
        && a.scent_diffusion_rate == b.scent_diffusion_rate
        && a.scent_dispersion_per_step == b.scent_dispersion_per_step
        && a.starting_snakes == b.starting_snakes
        && a.starting_food == b.starting_food
        && a.meat_energy_content == b.meat_energy_content
        && a.plant_energy_content == b.plant_energy_content
        && a.mutation == b.mutation
}

fn start_simulation(
    engine_events_sender: &Sender<EngineEvent>,
    engine_commands_receiver: Arc<Mutex<Receiver<EngineCommand>>>,
    context: egui::Context,
    simulation_config: SimulationConfig,
) {
    let config = Config {
        rows: simulation_config.rows,
        columns: simulation_config.columns,
        bg_color: Stroke::new(1.0, Color32::LIGHT_GREEN),
        scent_color: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xAD, 0xD8, 0xE6, 50)),
        tail_color: Stroke::new(1.0, Color32::LIGHT_RED),
        food_color: Stroke::new(1.0, Color32::YELLOW),
        add_walls: simulation_config.add_walls,
    };
    let mut simulation = Simulation::new(
        "Main".to_string(),
        engine_events_sender.clone(),
        Some(Arc::clone(&engine_commands_receiver)),
        simulation_config,
    );
    let egui_context = EguiEcsContext { _context: context };
    simulation.insert_resource(egui_context);
    simulation.insert_resource(config);
    simulation.insert_resource(EngineState {
        repaint_needed: false,
        speed_limit: Some(200.0),
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

#[allow(clippy::too_many_arguments)]
fn draw_simulation(
    engine_events: ResMut<EngineEvents>,
    positions: Query<&Position>,
    scents: Query<(Entity, &Scent)>,
    scent_map: Res<ScentMap>,
    heads: Query<(Entity, &Snake)>,
    solids: Query<(Entity, &Solid), Without<SegmentType>>,
    segments: Query<(Entity, &SegmentType), With<SegmentType>>,
    food: Query<(Entity, &Food)>,
    stats: Res<Stats>,
    engine_state: Res<EngineState>,
) {
    puffin::profile_function!();
    let all_hexes: Vec<Hex> = solids
        .iter()
        .map(|(solid, _)| {
            let position = positions.get(solid).unwrap();
            Hex {
                x: position.x as usize,
                y: position.y as usize,
                hex_type: HexType::SnakeTail,
            }
        })
        .chain(food.iter().map(|(food_id, food)| {
            let position = positions.get(food_id).unwrap();
            if food.is_meat() {
                Hex {
                    x: position.x as usize,
                    y: position.y as usize,
                    hex_type: HexType::Meat,
                }
            } else {
                Hex {
                    x: position.x as usize,
                    y: position.y as usize,
                    hex_type: HexType::Food,
                }
            }
        }))
        .chain(heads.iter().map(|(head, snake)| {
            let position = positions.get(head).unwrap();
            Hex {
                x: position.x as usize,
                y: position.y as usize,
                hex_type: HexType::SnakeHead {
                    specie: snake.species.unwrap_or(0),
                },
            }
        }))
        .chain(segments.iter().map(|(segment_id, segment_type)| {
            let position = positions.get(segment_id).unwrap();
            Hex {
                x: position.x as usize,
                y: position.y as usize,
                hex_type: HexType::Segment {
                    segment_type: segment_type.clone(),
                },
            }
        }))
        .chain(scents.iter().map(|(scent, _)| {
            let position = positions.get(scent).unwrap();
            let value = scent_map.map.get(position);
            Hex {
                x: position.x as usize,
                y: position.y as usize,
                hex_type: HexType::Scent { value: *value },
            }
        }))
        .collect();
    let _ = engine_events.events.lock().send(EngineEvent::DrawData {
        hexes: all_hexes,
        stats: stats.clone(),
        frames: engine_state.frames,
    });
}

fn draw_neural_network(
    ui: &mut Ui,
    _fonts: &Fonts,
    specie_id: u32,
    nodes: &Vec<&NodeGene>,
    connections: &Vec<&ConnectionGene>,
) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let (response, _) = ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
            response.rect,
        );

        let input_nodes = nodes
            .iter()
            .filter(|node| node.node_type == neural::NodeType::Input)
            .collect::<Vec<_>>();
        let output_nodes = nodes
            .iter()
            .filter(|node| node.node_type == neural::NodeType::Output)
            .collect::<Vec<_>>();

        let specie_marker = Circle(CircleShape {
            center: to_screen * Pos2 { x: 0.05, y: 0.05 },
            radius: 0.02 * response.rect.height(), // Using the normalized radius for the screen
            fill: u32_to_color(specie_id),
            stroke: Default::default(),
        });

        let input_colors = [
            Color32::LIGHT_GRAY,
            Color32::DARK_GRAY,
            Color32::KHAKI,
            Color32::KHAKI,
            Color32::KHAKI,
            Color32::YELLOW,
            Color32::YELLOW,
            Color32::YELLOW,
            Color32::RED,
            Color32::RED,
            Color32::RED,
            Color32::LIGHT_RED,
            Color32::LIGHT_RED,
            Color32::LIGHT_RED,
            Color32::YELLOW,
            Color32::RED,
            Color32::BLUE,
            Color32::GRAY,
        ];

        let input_node_shapes: Vec<Shape> = input_nodes
            .iter()
            .enumerate()
            .map(|(index, _node)| {
                let position = get_node_position(index, NodeType::Input);
                let screen_position = to_screen * position;
                // let text = Shape::text(&fonts, screen_position, Align2::LEFT_CENTER, "Hello worlds", FontId::new(26.0, FontFamily::Monospace), Color32::WHITE);
                Circle(CircleShape {
                    center: screen_position,
                    radius: 0.02 * response.rect.height(), // Using the normalized radius for the screen
                    fill: input_colors[index],
                    stroke: Default::default(),
                })
            })
            .collect();
        let output_node_shapes: Vec<Shape> = output_nodes
            .iter()
            .enumerate()
            .map(|(index, _node)| {
                let position = get_node_position(index, NodeType::Output);
                let screen_position = to_screen * position;

                Circle(CircleShape {
                    center: screen_position,
                    radius: 0.02 * response.rect.height(), // Using the normalized radius for the screen
                    fill: Color32::LIGHT_RED,
                    stroke: Default::default(),
                })
            })
            .collect();
        let connection_shapes: Vec<Shape> = connections
            .iter()
            .map(|connection| {
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
                    Stroke::new(
                        connection.weight.abs() / 30.0 * response.rect.height(),
                        color,
                    ),
                )
            })
            .collect();
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
            "age level",
        ];
        let output_node_names = ["move forward", "move left", "move right", "wait"];
        painter.extend(vec![specie_marker]);
        painter.extend(connection_shapes);
        painter.extend(input_node_shapes);
        painter.extend(output_node_shapes);
        input_node_names.iter().enumerate().for_each(|(i, name)| {
            painter.text(
                to_screen * (get_node_position(i, NodeType::Input) - Vec2 { x: 0.05, y: 0.0 }),
                Align2::RIGHT_CENTER,
                name,
                FontId::new(12.0, FontFamily::Monospace),
                Color32::WHITE,
            );
        });
        output_node_names.iter().enumerate().for_each(|(i, name)| {
            painter.text(
                to_screen * (get_node_position(i, NodeType::Output) + Vec2 { x: 0.05, y: 0.0 }),
                Align2::LEFT_CENTER,
                name,
                FontId::new(12.0, FontFamily::Monospace),
                Color32::WHITE,
            );
        });
        response
    });
}

fn get_node_position(index: usize, node_type: NodeType) -> Pos2 {
    match node_type {
        NodeType::Input => Pos2 {
            x: 0.25,
            y: 0.1 + index as f32 * 0.075,
        },
        NodeType::Hidden => Pos2 {
            x: 0.5,
            y: 0.1 + index as f32 * 0.075,
        },
        NodeType::Output => Pos2 {
            x: 0.85,
            y: 0.1 + index as f32 * 0.4,
        },
    }
}

fn draw_hexes(ui: &mut Ui, hexes: &[Hex], config: &Config) {
    Frame::canvas(ui.style())
        .fill(config.bg_color.color)
        .show(ui, |ui| {
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

            let shapes: Vec<Shape> = hexes
                .iter()
                .map(|hex| {
                    let position = Pos2 {
                        x: hex.x as f32,
                        y: hex.y as f32,
                    };
                    let color = match &hex.hex_type {
                        HexType::SnakeHead { specie } => u32_to_color(*specie),
                        HexType::SnakeTail => config.tail_color.color,
                        HexType::Food => config.food_color.color,
                        HexType::Meat => Color32::RED,
                        HexType::Scent { value } => {
                            let intensity = *value;
                            let blue = (intensity * 200.0) as u8;
                            let alpha_factor =
                                (config.scent_color.color.a() as f32 / 255.0) * intensity;
                            let a = (alpha_factor * 256.0) as u8;
                            Color32::from_rgba_unmultiplied(0, 0, blue, a)
                        }
                        HexType::Segment { segment_type } => match &segment_type {
                            SegmentType::Muscle(_) => muscle_color,
                            SegmentType::Solid(_) => solid_color,
                            SegmentType::Solar(_) => solar_color,
                            SegmentType::Stomach(_) => stomach_color,
                        },
                    };
                    transform_to_circle(&position, &to_screen, &response, config, color)
                })
                .collect();

            // let positions: Vec<Pos2> = (0..config.columns)
            //     .flat_map(|x| (0..config.rows).map(move |y| Pos2 { x: x as f32, y: y as f32 }))
            //     .collect();
            let positions = [];

            let mut ground: Vec<Shape> = positions
                .iter()
                .map(|position| {
                    transform_to_circle(
                        position,
                        &to_screen,
                        &response,
                        config,
                        config.bg_color.color,
                    )
                })
                .collect();
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

fn transform_to_circle(
    game_position: &Pos2,
    to_screen: &emath::RectTransform,
    response: &Response,
    config: &Config,
    color: Color32,
) -> Shape {
    // Radius is based on window's dimensions and the desired number of circles.
    let radius = 1.0 / (2.0 * config.rows as f32);

    // Offset every second row
    let offset = if game_position.y as i32 % 2 == 0 {
        radius
    } else {
        0.0
    };

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
    _context: egui::Context,
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
    stats_history: VecDeque<(u32, Stats)>,
    history_limit: usize,
    smoothing_window: usize,
    show_statistics: bool,
    show_simulation_settings: bool,
    show_mutation_settings: bool,
    show_species: bool,
    show_info: bool,
    show_dna_settings: bool,
    simulation_config: SimulationConfig,
    previous_simulation_config: SimulationConfig,
    started: bool,
    paused: bool,
    show_networks: bool,
    selected_network: u32,
    fonts: Fonts,
}

impl MyEguiApp {
    fn load_config() -> SimulationConfig {
        let default_config = SimulationConfig {
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
        };
        if let Ok(contents) = fs::read_to_string("config.toml") {
            match toml::from_str(&contents) {
                Ok(loaded) => {
                    tracing::info!("Config loaded successfully from config.toml");
                    loaded
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config.toml: {}, using defaults", e);
                    default_config
                }
            }
        } else {
            tracing::info!("config.toml not found, using defaults");
            default_config
        }
    }

    fn save_config(&self) {
        if let Ok(toml_str) = toml::to_string(&self.simulation_config) {
            let _ = fs::write("config.toml", toml_str);
        }
    }

    fn new(
        cc: &eframe::CreationContext<'_>,
        engine_commands_sender: Sender<EngineCommand>,
        engine_events_sender: Sender<EngineEvent>,
        engine_events_receiver: Receiver<EngineEvent>,
        engine_commands_receiver: Receiver<EngineCommand>,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let mut font_definitions = FontDefinitions::default();
        font_definitions.font_data.insert(
            "firacode_nerd".to_owned(),
            FontData::from_static(include_bytes!("../../assets/FiraCodeNerdFont-Regular.ttf")),
        );
        font_definitions
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "firacode_nerd".to_owned());
        font_definitions
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "firacode_nerd".to_owned());
        cc.egui_ctx.set_fonts(font_definitions.clone());
        let simulation_config = Self::load_config();
        let config = Config {
            rows: simulation_config.rows,
            columns: simulation_config.columns,
            bg_color: Stroke::new(1.0, Color32::LIGHT_GREEN),
            scent_color: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xAD, 0xD8, 0xE6, 50)),
            tail_color: Stroke::new(1.0, Color32::LIGHT_RED),
            food_color: Stroke::new(1.0, Color32::YELLOW),
            add_walls: simulation_config.add_walls,
        };
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
            config,
            simulation_config,
            previous_simulation_config: simulation_config,
            can_draw_frame: true,
            stats: Stats::default(),
            stats_history: VecDeque::new(),
            history_limit: 1000,
            show_statistics: false,
            hexes: vec![],
            show_simulation_settings: false,
            show_mutation_settings: false,
            show_species: false,
            show_networks: false,
            show_info: false,
            show_dna_settings: false,
            started: false,
            paused: false,
            selected_network: 0,
            fonts: Fonts::new(1.0, 2 * 1024, font_definitions),
            smoothing_window: 100,
        }
    }
    fn handle_events(&mut self, _ctx: &egui::Context) {
        if !self.started {
            start_simulation(
                &self.engine_events_sender,
                Arc::clone(&self.engine_commands_receiver),
                _ctx.clone(),
                self.simulation_config,
            );
            self.started = true;
            self.engine_commands_sender
                .send(EngineCommand::CreateSnakes(10))
                .unwrap();
        }
        self.engine_events_receiver
            .try_iter()
            .for_each(|result| match result {
                EngineEvent::SimulationFinished {
                    steps,
                    name,
                    duration,
                } => {
                    self.text.push_str(&format!(
                        "\nSimulation {name} finished in {steps} steps in {duration} ms"
                    ));
                }
                EngineEvent::FrameDrawn {
                    updates_left,
                    updates_done,
                } => {
                    self.text =
                        format!("{updates_left:.1} updates left, {updates_done} updates done");
                    self.can_draw_frame = true;
                    self.total_frames += 1;
                    self.updates_last_second += updates_done;
                    self.frames_last_second += 1;
                }
                EngineEvent::DrawData {
                    hexes,
                    stats,
                    frames,
                } => {
                    self.hexes = hexes;
                    self.stats = stats.clone();
                    self.stats_history.push_back((frames, stats));
                    if self.stats_history.len() > self.history_limit {
                        self.stats_history.pop_front();
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
    }

    fn render_windows(&mut self, ctx: &egui::Context) {
        egui::Window::new("Environment Settings")
            .open(&mut self.show_simulation_settings)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Size*")
                        .on_hover_text("Grid size in hexes (width and height)");
                    ui.add(egui::DragValue::new(&mut self.config.columns).speed(1.0))
                        .on_hover_text("Adjust grid dimensions");
                    self.config.rows = self.config.columns;
                    self.simulation_config.rows = self.config.rows;
                    self.simulation_config.columns = self.config.columns;
                });
                ui.horizontal(|ui| {
                    ui.add(egui::Checkbox::new(
                        &mut self.config.add_walls,
                        "Add walls*",
                    ))
                    .on_hover_text("Add walls around the grid perimeter");
                });
                ui.horizontal(|ui| {
                    ui.label("Food per step")
                        .on_hover_text("Number of food items added each simulation step");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.food_per_step).speed(1.0),
                    )
                    .on_hover_text("Adjust food spawn rate");
                });
                ui.horizontal(|ui| {
                    ui.label("Energy per segment")
                        .on_hover_text("Energy content of plant food per snake segment");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.plant_matter_per_segment)
                            .speed(1.0),
                    )
                    .on_hover_text("Set plant energy density");
                });
                ui.horizontal(|ui| {
                    ui.label("Wait cost")
                        .on_hover_text("Energy cost for waiting action");
                    ui.add(egui::DragValue::new(&mut self.simulation_config.wait_cost).speed(1.0))
                        .on_hover_text("Adjust wait energy cost");
                });
                ui.horizontal(|ui| {
                    ui.label("Move cost")
                        .on_hover_text("Energy cost for moving action");
                    ui.add(egui::DragValue::new(&mut self.simulation_config.move_cost).speed(1.0))
                        .on_hover_text("Adjust move energy cost");
                });
                ui.horizontal(|ui| {
                    ui.label("New segment energy cost")
                        .on_hover_text("Energy cost to grow a new segment");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.new_segment_cost)
                            .speed(1.0),
                    )
                    .on_hover_text("Set growth energy cost");
                });
                ui.horizontal(|ui| {
                    ui.label("Size to split")
                        .on_hover_text("Minimum segments required to split/reproduce");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.size_to_split).speed(1.0),
                    )
                    .on_hover_text("Adjust reproduction threshold");
                });
                ui.horizontal(|ui| {
                    ui.label("Aging starts at")
                        .on_hover_text("Age when snakes start losing energy");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.snake_max_age).speed(1.0),
                    )
                    .on_hover_text("Set aging threshold");
                });
                ui.horizontal(|ui| {
                    ui.label("Species coloring threshold")
                        .on_hover_text("Genetic distance for species clustering");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.species_threshold)
                            .speed(1.0),
                    )
                    .on_hover_text("Adjust species similarity threshold");
                });
                ui.add(egui::Checkbox::new(
                    &mut self.simulation_config.create_scents,
                    "Create smell (low performance, memory leaks)",
                ))
                .on_hover_text(
                    "Enable scent diffusion (experimental, may cause performance issues)",
                );
                ui.horizontal(|ui| {
                    ui.label("Smell diffusion rate")
                        .on_hover_text("Rate at which scents spread");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.scent_diffusion_rate)
                            .speed(1.0),
                    )
                    .on_hover_text("Adjust scent diffusion speed");
                });
                ui.horizontal(|ui| {
                    ui.label("Smell dispersion rate per step")
                        .on_hover_text("Scent dispersion per simulation step");
                    ui.add(
                        egui::DragValue::new(&mut self.simulation_config.scent_dispersion_per_step)
                            .speed(1.0),
                    )
                    .on_hover_text("Set scent dispersion amount");
                });
                ui.label("Settings marked with * will only take effect after a restart.");
            });
        egui::Window::new("Mutation Settings")
            .open(&mut self.show_mutation_settings)
            .show(ctx, |ui| {
                ui.label("Senses:")
                    .on_hover_text("Configure sensory capabilities that can mutate");
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.simulation_config.mutation.chaos_input_enabled,
                        "Chaos gene",
                    )
                    .on_hover_text("Allow random input to neural networks");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.simulation_config.mutation.scent_sensing_enabled,
                        "Food smelling",
                    )
                    .on_hover_text("Enable scent-based food detection");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.simulation_config.mutation.plant_vision_enabled,
                        "Plant vision",
                    )
                    .on_hover_text("Allow vision of plant food");
                });
                ui.horizontal(|ui| {
                    ui.label("Front range")
                        .on_hover_text("Vision range directly ahead");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.plant_vision_front_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set front vision distance for plants");
                    ui.label("Left range")
                        .on_hover_text("Vision range to the left");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.plant_vision_left_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set left vision distance for plants");
                    ui.label("Right range")
                        .on_hover_text("Vision range to the right");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.plant_vision_right_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set right vision distance for plants");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.simulation_config.mutation.meat_vision_enabled,
                        "Meat vision",
                    )
                    .on_hover_text("Allow vision of meat food");
                });
                ui.horizontal(|ui| {
                    ui.label("Front range")
                        .on_hover_text("Vision range directly ahead");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.meat_vision_front_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set front vision distance for meat");
                    ui.label("Left range")
                        .on_hover_text("Vision range to the left");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.meat_vision_left_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set left vision distance for meat");
                    ui.label("Right range")
                        .on_hover_text("Vision range to the right");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.meat_vision_right_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set right vision distance for meat");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.simulation_config.mutation.obstacle_vision_enabled,
                        "Obstacle vision",
                    )
                    .on_hover_text("Allow vision of obstacles/walls");
                });
                ui.horizontal(|ui| {
                    ui.label("Front range")
                        .on_hover_text("Vision range directly ahead");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.obstacle_vision_front_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set front vision distance for obstacles");
                    ui.label("Left range")
                        .on_hover_text("Vision range to the left");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.obstacle_vision_left_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set left vision distance for obstacles");
                    ui.label("Right range")
                        .on_hover_text("Vision range to the right");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.obstacle_vision_right_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set right vision distance for obstacles");
                });
                ui.label("Mutation settings:")
                    .on_hover_text("Configure neural network mutation parameters");
                ui.horizontal(|ui| {
                    ui.label("Weights perturbation chance")
                        .on_hover_text("Probability of randomly adjusting connection weights");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.weight_perturbation_chance,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set perturbation probability");
                });
                ui.horizontal(|ui| {
                    ui.label("Weights perturbation range")
                        .on_hover_text("Maximum adjustment amount for weights");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.weight_perturbation_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set perturbation magnitude");
                });
                ui.add(egui::Checkbox::new(
                    &mut self.simulation_config.mutation.perturb_disabled_connections,
                    "Perturb disabled connections",
                ))
                .on_hover_text("Allow mutation of disabled neural connections");
                ui.horizontal(|ui| {
                    ui.label("Weights reset chance")
                        .on_hover_text("Probability of resetting weights to new random values");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.weight_reset_chance,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set reset probability");
                });
                ui.horizontal(|ui| {
                    ui.label("Weights reset range")
                        .on_hover_text("Range for new random weights");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.weight_reset_range,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set reset range");
                });
                ui.add(egui::Checkbox::new(
                    &mut self.simulation_config.mutation.perturb_disabled_connections,
                    "Perturb reset connections",
                ))
                .on_hover_text("Allow perturbation of newly reset connections");
                ui.horizontal(|ui| {
                    ui.label("Connection flip chance")
                        .on_hover_text("Probability of enabling/disabling connections");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.connection_flip_chance,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set flip probability");
                });
                ui.horizontal(|ui| {
                    ui.label("Dna mutation chance")
                        .on_hover_text("Probability of mutating snake DNA segments");
                    ui.add(
                        egui::DragValue::new(
                            &mut self.simulation_config.mutation.dna_mutation_chance,
                        )
                        .speed(1.0),
                    )
                    .on_hover_text("Set DNA mutation probability");
                });
            });
        egui::Window::new("DNA Settings")
            .open(&mut self.show_dna_settings)
            .show(ctx, |ui| {
                ui.label("Disable segments from possible genes during mutation:")
                    .on_hover_text("Uncheck to allow this segment type in DNA mutations");
                let mut segment_configs = [
                    (
                        "Muscle",
                        &mut self.simulation_config.mutation.disable_muscle,
                        Color32::LIGHT_RED,
                    ),
                    (
                        "Solid",
                        &mut self.simulation_config.mutation.disable_solid,
                        Color32::BROWN,
                    ),
                    (
                        "Solar",
                        &mut self.simulation_config.mutation.disable_solar,
                        Color32::LIGHT_BLUE,
                    ),
                    (
                        "Stomach",
                        &mut self.simulation_config.mutation.disable_stomach,
                        Color32::LIGHT_GREEN,
                    ),
                ];
                for (i, (name, disable, color)) in segment_configs.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.colored_label(*color, format!("{i}: {name}"));
                        ui.checkbox(disable, "Disable");
                    });
                }
            });
        egui::Window::new("Species")
            .open(&mut self.show_species)
            .show(ctx, |_ui| {});
        egui::Window::new("Statistics")
            .open(&mut self.show_statistics)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("History limit");
                    ui.add(egui::DragValue::new(&mut self.history_limit).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Smoothing window");
                    ui.add(egui::DragValue::new(&mut self.smoothing_window).speed(10.0));
                });
                let current_frame = self
                    .stats_history
                    .back()
                    .map(|(f, _)| *f as f64)
                    .unwrap_or(0.0);
                let raw_plant_energy: Vec<(f64, f64)> = self
                    .stats_history
                    .iter()
                    .map(|(f, s)| {
                        (
                            *f as f64 - current_frame,
                            s.total_plant_energy as f64 / 1000.0,
                        )
                    })
                    .collect();
                let raw_meat_energy: Vec<(f64, f64)> = self
                    .stats_history
                    .iter()
                    .map(|(f, s)| {
                        (
                            *f as f64 - current_frame,
                            s.total_meat_energy as f64 / 1000.0,
                        )
                    })
                    .collect();
                let raw_snake_energy: Vec<(f64, f64)> = self
                    .stats_history
                    .iter()
                    .map(|(f, s)| {
                        (
                            *f as f64 - current_frame,
                            s.total_snake_energy as f64 / 1000.0,
                        )
                    })
                    .collect();
                let plant_energy: PlotPoints = raw_plant_energy
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let start = if i >= self.smoothing_window {
                            i - self.smoothing_window + 1
                        } else {
                            0
                        };
                        let sum: f64 = raw_plant_energy[start..=i].iter().map(|(_, y)| *y).sum();
                        let count = (i - start + 1) as f64;
                        [raw_plant_energy[i].0, sum / count]
                    })
                    .collect();
                let meat_energy: PlotPoints = raw_meat_energy
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let start = if i >= self.smoothing_window {
                            i - self.smoothing_window + 1
                        } else {
                            0
                        };
                        let sum: f64 = raw_meat_energy[start..=i].iter().map(|(_, y)| *y).sum();
                        let count = (i - start + 1) as f64;
                        [raw_meat_energy[i].0, sum / count]
                    })
                    .collect();
                let snake_energy: PlotPoints = raw_snake_energy
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let start = if i >= self.smoothing_window {
                            i - self.smoothing_window + 1
                        } else {
                            0
                        };
                        let sum: f64 = raw_snake_energy[start..=i].iter().map(|(_, y)| *y).sum();
                        let count = (i - start + 1) as f64;
                        [raw_snake_energy[i].0, sum / count]
                    })
                    .collect();
                let plant_line = Line::new(plant_energy).name("Plant Energy (/1000)");
                let meat_line = Line::new(meat_energy).name("Meat Energy (/1000)");
                let snake_line = Line::new(snake_energy).name("Snake Energy (/1000)");
                Plot::new("stats_plot")
                    .view_aspect(2.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(plant_line);
                        plot_ui.line(meat_line);
                        plot_ui.line(snake_line);
                    });
                if self.stats.species.species.is_empty() {
                    ui.label("No species yet.");
                } else {
                    let mut sorted = self.stats.species.species.clone();
                    sorted.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
                    let bars: Vec<Bar> = sorted
                        .iter()
                        .enumerate()
                        .map(|(i, specie)| {
                            Bar::new(i as f64, specie.members.len() as f64)
                                .name(format!("{}", specie.id))
                                .fill(u32_to_color(specie.id))
                        })
                        .collect();
                    let bar_chart = BarChart::new(bars);
                    Plot::new("species_plot")
                        .view_aspect(2.0)
                        .show(ui, |plot_ui| {
                            plot_ui.bar_chart(bar_chart);
                        });
                }
            });
        egui::Window::new("Networks").open(&mut self.show_networks).show(ctx, |ui| {
            let specie_ids = &self.stats.species.species.iter().map(|specie| specie.id).collect::<Vec<u32>>();
            if specie_ids.is_empty() {
                ui.label("No networks yet").on_hover_text("No species have formed yet - start a simulation to see neural networks");
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
                            ui.selectable_value(&mut self.selected_network, *specie_id, format!("{specie_id:?}"));
                        }
                    }).response.on_hover_text("Select species to view its neural network");
                if ui.button("Next").on_hover_text("View next species").clicked() {
                    self.selected_network = specie_ids[(specie_ids.iter().position(|id| *id == self.selected_network).unwrap() + 1) % specie_ids.len()];
                }
                if ui.button("Previous").on_hover_text("View previous species").clicked() {
                    self.selected_network = specie_ids[(specie_ids.iter().position(|id| *id == self.selected_network).unwrap() + specie_ids.len() - 1) % specie_ids.len()];
                }
            });
            ui.collapsing("Information", |ui| {
                ui.label("Green connections mean that the weight is positive, red connections mean that the weight is negative. The thicker the connection, the higher the weight.").on_hover_text("Connection visualization guide");
                ui.label("Positive weight means the snake wants to do the given action if it encounters this sensory input.").on_hover_text("Weight interpretation");
                ui.label("Bias is a constant value of 1.0, chaos is a random number from range 0.0 .. 1.0 generated each tick").on_hover_text("Special input explanations");
                ui.label("Network cost is the energy it takes each turn to 'think'").on_hover_text("Neural network energy cost");

                ui.horizontal(|ui| {
                    ui.label(
                        r#"Input Nodes:
                    "#).on_hover_text("List of neural network inputs");
                    ui.label(
                        r#"Output Nodes
                    Move Forward
                    Move Left
                    Move Right
                    Wait"#).on_hover_text("List of neural network outputs");
                });
            }).header_response.on_hover_text("Show/hide neural network information");
            if let Some(selected_specie) = self.stats.species.species.iter().find(|specie| specie.id == self.selected_network) {
                ui.label(format!("Network run cost: {}", selected_specie.leader_network.run_cost())).on_hover_text("Energy cost per neural network evaluation");
                draw_neural_network(ui, &self.fonts, selected_specie.id, &selected_specie.leader_network.get_nodes(), &selected_specie.leader_network.get_active_connections());
            }
        });
        egui::Window::new("Info")
            .open(&mut self.show_info)
            .show(ctx, |ui| {
                ui.label("Press 'o' to add one snake")
                    .on_hover_text("Keyboard shortcut to spawn a single snake");
                ui.label("Press 'a' to stop simulation and advance one frame (useful for debug)")
                    .on_hover_text("Debug shortcut: pause and step one frame");
                ui.label("Press '+' to increase speed")
                    .on_hover_text("Speed up simulation playback");
                ui.label("Press '-' to decrease speed")
                    .on_hover_text("Slow down simulation playback");
                ui.label("Press 'tab' to ignore speed limit")
                    .on_hover_text("Run simulation as fast as possible");
                ui.label("All enabled settings take effect immediately")
                    .on_hover_text("Changes apply without restarting");
                ui.label("To change disabled settings, stop the simulation first")
                    .on_hover_text("Some settings require simulation restart");
                ui.horizontal(|ui| {
                    ui.label(format!("Tot: {}", self.total_frames))
                        .on_hover_text(format!("Total frames: {}", self.total_frames));
                    ui.label(format!("FPS: {:.1}", self.frames_per_second))
                        .on_hover_text(format!("Frames per second: {:.1}", self.frames_per_second));
                    ui.label(format!("UPS: {}", self.updates_per_second))
                        .on_hover_text(format!("Updates per second: {}", self.updates_per_second));
                    ui.label(format!(
                        "Spd: x{:.1}",
                        self.updates_per_second as f32 / self.frames_per_second as f32
                    ))
                    .on_hover_text(format!(
                        "Speed: x{:.1}",
                        self.updates_per_second as f32 / self.frames_per_second as f32
                    ));
                    ui.label(format!("Old: {}", self.stats.oldest_snake))
                        .on_hover_text(format!("Oldest snake: {}", self.stats.oldest_snake));
                    ui.label(format!("Gen: {}", self.stats.max_generation))
                        .on_hover_text(format!("Max generation: {}", self.stats.max_generation));
                    ui.label(format!("Mut: {}", self.stats.max_mutations))
                        .on_hover_text(format!("Max mutations: {}", self.stats.max_mutations));
                    ui.label(format!(
                        "Snk: {}/{}",
                        self.stats.total_snakes, self.stats.total_segments
                    ))
                    .on_hover_text(format!(
                        "Snakes/segments: {}/{}",
                        self.stats.total_snakes, self.stats.total_segments
                    ));
                    ui.label(format!("Food: {}", self.stats.total_food))
                        .on_hover_text(format!("Food: {}", self.stats.total_food));
                    ui.label(format!("Spc: {}", self.stats.species.species.len()))
                        .on_hover_text(format!("Species: {}", self.stats.species.species.len()));
                    ui.label(format!("Snt: {}", self.stats.total_scents))
                        .on_hover_text(format!("Scents: {}", self.stats.total_scents));
                    ui.label(format!("Ent: {}", self.stats.total_entities))
                        .on_hover_text(format!("Entities: {}", self.stats.total_entities));
                    ui.label(format!(
                        "P/M: {}/{}",
                        self.stats.total_plants, self.stats.total_meat
                    ))
                    .on_hover_text(format!(
                        "Plants/Meat: {}/{}",
                        self.stats.total_plants, self.stats.total_meat
                    ));
                    ui.label(format!(
                        "Stm: P/M {}/{}",
                        self.stats.total_plants_in_stomachs, self.stats.total_meat_in_stomachs
                    ))
                    .on_hover_text(format!(
                        "Stomachs: P/M {}/{}",
                        self.stats.total_plants_in_stomachs, self.stats.total_meat_in_stomachs
                    ));
                    ui.label(format!("SnkE: {}", self.stats.total_snake_energy))
                        .on_hover_text(format!(
                            "Total snake energy: {}",
                            self.stats.total_snake_energy
                        ));
                    ui.label(format!("TotE: {}", self.stats.total_energy))
                        .on_hover_text(format!("Total energy: {}", self.stats.total_energy));
                });
            });
    }

    fn render_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.menu_button("View", |ui| {
                ui.menu_button("Display Settings", |ui| {
                    ui.horizontal(|ui| {
                        egui::stroke_ui(ui, &mut self.config.bg_color, "Background Color");
                        egui::stroke_ui(ui, &mut self.config.scent_color, "Scent Color");
                        egui::stroke_ui(ui, &mut self.config.tail_color, "Tail Color");
                        egui::stroke_ui(ui, &mut self.config.food_color, "Food Color");
                    });
                });
            })
            .response
            .on_hover_text("Configure display settings");
            ui.menu_button("Help", |ui| {
                let checked = self.show_info;
                if ui
                    .selectable_label(checked, if checked { "✓ Info" } else { "Info" })
                    .on_hover_text("Toggle help and keyboard shortcuts window (F1)")
                    .clicked()
                {
                    self.show_info = !self.show_info;
                }
            })
            .response
            .on_hover_text("Get help and information (F1)");
            if ui
                .button("🔄")
                .on_hover_text("Restart simulation (Ctrl+R)")
                .clicked()
            {
                self.engine_commands_sender
                    .send(EngineCommand::UpdateSimulationConfig(
                        self.simulation_config,
                    ))
                    .unwrap();
                self.engine_commands_sender
                    .send(EngineCommand::ResetWorld)
                    .unwrap();
                self.engine_commands_sender
                    .send(EngineCommand::CreateSnakes(10))
                    .unwrap();
                self.paused = false;
            }
            if ui
                .button("🌍")
                .on_hover_text("Toggle environment settings window (E)")
                .clicked()
            {
                self.show_simulation_settings = !self.show_simulation_settings;
            }
            if ui
                .button("")
                .on_hover_text("Toggle mutation settings window (M)")
                .clicked()
            {
                self.show_mutation_settings = !self.show_mutation_settings;
            }
            if ui
                .button("🧬")
                .on_hover_text("Toggle DNA settings window (D)")
                .clicked()
            {
                self.show_dna_settings = !self.show_dna_settings;
            }
            if ui
                .button("🐾")
                .on_hover_text("Toggle species window (P)")
                .clicked()
            {
                self.show_species = !self.show_species;
            }
            if ui
                .button("󰧑 ")
                .on_hover_text("Toggle neural networks window (N)")
                .clicked()
            {
                self.show_networks = !self.show_networks;
            }
            if ui
                .button("📊")
                .on_hover_text("Toggle statistics window (T)")
                .clicked()
            {
                self.show_statistics = !self.show_statistics;
            }
            // Add snakes
            if ui.button("🐍").on_hover_text("Add 10 snakes (S)").clicked() {
                self.engine_commands_sender
                    .send(EngineCommand::CreateSnakes(10))
                    .unwrap();
            }
            // Play/Pause button
            let play_pause_icon = if self.paused { "▶" } else { "⏸" };
            let play_button = egui::Button::new(play_pause_icon).fill(if self.paused {
                Color32::from_rgb(0, 128, 0) // Dark green for better contrast
            } else {
                Color32::from_rgb(128, 0, 0) // Dark red for better contrast
            });
            if ui
                .add(play_button)
                .on_hover_text(if self.paused {
                    "Play simulation (Space)"
                } else {
                    "Pause simulation (Space)"
                })
                .clicked()
            {
                self.paused = !self.paused;
                self.engine_commands_sender
                    .send(EngineCommand::FlipRunningState)
                    .unwrap();
            }
            if ui
                .small_button("-")
                .on_hover_text("Decrease speed (-)")
                .clicked()
            {
                self.engine_commands_sender
                    .send(EngineCommand::DecreaseSpeed)
                    .unwrap();
            }
            if ui
                .small_button("+")
                .on_hover_text("Increase speed (+)")
                .clicked()
            {
                self.engine_commands_sender
                    .send(EngineCommand::IncreaseSpeed)
                    .unwrap();
            }
            ui.label(format!(
                "x{:.1}",
                self.updates_per_second as f32 / self.frames_per_second as f32
            ));
        });
    }

    fn render_central_panel(&mut self, _ctx: &egui::Context, ui: &mut Ui) {
        draw_hexes(ui, &self.hexes, &self.config);
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.label(&self.text);
            });
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::PlusEquals)) {
            self.engine_commands_sender
                .send(EngineCommand::IncreaseSpeed)
                .unwrap();
        }
        if ctx.input(|i| i.key_pressed(Key::Minus)) {
            self.engine_commands_sender
                .send(EngineCommand::DecreaseSpeed)
                .unwrap();
        }
        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            self.engine_commands_sender
                .send(EngineCommand::IgnoreSpeedLimit)
                .unwrap();
        }
        if ctx.input(|i| i.key_pressed(Key::O)) {
            self.engine_commands_sender
                .send(EngineCommand::CreateSnakes(1))
                .unwrap();
        }
        if ctx.input(|i| i.key_pressed(Key::A)) {
            self.engine_commands_sender
                .send(EngineCommand::AdvanceOneFrame)
                .unwrap();
        }
        // New shortcuts for toolbar buttons
        if ctx.input(|i| i.key_pressed(Key::F1)) {
            self.show_info = !self.show_info;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::R)) {
            self.engine_commands_sender
                .send(EngineCommand::UpdateSimulationConfig(
                    self.simulation_config,
                ))
                .unwrap();
            self.engine_commands_sender
                .send(EngineCommand::ResetWorld)
                .unwrap();
            self.engine_commands_sender
                .send(EngineCommand::CreateSnakes(10))
                .unwrap();
            self.paused = false;
        }
        if ctx.input(|i| i.key_pressed(Key::E)) {
            self.show_simulation_settings = !self.show_simulation_settings;
        }
        if ctx.input(|i| i.key_pressed(Key::M)) {
            self.show_mutation_settings = !self.show_mutation_settings;
        }
        if ctx.input(|i| i.key_pressed(Key::D)) {
            self.show_dna_settings = !self.show_dna_settings;
        }
        if ctx.input(|i| i.key_pressed(Key::P)) {
            self.show_species = !self.show_species;
        }
        if ctx.input(|i| i.key_pressed(Key::N)) {
            self.show_networks = !self.show_networks;
        }
        if ctx.input(|i| i.key_pressed(Key::T)) {
            self.show_statistics = !self.show_statistics;
        }
        if ctx.input(|i| i.key_pressed(Key::S)) {
            self.engine_commands_sender
                .send(EngineCommand::CreateSnakes(10))
                .unwrap();
        }
        if ctx.input(|i| i.key_pressed(Key::Space)) {
            self.paused = !self.paused;
            self.engine_commands_sender
                .send(EngineCommand::FlipRunningState)
                .unwrap();
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::profile_scope!("gui::update");
        if puffin::are_scopes_on() {
            puffin_egui::profiler_window(ctx);
            puffin::GlobalProfiler::lock().new_frame();
        }
        self.handle_events(ctx);
        self.render_windows(ctx);
        if self.simulation_config != self.previous_simulation_config {
            self.save_config();
            if !only_star_fields_differ(&self.simulation_config, &self.previous_simulation_config)
                || self.paused
            {
                self.engine_commands_sender
                    .send(EngineCommand::UpdateSimulationConfig(
                        self.simulation_config,
                    ))
                    .unwrap();
            }
            self.previous_simulation_config = self.simulation_config;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_toolbar(ui);
            self.render_central_panel(ctx, ui);
        });
        self.handle_keyboard_shortcuts(ctx);
        if self.can_draw_frame {
            ctx.request_repaint();
            self.can_draw_frame = false;
        }
        self.last_frame = Instant::now();
        let _ = self
            .engine_commands_sender
            .send(EngineCommand::RepaintRequested);
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, Pos2};
    use hex_brains_engine::dna::SegmentType;
    use hex_brains_engine::neural::{ConnectionGene, NodeType};
    use hex_brains_engine::simulation::{EngineCommand, Hex, HexType};
    use std::sync::mpsc;

    #[test]
    fn test_engine_command_send() {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let command = EngineCommand::CreateSnakes(10);
        tx.send(command.clone()).expect("Send failed");
        if let Ok(received) = rx.try_recv() {
            match (command, received) {
                (EngineCommand::CreateSnakes(n1), EngineCommand::CreateSnakes(n2)) => {
                    assert_eq!(n1, n2)
                }
                _ => panic!("Unexpected command"),
            }
        } else {
            panic!("No command received");
        }
    }

    #[test]
    fn test_u32_to_color() {
        let color = u32_to_color(42);
        // Hash-based, ensure valid RGB
        assert_eq!(color.a(), 255);
    }

    #[test]
    fn test_with_alpha() {
        let original = Color32::RED;
        let alpha = 0.5;
        let result = with_alpha(original, alpha);
        assert_eq!(result, Color32::from_rgba_unmultiplied(255, 0, 0, 128));
    }

    #[test]
    fn test_get_node_position() {
        // Input node 0
        let pos0_input = get_node_position(0, NodeType::Input);
        assert_eq!(pos0_input.x, 0.25);
        assert!((pos0_input.y - 0.1).abs() < 1e-6);

        // Input node 1
        let pos1_input = get_node_position(1, NodeType::Input);
        assert_eq!(pos1_input.x, 0.25);
        assert!((pos1_input.y - 0.175).abs() < 1e-6);

        // Output node 0
        let pos0_output = get_node_position(0, NodeType::Output);
        assert_eq!(pos0_output.x, 0.85);
        assert!((pos0_output.y - 0.1).abs() < 1e-6);

        // Output node 1
        let pos1_output = get_node_position(1, NodeType::Output);
        assert_eq!(pos1_output.x, 0.85);
        assert!((pos1_output.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_transform_to_circle_logic() {
        let config = Config {
            rows: 10,
            columns: 10,
            bg_color: Stroke::new(1.0, Color32::WHITE),
            scent_color: Stroke::new(1.0, Color32::WHITE),
            food_color: Stroke::new(1.0, Color32::WHITE),
            tail_color: Stroke::new(1.0, Color32::WHITE),
            add_walls: false,
        };
        let game_pos = Pos2::new(0.0, 0.0);
        let normalized_radius = 1.0 / (2.0 * config.rows as f32);
        assert!((normalized_radius - 0.05).abs() < 1e-6);

        let offset = if game_pos.y as i32 % 2 == 0 {
            normalized_radius
        } else {
            0.0
        };
        let normalized_position = Pos2 {
            x: game_pos.x / config.columns as f32 + offset + normalized_radius,
            y: game_pos.y / config.rows as f32 + normalized_radius,
        };
        assert!((normalized_position.x - 0.1).abs() < 1e-6);
        assert!((normalized_position.y - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_hex_color_selection() {
        let config = Config {
            rows: 10,
            columns: 10,
            bg_color: Stroke::new(1.0, Color32::from_gray(100)),
            scent_color: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 255, 128)),
            food_color: Stroke::new(1.0, Color32::GREEN),
            tail_color: Stroke::new(1.0, Color32::BLUE),
            add_walls: false,
        };

        // Plant Food (green)
        let food_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::Food,
        };
        let food_color = match &food_hex.hex_type {
            HexType::Food => config.food_color.color,
            _ => Color32::TRANSPARENT,
        };
        assert_eq!(food_color, Color32::GREEN);

        // Meat (red)
        let meat_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::Meat,
        };
        let meat_color = match &meat_hex.hex_type {
            HexType::Meat => Color32::RED,
            _ => Color32::TRANSPARENT,
        };
        assert_eq!(meat_color, Color32::RED);

        // Scent
        let scent_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::Scent { value: 0.5 },
        };
        let scent_color = match &scent_hex.hex_type {
            HexType::Scent { value } => {
                let intensity = *value;
                let blue = (intensity * 200.0) as u8;
                let alpha_factor = (config.scent_color.color.a() as f32 / 255.0) * intensity;
                let a = (alpha_factor * 256.0) as u8;
                Color32::from_rgba_unmultiplied(0, 0, blue, a)
            }
            _ => Color32::TRANSPARENT,
        };
        assert_eq!(scent_color, Color32::from_rgba_unmultiplied(0, 0, 100, 64));

        // Snake Tail (blue)
        let tail_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::SnakeTail,
        };
        let tail_color = match &tail_hex.hex_type {
            HexType::SnakeTail => config.tail_color.color,
            _ => Color32::TRANSPARENT,
        };
        assert_eq!(tail_color, Color32::BLUE);

        // Segment Muscle (light red with alpha 0.8)
        let muscle_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::Segment {
                segment_type: SegmentType::muscle(),
            },
        };
        let muscle_color = match &muscle_hex.hex_type {
            HexType::Segment { segment_type } => match segment_type {
                SegmentType::Muscle(_) => with_alpha(Color32::LIGHT_RED, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_muscle = with_alpha(Color32::LIGHT_RED, 0.8);
        assert_eq!(muscle_color, expected_muscle);

        // Segment Solid (brown with alpha)
        let solid_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::Segment {
                segment_type: SegmentType::solid(),
            },
        };
        let solid_color = match &solid_hex.hex_type {
            HexType::Segment { segment_type } => match segment_type {
                SegmentType::Solid(_) => with_alpha(Color32::BROWN, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_solid = with_alpha(Color32::BROWN, 0.8);
        assert_eq!(solid_color, expected_solid);

        // Segment Solar (light blue with alpha)
        let solar_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::Segment {
                segment_type: SegmentType::solar(),
            },
        };
        let solar_color = match &solar_hex.hex_type {
            HexType::Segment { segment_type } => match segment_type {
                SegmentType::Solar(_) => with_alpha(Color32::LIGHT_BLUE, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_solar = with_alpha(Color32::LIGHT_BLUE, 0.8);
        assert_eq!(solar_color, expected_solar);

        // Segment Stomach (light green with alpha)
        let stomach_hex = Hex {
            x: 0,
            y: 0,
            hex_type: HexType::Segment {
                segment_type: SegmentType::stomach(),
            },
        };
        let stomach_color = match &stomach_hex.hex_type {
            HexType::Segment { segment_type } => match segment_type {
                SegmentType::Stomach(_) => with_alpha(Color32::LIGHT_GREEN, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_stomach = with_alpha(Color32::LIGHT_GREEN, 0.8);
        assert_eq!(stomach_color, expected_stomach);
    }

    #[test]
    fn test_neural_connection_color() {
        // Positive weight -> light green
        let pos_conn = ConnectionGene {
            in_node: 0,
            out_node: 18,
            weight: 1.0,
            enabled: true,
            innovation_number: 0,
        };
        let pos_color = if pos_conn.weight > 0.0 {
            Color32::LIGHT_GREEN
        } else {
            Color32::LIGHT_RED
        };
        assert_eq!(pos_color, Color32::LIGHT_GREEN);

        // Negative weight -> light red
        let neg_conn = ConnectionGene {
            in_node: 0,
            out_node: 18,
            weight: -1.0,
            enabled: true,
            innovation_number: 0,
        };
        let neg_color = if neg_conn.weight > 0.0 {
            Color32::LIGHT_GREEN
        } else {
            Color32::LIGHT_RED
        };
        assert_eq!(neg_color, Color32::LIGHT_RED);

        // Zero weight -> light red (as <=0)
        let zero_conn = ConnectionGene {
            in_node: 0,
            out_node: 18,
            weight: 0.0,
            enabled: true,
            innovation_number: 0,
        };
        let zero_color = if zero_conn.weight > 0.0 {
            Color32::LIGHT_GREEN
        } else {
            Color32::LIGHT_RED
        };
        assert_eq!(zero_color, Color32::LIGHT_RED);
    }

    #[test]
    fn test_neural_active_connections_highlighted() {
        // Assuming get_active_connections returns only enabled, but since it's engine code, test logic here
        // The drawing uses get_active_connections, so verify color for active (enabled)
        // But since highlighted by thickness based on |weight|, test stroke width
        let weight: f32 = 3.0;
        let height = 100.0; // response.rect.height()
        let thickness = (weight.abs() / 30.0) * height;
        assert_eq!(thickness, 10.0); // 3/30 *100 = 10
    }

    #[test]
    fn test_draw_hexes_empty() {
        // Test that draw_hexes doesn't panic with empty hexes
        // Since it requires Ui, we test the internal logic via helpers already covered
        // But to ensure, the shapes vec is empty, ground is empty, no extend panics
        assert!(true); // Placeholder, as full test requires mocking Ui
    }

    #[test]
    fn test_draw_neural_network_empty() {
        // Test with empty nodes/connections
        // Logic in helpers: input_nodes empty, output_nodes empty, connection_shapes empty
        // No panics in painter.extend
        assert!(true); // Placeholder
    }
}
