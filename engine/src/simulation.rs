use crate::core::die_from_collisions;
use crate::core::SolidsMap;
use crate::core::{
    add_scents, assign_solid_positions, destroy_old_food, diffuse_scents, disperse_scents, Solid,
};
use crate::core::{
    assign_missing_segments, assign_species, calculate_stats, create_food, create_snake, eat_food,
    grow, increase_age, movement, split, starve, think, update_positions, FoodMap, Position,
    RandomNeuralBrain, Species,
};
use crate::core::{
    assign_segment_positions, despawn_food, incease_move_potential, process_food, Food, Map2d,
    Map3d, ScentMap, SegmentMap,
};
use crate::dna::{Dna, SegmentType};
use crate::neural::InnovationTracker;
use bevy_ecs::prelude::{IntoSystemConfigs, Res, ResMut, Resource, Schedule, World};
use tinyrand::{RandRange, Wyrand};

#[derive(Resource)]
pub struct RngResource {
    pub rng: Wyrand,
}
use parking_lot::Mutex;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

pub struct Simulation {
    first_schedule: Schedule,
    core_schedule: Schedule,
    secondary_schedule: Schedule,
    gui_schedule: Schedule,
    pub world: World,
    pub name: String,
    engine_events: Sender<EngineEvent>,
    // only the main simulation may receive commands
    engine_commands: Option<Arc<Mutex<Receiver<EngineCommand>>>>,
    has_gui: bool,
}

#[derive(Debug, Clone)]
pub struct Hex {
    pub x: usize,
    pub y: usize,
    pub hex_type: HexType,
}

#[derive(Debug, Clone)]
pub enum HexType {
    Food,
    SnakeHead { specie: u32 },
    SnakeTail,
    Scent { value: f32 },
    Segment { segment_type: SegmentType },
    Meat,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct Stats {
    pub total_snakes: usize,
    pub total_food: usize,
    pub oldest_snake: u32,
    pub food: usize,
    pub total_segments: usize,
    pub max_generation: u32,
    pub max_mutations: u32,
    pub species: Species,
    pub total_entities: usize,
    pub total_scents: usize,
    pub total_snake_energy: f32,
    pub total_plants_in_stomachs: f32,
    pub total_meat_in_stomachs: f32,
    pub total_plants: f32,
    pub total_meat: f32,
    pub total_energy: f32,
    pub total_plant_energy: f32,
    pub total_meat_energy: f32,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    SimulationFinished {
        steps: u32,
        name: String,
        duration: u128,
    },
    DrawData {
        hexes: Vec<Hex>,
        stats: Stats,
        frames: u32,
    },
    FrameDrawn {
        updates_left: f32,
        updates_done: u32,
    },
}

#[derive(Debug, Resource, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MutationConfig {
    pub scent_sensing_enabled: bool,
    pub plant_vision_enabled: bool,
    pub meat_vision_enabled: bool,
    pub obstacle_vision_enabled: bool,
    pub chaos_input_enabled: bool,
    pub plant_vision_front_range: u32,
    pub plant_vision_left_range: u32,
    pub plant_vision_right_range: u32,
    pub meat_vision_front_range: u32,
    pub meat_vision_left_range: u32,
    pub meat_vision_right_range: u32,
    pub obstacle_vision_front_range: u32,
    pub obstacle_vision_left_range: u32,
    pub obstacle_vision_right_range: u32,
    pub weight_perturbation_range: f32,
    pub weight_perturbation_chance: f64,
    pub perturb_disabled_connections: bool,
    pub connection_flip_chance: f64,
    pub dna_mutation_chance: f64,
    pub weight_reset_chance: f64,
    pub weight_reset_range: f32,
    pub disable_muscle: bool,
    pub disable_solid: bool,
    pub disable_solar: bool,
    pub disable_stomach: bool,
}

impl Default for MutationConfig {
    fn default() -> Self {
        MutationConfig {
            scent_sensing_enabled: true,
            plant_vision_enabled: true,
            obstacle_vision_enabled: true,
            chaos_input_enabled: true,
            plant_vision_front_range: 5,
            plant_vision_left_range: 3,
            plant_vision_right_range: 3,
            obstacle_vision_front_range: 5,
            obstacle_vision_left_range: 3,
            obstacle_vision_right_range: 3,
            weight_perturbation_range: 0.1,
            weight_perturbation_chance: 0.75,
            perturb_disabled_connections: false,
            connection_flip_chance: 0.3,
            dna_mutation_chance: 0.5,
            weight_reset_chance: 0.1,
            weight_reset_range: 1.0,
            meat_vision_front_range: 5,
            meat_vision_left_range: 3,
            meat_vision_right_range: 3,
            meat_vision_enabled: true,
            disable_muscle: false,
            disable_solid: false,
            disable_solar: false,
            disable_stomach: false,
        }
    }
}

#[derive(Default)]
pub struct MutationConfigBuilder {
    scent_sensing_enabled: Option<bool>,
    plant_vision_enabled: Option<bool>,
    meat_vision_enabled: Option<bool>,
    obstacle_vision_enabled: Option<bool>,
    chaos_input_enabled: Option<bool>,
    plant_vision_front_range: Option<u32>,
    plant_vision_left_range: Option<u32>,
    plant_vision_right_range: Option<u32>,
    meat_vision_front_range: Option<u32>,
    meat_vision_left_range: Option<u32>,
    meat_vision_right_range: Option<u32>,
    obstacle_vision_front_range: Option<u32>,
    obstacle_vision_left_range: Option<u32>,
    obstacle_vision_right_range: Option<u32>,
    weight_perturbation_range: Option<f32>,
    weight_perturbation_chance: Option<f64>,
    perturb_disabled_connections: Option<bool>,
    connection_flip_chance: Option<f64>,
    dna_mutation_chance: Option<f64>,
    weight_reset_chance: Option<f64>,
    weight_reset_range: Option<f32>,
    disable_muscle: Option<bool>,
    disable_solid: Option<bool>,
    disable_solar: Option<bool>,
    disable_stomach: Option<bool>,
}

impl MutationConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scent_sensing_enabled(mut self, v: bool) -> Self {
        self.scent_sensing_enabled = Some(v);
        self
    }

    pub fn plant_vision_enabled(mut self, v: bool) -> Self {
        self.plant_vision_enabled = Some(v);
        self
    }

    pub fn meat_vision_enabled(mut self, v: bool) -> Self {
        self.meat_vision_enabled = Some(v);
        self
    }

    pub fn obstacle_vision_enabled(mut self, v: bool) -> Self {
        self.obstacle_vision_enabled = Some(v);
        self
    }

    pub fn chaos_input_enabled(mut self, v: bool) -> Self {
        self.chaos_input_enabled = Some(v);
        self
    }

    pub fn plant_vision_front_range(mut self, v: u32) -> Self {
        self.plant_vision_front_range = Some(v);
        self
    }

    pub fn plant_vision_left_range(mut self, v: u32) -> Self {
        self.plant_vision_left_range = Some(v);
        self
    }

    pub fn plant_vision_right_range(mut self, v: u32) -> Self {
        self.plant_vision_right_range = Some(v);
        self
    }

    pub fn meat_vision_front_range(mut self, v: u32) -> Self {
        self.meat_vision_front_range = Some(v);
        self
    }

    pub fn meat_vision_left_range(mut self, v: u32) -> Self {
        self.meat_vision_left_range = Some(v);
        self
    }

    pub fn meat_vision_right_range(mut self, v: u32) -> Self {
        self.meat_vision_right_range = Some(v);
        self
    }

    pub fn obstacle_vision_front_range(mut self, v: u32) -> Self {
        self.obstacle_vision_front_range = Some(v);
        self
    }

    pub fn obstacle_vision_left_range(mut self, v: u32) -> Self {
        self.obstacle_vision_left_range = Some(v);
        self
    }

    pub fn obstacle_vision_right_range(mut self, v: u32) -> Self {
        self.obstacle_vision_right_range = Some(v);
        self
    }

    pub fn weight_perturbation_range(mut self, v: f32) -> Self {
        self.weight_perturbation_range = Some(v);
        self
    }

    pub fn weight_perturbation_chance(mut self, v: f64) -> Self {
        self.weight_perturbation_chance = Some(v);
        self
    }

    pub fn perturb_disabled_connections(mut self, v: bool) -> Self {
        self.perturb_disabled_connections = Some(v);
        self
    }

    pub fn connection_flip_chance(mut self, v: f64) -> Self {
        self.connection_flip_chance = Some(v);
        self
    }

    pub fn dna_mutation_chance(mut self, v: f64) -> Self {
        self.dna_mutation_chance = Some(v);
        self
    }

    pub fn weight_reset_chance(mut self, v: f64) -> Self {
        self.weight_reset_chance = Some(v);
        self
    }

    pub fn weight_reset_range(mut self, v: f32) -> Self {
        self.weight_reset_range = Some(v);
        self
    }

    pub fn disable_muscle(mut self, v: bool) -> Self {
        self.disable_muscle = Some(v);
        self
    }

    pub fn disable_solid(mut self, v: bool) -> Self {
        self.disable_solid = Some(v);
        self
    }

    pub fn disable_solar(mut self, v: bool) -> Self {
        self.disable_solar = Some(v);
        self
    }

    pub fn disable_stomach(mut self, v: bool) -> Self {
        self.disable_stomach = Some(v);
        self
    }

    pub fn build(self) -> Result<MutationConfig, String> {
        let scent_sensing_enabled = self.scent_sensing_enabled.unwrap_or(true);
        let plant_vision_enabled = self.plant_vision_enabled.unwrap_or(true);
        let meat_vision_enabled = self.meat_vision_enabled.unwrap_or(true);
        let obstacle_vision_enabled = self.obstacle_vision_enabled.unwrap_or(true);
        let chaos_input_enabled = self.chaos_input_enabled.unwrap_or(true);
        let plant_vision_front_range = self.plant_vision_front_range.unwrap_or(5);
        let plant_vision_left_range = self.plant_vision_left_range.unwrap_or(3);
        let plant_vision_right_range = self.plant_vision_right_range.unwrap_or(3);
        let meat_vision_front_range = self.meat_vision_front_range.unwrap_or(5);
        let meat_vision_left_range = self.meat_vision_left_range.unwrap_or(3);
        let meat_vision_right_range = self.meat_vision_right_range.unwrap_or(3);
        let obstacle_vision_front_range = self.obstacle_vision_front_range.unwrap_or(5);
        let obstacle_vision_left_range = self.obstacle_vision_left_range.unwrap_or(3);
        let obstacle_vision_right_range = self.obstacle_vision_right_range.unwrap_or(3);
        let weight_perturbation_range = self.weight_perturbation_range.unwrap_or(0.1);
        let weight_perturbation_chance = self.weight_perturbation_chance.unwrap_or(0.75);
        let perturb_disabled_connections = self.perturb_disabled_connections.unwrap_or(false);
        let connection_flip_chance = self.connection_flip_chance.unwrap_or(0.3);
        let dna_mutation_chance = self.dna_mutation_chance.unwrap_or(0.5);
        let weight_reset_chance = self.weight_reset_chance.unwrap_or(0.1);
        let weight_reset_range = self.weight_reset_range.unwrap_or(1.0);
        let disable_muscle = self.disable_muscle.unwrap_or(false);
        let disable_solid = self.disable_solid.unwrap_or(false);
        let disable_solar = self.disable_solar.unwrap_or(false);
        let disable_stomach = self.disable_stomach.unwrap_or(false);

        // Validation
        if plant_vision_front_range == 0
            || plant_vision_left_range == 0
            || plant_vision_right_range == 0
            || meat_vision_front_range == 0
            || meat_vision_left_range == 0
            || meat_vision_right_range == 0
            || obstacle_vision_front_range == 0
            || obstacle_vision_left_range == 0
            || obstacle_vision_right_range == 0
        {
            return Err("Vision ranges must be greater than 0".to_string());
        }
        if weight_perturbation_range <= 0.0 {
            return Err("Weight perturbation range must be greater than 0".to_string());
        }
        if !(0.0..=1.0).contains(&weight_perturbation_chance) {
            return Err("Weight perturbation chance must be between 0 and 1".to_string());
        }
        if !(0.0..=1.0).contains(&connection_flip_chance) {
            return Err("Connection flip chance must be between 0 and 1".to_string());
        }
        if !(0.0..=1.0).contains(&dna_mutation_chance) {
            return Err("DNA mutation chance must be between 0 and 1".to_string());
        }
        if !(0.0..=1.0).contains(&weight_reset_chance) {
            return Err("Weight reset chance must be between 0 and 1".to_string());
        }
        if weight_reset_range <= 0.0 {
            return Err("Weight reset range must be greater than 0".to_string());
        }

        Ok(MutationConfig {
            scent_sensing_enabled,
            plant_vision_enabled,
            meat_vision_enabled,
            obstacle_vision_enabled,
            chaos_input_enabled,
            plant_vision_front_range,
            plant_vision_left_range,
            plant_vision_right_range,
            meat_vision_front_range,
            meat_vision_left_range,
            meat_vision_right_range,
            obstacle_vision_front_range,
            obstacle_vision_left_range,
            obstacle_vision_right_range,
            weight_perturbation_range,
            weight_perturbation_chance,
            perturb_disabled_connections,
            connection_flip_chance,
            dna_mutation_chance,
            weight_reset_chance,
            weight_reset_range,
            disable_muscle,
            disable_solid,
            disable_solar,
            disable_stomach,
        })
    }
}

#[derive(Debug, Resource, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SimulationConfig {
    pub rows: usize,
    pub columns: usize,
    pub starting_snakes: usize,
    pub starting_food: usize,
    pub food_per_step: usize,
    pub plant_matter_per_segment: f32,
    pub wait_cost: f32,
    pub move_cost: f32,
    pub new_segment_cost: f32,
    pub size_to_split: usize,
    pub species_threshold: f32,
    pub mutation: MutationConfig,
    pub add_walls: bool,
    pub scent_diffusion_rate: f32,
    pub scent_dispersion_per_step: f32,
    pub create_scents: bool,
    pub snake_max_age: u32,
    pub meat_energy_content: f32,
    pub plant_energy_content: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig {
            rows: 10,
            columns: 10,
            starting_snakes: 0,
            starting_food: 0,
            food_per_step: 1,
            plant_matter_per_segment: 10.0,
            wait_cost: 0.0,
            move_cost: 1.0,
            new_segment_cost: 50.0,
            size_to_split: 2,
            species_threshold: 0.3,
            mutation: MutationConfig::default(),
            add_walls: false,
            scent_diffusion_rate: 0.01,
            scent_dispersion_per_step: 0.01,
            create_scents: false,
            snake_max_age: 10000,
            meat_energy_content: 20.0,
            plant_energy_content: 10.0,
        }
    }
}

#[derive(Default)]
pub struct SimulationConfigBuilder {
    rows: Option<usize>,
    columns: Option<usize>,
    starting_snakes: Option<usize>,
    starting_food: Option<usize>,
    food_per_step: Option<usize>,
    plant_matter_per_segment: Option<f32>,
    wait_cost: Option<f32>,
    move_cost: Option<f32>,
    new_segment_cost: Option<f32>,
    size_to_split: Option<usize>,
    species_threshold: Option<f32>,
    mutation: Option<MutationConfig>,
    add_walls: Option<bool>,
    scent_diffusion_rate: Option<f32>,
    scent_dispersion_per_step: Option<f32>,
    create_scents: Option<bool>,
    snake_max_age: Option<u32>,
    meat_energy_content: Option<f32>,
    plant_energy_content: Option<f32>,
}

impl SimulationConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rows(mut self, v: usize) -> Self {
        self.rows = Some(v);
        self
    }

    pub fn columns(mut self, v: usize) -> Self {
        self.columns = Some(v);
        self
    }

    pub fn starting_snakes(mut self, v: usize) -> Self {
        self.starting_snakes = Some(v);
        self
    }

    pub fn starting_food(mut self, v: usize) -> Self {
        self.starting_food = Some(v);
        self
    }

    pub fn food_per_step(mut self, v: usize) -> Self {
        self.food_per_step = Some(v);
        self
    }

    pub fn plant_matter_per_segment(mut self, v: f32) -> Self {
        self.plant_matter_per_segment = Some(v);
        self
    }

    pub fn wait_cost(mut self, v: f32) -> Self {
        self.wait_cost = Some(v);
        self
    }

    pub fn move_cost(mut self, v: f32) -> Self {
        self.move_cost = Some(v);
        self
    }

    pub fn new_segment_cost(mut self, v: f32) -> Self {
        self.new_segment_cost = Some(v);
        self
    }

    pub fn size_to_split(mut self, v: usize) -> Self {
        self.size_to_split = Some(v);
        self
    }

    pub fn species_threshold(mut self, v: f32) -> Self {
        self.species_threshold = Some(v);
        self
    }

    pub fn mutation(mut self, v: MutationConfig) -> Self {
        self.mutation = Some(v);
        self
    }

    pub fn add_walls(mut self, v: bool) -> Self {
        self.add_walls = Some(v);
        self
    }

    pub fn scent_diffusion_rate(mut self, v: f32) -> Self {
        self.scent_diffusion_rate = Some(v);
        self
    }

    pub fn scent_dispersion_per_step(mut self, v: f32) -> Self {
        self.scent_dispersion_per_step = Some(v);
        self
    }

    pub fn create_scents(mut self, v: bool) -> Self {
        self.create_scents = Some(v);
        self
    }

    pub fn snake_max_age(mut self, v: u32) -> Self {
        self.snake_max_age = Some(v);
        self
    }

    pub fn meat_energy_content(mut self, v: f32) -> Self {
        self.meat_energy_content = Some(v);
        self
    }

    pub fn plant_energy_content(mut self, v: f32) -> Self {
        self.plant_energy_content = Some(v);
        self
    }

    pub fn build(self) -> Result<SimulationConfig, String> {
        let rows = self.rows.unwrap_or(10);
        let columns = self.columns.unwrap_or(10);
        let starting_snakes = self.starting_snakes.unwrap_or(0);
        let starting_food = self.starting_food.unwrap_or(0);
        let food_per_step = self.food_per_step.unwrap_or(1);
        let plant_matter_per_segment = self.plant_matter_per_segment.unwrap_or(10.0);
        let wait_cost = self.wait_cost.unwrap_or(0.0);
        let move_cost = self.move_cost.unwrap_or(1.0);
        let new_segment_cost = self.new_segment_cost.unwrap_or(50.0);
        let size_to_split = self.size_to_split.unwrap_or(2);
        let species_threshold = self.species_threshold.unwrap_or(0.3);
        let mutation = self.mutation.unwrap_or_default();
        let add_walls = self.add_walls.unwrap_or(false);
        let scent_diffusion_rate = self.scent_diffusion_rate.unwrap_or(0.01);
        let scent_dispersion_per_step = self.scent_dispersion_per_step.unwrap_or(0.01);
        let create_scents = self.create_scents.unwrap_or(false);
        let snake_max_age = self.snake_max_age.unwrap_or(10000);
        let meat_energy_content = self.meat_energy_content.unwrap_or(20.0);
        let plant_energy_content = self.plant_energy_content.unwrap_or(10.0);

        // Validation
        if rows == 0 {
            return Err("Rows must be greater than 0".to_string());
        }
        if columns == 0 {
            return Err("Columns must be greater than 0".to_string());
        }
        if plant_matter_per_segment <= 0.0 {
            return Err("Plant matter per segment must be greater than 0".to_string());
        }
        if wait_cost < 0.0 {
            return Err("Wait cost must be non-negative".to_string());
        }
        if move_cost < 0.0 {
            return Err("Move cost must be non-negative".to_string());
        }
        if new_segment_cost < 0.0 {
            return Err("New segment cost must be non-negative".to_string());
        }
        if size_to_split == 0 {
            return Err("Size to split must be greater than 0".to_string());
        }
        if !(0.0..=1.0).contains(&species_threshold) {
            return Err("Species threshold must be between 0 and 1".to_string());
        }
        if !(0.0..=1.0).contains(&scent_diffusion_rate) {
            return Err("Scent diffusion rate must be between 0 and 1".to_string());
        }
        if !(0.0..=1.0).contains(&scent_dispersion_per_step) {
            return Err("Scent dispersion per step must be between 0 and 1".to_string());
        }
        if snake_max_age == 0 {
            return Err("Snake max age must be greater than 0".to_string());
        }
        if meat_energy_content <= 0.0 {
            return Err("Meat energy content must be greater than 0".to_string());
        }
        if plant_energy_content <= 0.0 {
            return Err("Plant energy content must be greater than 0".to_string());
        }

        Ok(SimulationConfig {
            rows,
            columns,
            starting_snakes,
            starting_food,
            food_per_step,
            plant_matter_per_segment,
            wait_cost,
            move_cost,
            new_segment_cost,
            size_to_split,
            species_threshold,
            mutation,
            add_walls,
            scent_diffusion_rate,
            scent_dispersion_per_step,
            create_scents,
            snake_max_age,
            meat_energy_content,
            plant_energy_content,
        })
    }
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    RepaintRequested,
    IncreaseSpeed,
    DecreaseSpeed,
    IgnoreSpeedLimit,
    FlipRunningState,
    CreateSnakes(usize),
    StopSimulation,
    UpdateSimulationConfig(SimulationConfig),
    AdvanceOneFrame,
    ResetWorld,
}

#[derive(Default, Debug, Resource, Clone, Copy)]
pub struct EngineState {
    pub repaint_needed: bool,
    pub speed_limit: Option<f32>,
    pub running: bool,
    pub frames_left: f32,
    pub frames: u32,
    pub updates_done: u32,
    pub finished: bool,
    pub ignore_speed_limit: bool,
}

#[derive(Resource)]
pub struct EngineEvents {
    pub events: Mutex<Sender<EngineEvent>>,
}

fn turn_counter(mut engine_state: ResMut<EngineState>) {
    puffin::profile_function!();
    if engine_state.speed_limit.is_some() && !engine_state.ignore_speed_limit {
        engine_state.frames_left -= 1.0;
    }
    engine_state.updates_done += 1;
    engine_state.frames += 1;
}

fn should_simulate_frame(engine_state: Res<EngineState>) -> bool {
    let result = engine_state.ignore_speed_limit
        || engine_state.speed_limit.is_none()
        || (engine_state.running && engine_state.frames_left > 0.0);
    if result && !engine_state.running {
        tracing::warn!(
            "Simulating frame while not running: ignore={}, speed_limit={:?}, frames_left={}",
            engine_state.ignore_speed_limit,
            engine_state.speed_limit,
            engine_state.frames_left
        );
    }
    result
}

fn should_calculate_stats(_engine_state: Res<EngineState>) -> bool {
    true
}
fn should_despawn_food(engine_state: Res<EngineState>) -> bool {
    engine_state.frames % 10 == 0
}

fn should_increase_age(engine_state: Res<EngineState>) -> bool {
    engine_state.frames % 10 == 0
}

impl Simulation {
    pub fn new(
        name: String,
        engine_events: Sender<EngineEvent>,
        engine_commands: Option<Arc<Mutex<Receiver<EngineCommand>>>>,
        config: SimulationConfig,
    ) -> Self {
        let mut world = World::new();
        let innovation_tracker = InnovationTracker::new();
        // for _ in 0..config.starting_snakes {
        //     world.spawn(create_snake(config.energy_per_segment, (50, 50), Box::new(RandomNeuralBrain::new(&mut innovation_tracker))));
        // }
        // for _ in 0..config.starting_food {
        //     world.spawn(
        // }
        let mut solids = SolidsMap {
            map: Map2d::new(config.columns, config.rows, false),
        };
        if config.add_walls {
            for x in 0..config.columns {
                let middle = config.rows / 2;
                if x != middle && x != middle + 1 && x != middle - 1 {
                    let position = Position {
                        x: x as i32,
                        y: (config.rows / 4) as i32,
                    };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                    let position = Position {
                        x: x as i32,
                        y: (2 * config.rows / 4) as i32,
                    };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                    let position = Position {
                        x: x as i32,
                        y: (3 * config.rows / 4) as i32,
                    };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                }
            }
        }
        world.insert_resource(config);
        world.insert_resource(Stats::default());
        world.insert_resource(FoodMap {
            map: Map2d::new(config.columns, config.rows, Food::default()),
        });
        world.insert_resource(solids);
        world.insert_resource(ScentMap {
            map: Map2d::new(config.columns, config.rows, 0.0),
        });
        world.insert_resource(SegmentMap {
            map: Map3d::new(config.columns, config.rows),
        });
        world.insert_resource(EngineEvents {
            events: Mutex::new(engine_events.clone()),
        });
        world.insert_resource(innovation_tracker);
        world.insert_resource(Species::default());
        let rng = RngResource {
            rng: Wyrand::default(),
        };
        world.insert_resource(rng);
        let mut first_schedule = Schedule::default();
        let mut core_schedule = Schedule::default();
        let mut secondary_schedule = Schedule::default();
        first_schedule.add_systems(
            (
                assign_species,
                starve,
                (
                    assign_missing_segments,
                    create_food,
                    incease_move_potential,
                    process_food,
                ),
                die_from_collisions,
                grow,
                add_scents,
            )
                .chain()
                .run_if(should_simulate_frame),
        );
        core_schedule.add_systems(
            (
                (
                    think,
                    increase_age.run_if(should_increase_age),
                    calculate_stats.run_if(should_calculate_stats),
                    diffuse_scents,
                ),
                (movement, update_positions, split).chain(),
                eat_food,
                destroy_old_food,
            )
                .chain()
                .run_if(should_simulate_frame),
        );
        secondary_schedule.add_systems(
            (
                (assign_solid_positions, assign_segment_positions),
                (
                    turn_counter,
                    disperse_scents,
                    despawn_food.run_if(should_despawn_food),
                ),
            )
                .chain()
                .run_if(should_simulate_frame),
        );
        let gui_schedule = Schedule::default();
        Simulation {
            first_schedule,
            core_schedule,
            secondary_schedule,
            gui_schedule,
            world,
            name,
            engine_events,
            engine_commands,
            has_gui: false,
        }
    }

    pub fn step(&mut self) {
        puffin::profile_function!();
        self.first_schedule.run(&mut self.world);
        self.core_schedule.run(&mut self.world);
        self.secondary_schedule.run(&mut self.world);
    }

    pub fn is_done(&mut self) -> bool {
        let engine_state = self.world.get_resource::<EngineState>().unwrap();
        engine_state.finished
    }

    pub fn run(&mut self) -> EngineEvent {
        let start_time = Instant::now();
        let mut all_events = vec![];
        while !self.is_done() {
            self.handle_commands();
            let events = self.simulation_loop();
            all_events.extend(events);
        }
        let duration = start_time.elapsed().as_millis();
        let engine_state = self.world.get_resource::<EngineState>().unwrap();
        let result = EngineEvent::SimulationFinished {
            steps: engine_state.frames,
            name: self.name.clone(),
            duration,
        };
        all_events.push(result.clone());
        self.send_events(all_events);
        result
    }
    fn handle_commands(&mut self) {
        if let Some(commands) = self
            .engine_commands
            .as_ref()
            .map(|arc_mutex| arc_mutex.lock())
        {
            commands.try_iter().for_each(|command| {
                match command {
                    EngineCommand::RepaintRequested => {
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.repaint_needed = true;
                    }
                    EngineCommand::IncreaseSpeed => {
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.speed_limit = engine_state
                            .speed_limit
                            .map(|limit| limit.max(0.01) * 2.0)
                            .or(Some(0.02));
                    }
                    EngineCommand::DecreaseSpeed => {
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.speed_limit = engine_state
                            .speed_limit
                            .map(|limit| limit.max(0.04) / 2.0)
                            .or(Some(0.02));
                    }
                    EngineCommand::IgnoreSpeedLimit => {
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.ignore_speed_limit = !engine_state.ignore_speed_limit;
                    }
                    EngineCommand::FlipRunningState => {
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.running = !engine_state.running;
                    }
                    EngineCommand::CreateSnakes(amount) => {
                        let config = *self.world.get_resource::<SimulationConfig>().unwrap();
                        let mut snake_data = vec![];
                        for _ in 0..amount {
                            let mut rng_temp =
                                self.world.get_resource_mut::<RngResource>().unwrap();
                            let x = rng_temp.rng.next_range(0..config.columns) as i32;
                            let y = rng_temp.rng.next_range(0..config.rows) as i32;
                            let dna = Dna::random(&mut rng_temp.rng, 8, &config.mutation);
                            snake_data.push((x, y, dna));
                        }
                        let mut entities_to_spawn = vec![];
                        let mut innovation_tracker =
                            self.world.remove_resource::<InnovationTracker>().unwrap();
                        let mut rng_resource = self.world.remove_resource::<RngResource>().unwrap();
                        {
                            for (x, y, dna) in snake_data {
                                let brain = Box::new(RandomNeuralBrain::new(
                                    &mut innovation_tracker,
                                    &mut rng_resource.rng,
                                ));
                                let (a, b, mut c, d, e) =
                                    create_snake(100.0, (x, y), brain, dna, &mut rng_resource.rng);
                                c.metabolism.segment_basic_cost =
                                    c.brain.get_neural_network().unwrap().run_cost();
                                entities_to_spawn.push((a, b, c, d, e));
                            }
                        }
                        self.world.insert_resource(innovation_tracker);
                        self.world.insert_resource(rng_resource);
                        for (a, b, c, d, e) in entities_to_spawn {
                            self.world.spawn((a, b, c, d, e));
                        }
                    }
                    EngineCommand::StopSimulation => {
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.finished = true;
                    }
                    EngineCommand::UpdateSimulationConfig(new_config) => {
                        self.world.remove_resource::<SimulationConfig>();
                        self.world.insert_resource(new_config);
                    }
                    EngineCommand::AdvanceOneFrame => {
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.ignore_speed_limit = false;
                        engine_state.speed_limit = Some(0.0);
                        engine_state.frames_left += 1.0;
                    }
                    EngineCommand::ResetWorld => {
                        // Despawn all entities
                        let entities: Vec<bevy_ecs::entity::Entity> = self
                            .world
                            .query::<bevy_ecs::entity::Entity>()
                            .iter(&self.world)
                            .collect();
                        for entity in entities {
                            self.world.despawn(entity);
                        }
                        // Get config
                        let config = *self.world.get_resource::<SimulationConfig>().unwrap();
                        // Recreate solids
                        let mut solids = SolidsMap {
                            map: Map2d::new(config.columns, config.rows, false),
                        };
                        if config.add_walls {
                            for x in 0..config.columns {
                                let middle = config.rows / 2;
                                if x != middle && x != middle + 1 && x != middle - 1 {
                                    for &y_offset in
                                        &[config.rows / 4, 2 * config.rows / 4, 3 * config.rows / 4]
                                    {
                                        let position = Position {
                                            x: x as i32,
                                            y: y_offset as i32,
                                        };
                                        solids.map.set(&position, true);
                                        self.world.spawn((Solid, position));
                                    }
                                }
                            }
                        }
                        self.world.insert_resource(solids);
                        // Reset other resources
                        *self.world.get_resource_mut::<Stats>().unwrap() = Stats::default();
                        self.world.insert_resource(FoodMap {
                            map: Map2d::new(config.columns, config.rows, Food::default()),
                        });
                        self.world.insert_resource(ScentMap {
                            map: Map2d::new(config.columns, config.rows, 0.0),
                        });
                        self.world.insert_resource(SegmentMap {
                            map: Map3d::new(config.columns, config.rows),
                        });
                        self.world.insert_resource(Species::default());
                        let mut engine_state =
                            self.world.get_resource_mut::<EngineState>().unwrap();
                        engine_state.frames = 0;
                        engine_state.updates_done = 0;
                    }
                }
            });
        }
    }

    fn simulation_loop(&mut self) -> Vec<EngineEvent> {
        self.step();
        let mut events = vec![];
        let mut engine_state = self.world.get_resource_mut::<EngineState>().unwrap();
        if engine_state.repaint_needed && engine_state.running {
            let increment = engine_state.speed_limit.unwrap_or(0.00);
            engine_state.frames_left += increment;
            if let Some(limit) = engine_state.speed_limit {
                engine_state.frames_left = engine_state.frames_left.min(limit);
            }
            events.push(EngineEvent::FrameDrawn {
                updates_left: engine_state.frames_left,
                updates_done: engine_state.updates_done,
            });
            engine_state.updates_done = 0;
        }
        engine_state.repaint_needed = false;
        events
    }

    fn send_events(&self, events: Vec<EngineEvent>) {
        for event in events {
            let _ = self.engine_events.send(event);
        }
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystemConfigs<M>) {
        self.core_schedule.add_systems(system);
    }

    pub fn add_gui_system<M>(&mut self, system: impl IntoSystemConfigs<M>) {
        self.gui_schedule.add_systems(system);
        self.has_gui = true;
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.world.insert_resource(resource);
    }
}
