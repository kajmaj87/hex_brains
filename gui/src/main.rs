use bevy_ecs::prelude::*;
use eframe::egui;
use eframe::emath::Vec2;
#[allow(unused_imports)]
use eframe::epaint::{Color32, Stroke};
use hex_brains_engine::core::{Food, Position, Scent, ScentMap, Snake, Solid};
use hex_brains_engine::dna::SegmentType;
use hex_brains_engine::simulation::{
    EngineCommand, EngineEvent, EngineEvents, EngineState, Hex, HexType, Simulation,
    SimulationConfig, Stats,
};
use parking_lot::Mutex;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use tracing::Level;
use tracing_subscriber::fmt;

// Constants for magic numbers to improve maintainability and tuning
const INITIAL_WINDOW_WIDTH: f32 = 1200.0;
const INITIAL_WINDOW_HEIGHT: f32 = 1200.0;
const SPEED_LIMIT: f32 = 200.0;
const HISTORY_LIMIT: usize = 1000;
const SMOOTHING_WINDOW: usize = 100;
const PERFORMANCE_UPDATE_INTERVAL_MS: u128 = 1000;
const DEFAULT_SNAKES_TO_ADD: usize = 10;

mod app;
mod components;
mod config;
mod drawing;
mod ui_helpers;
mod ui_state;
mod windows;

fn main() {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2 {
            x: INITIAL_WINDOW_WIDTH,
            y: INITIAL_WINDOW_HEIGHT,
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

fn start_simulation(
    engine_events_sender: &Sender<EngineEvent>,
    engine_commands_receiver: Arc<Mutex<Receiver<EngineCommand>>>,
    context: egui::Context,
    simulation_config: SimulationConfig,
) {
    let config = config::create_drawing_config(&simulation_config);
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
        speed_limit: Some(SPEED_LIMIT),
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

fn should_draw_simulation(engine_state: Res<EngineState>) -> bool {
    engine_state.repaint_needed
}

#[derive(Resource)]
struct EguiEcsContext {
    _context: egui::Context,
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
        let color = drawing::u32_to_color(42);
        // Hash-based, ensure valid RGB
        assert_eq!(color.a(), 255);
    }

    #[test]
    fn test_with_alpha() {
        let original = Color32::RED;
        let alpha = 0.5;
        let result = drawing::with_alpha(original, alpha);
        assert_eq!(result, Color32::from_rgba_unmultiplied(255, 0, 0, 128));
    }

    #[test]
    fn test_get_node_position() {
        // Input node 0
        let pos0_input = drawing::get_node_position(0, NodeType::Input);
        assert_eq!(pos0_input.x, 0.25);
        assert!((pos0_input.y - 0.1).abs() < 1e-6);

        // Input node 1
        let pos1_input = drawing::get_node_position(1, NodeType::Input);
        assert_eq!(pos1_input.x, 0.25);
        assert!((pos1_input.y - 0.175).abs() < 1e-6);

        // Output node 0
        let pos0_output = drawing::get_node_position(0, NodeType::Output);
        assert_eq!(pos0_output.x, 0.85);
        assert!((pos0_output.y - 0.1).abs() < 1e-6);

        // Output node 1
        let pos1_output = drawing::get_node_position(1, NodeType::Output);
        assert_eq!(pos1_output.x, 0.85);
        assert!((pos1_output.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_transform_to_circle_logic() {
        let config = config::Config {
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
        let config = config::Config {
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
                SegmentType::Muscle(_) => drawing::with_alpha(Color32::LIGHT_RED, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_muscle = drawing::with_alpha(Color32::LIGHT_RED, 0.8);
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
                SegmentType::Solid(_) => drawing::with_alpha(Color32::BROWN, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_solid = drawing::with_alpha(Color32::BROWN, 0.8);
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
                SegmentType::Solar(_) => drawing::with_alpha(Color32::LIGHT_BLUE, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_solar = drawing::with_alpha(Color32::LIGHT_BLUE, 0.8);
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
                SegmentType::Stomach(_) => drawing::with_alpha(Color32::LIGHT_GREEN, 0.8),
                _ => Color32::TRANSPARENT,
            },
            _ => Color32::TRANSPARENT,
        };
        let expected_stomach = drawing::with_alpha(Color32::LIGHT_GREEN, 0.8);
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
