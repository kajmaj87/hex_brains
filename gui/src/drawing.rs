use eframe::emath::{Pos2, Rect, Vec2};
use eframe::epaint::{Color32, Fonts};
use eframe::{egui, emath};
use egui::epaint::CircleShape;
use egui::Shape::Circle;
use egui::{Align2, FontFamily, FontId, Frame, Response, Sense, Shape, Stroke, Ui};
use hex_brains_engine::dna::SegmentType;
use hex_brains_engine::neural;
use hex_brains_engine::neural::{ConnectionGene, NodeGene, NodeType};
use hex_brains_engine::simulation::{Hex, HexType};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::config::Config;

pub fn draw_neural_network(
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

pub fn get_node_position(index: usize, node_type: NodeType) -> Pos2 {
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

pub fn draw_hexes(ui: &mut Ui, hexes: &[Hex], config: &Config) {
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

pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (alpha * 256.0) as u8)
}

pub fn transform_to_circle(
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

pub fn u32_to_color(u: u32) -> Color32 {
    let mut hasher = DefaultHasher::new();
    u.hash(&mut hasher);
    let hash = hasher.finish();

    let r = (hash >> 16) as u8;
    let g = (hash >> 8) as u8;
    let b = hash as u8;

    Color32::from_rgb(r, g, b)
}
