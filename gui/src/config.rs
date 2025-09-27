use bevy_ecs::prelude::Resource;
use eframe::epaint::{Color32, Stroke};
use hex_brains_engine::simulation::{MutationConfig, SimulationConfig};
use std::fs;

/// Configuration for drawing the simulation grid
#[derive(Resource, Clone, Copy)]
pub struct Config {
    pub rows: usize,
    pub columns: usize,
    pub bg_color: Stroke,
    pub scent_color: Stroke,
    pub food_color: Stroke,
    pub tail_color: Stroke,
    pub add_walls: bool,
}

/// Load simulation configuration from config.toml, with defaults if not found or invalid
pub fn load_config() -> SimulationConfig {
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

/// Save simulation configuration to config.toml
pub fn save_config(config: &SimulationConfig) {
    if let Ok(toml_str) = toml::to_string(config) {
        let _ = fs::write("config.toml", toml_str);
    }
}

/// Check if only star fields (rows, columns, add_walls) differ between configs
pub fn only_star_fields_differ(a: &SimulationConfig, b: &SimulationConfig) -> bool {
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

/// Create a Config from SimulationConfig for drawing
pub fn create_drawing_config(simulation_config: &SimulationConfig) -> Config {
    Config {
        rows: simulation_config.rows,
        columns: simulation_config.columns,
        bg_color: Stroke::new(1.0, Color32::LIGHT_GREEN),
        scent_color: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xAD, 0xD8, 0xE6, 50)),
        tail_color: Stroke::new(1.0, Color32::LIGHT_RED),
        food_color: Stroke::new(1.0, Color32::YELLOW),
        add_walls: simulation_config.add_walls,
    }
}
