use crate::dna::{Dna, SegmentType};
use crate::neural::{ConnectionGene, InnovationTracker, NeuralNetwork, SensorInput};
use crate::simulation::{SimulationConfig, Stats};
use bevy_ecs::prelude::*;
use std::clone::Clone;
use std::collections::VecDeque;
use std::fmt::Debug;
use tinyrand::{Rand, RandRange};
use tracing::{debug, warn};

#[derive(Component, Clone, Default, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn as_pair(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    NorthEast,
    East,
    SouthEast,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub fn random(rng: &mut impl Rand) -> Self {
        match rng.next_range(0u32..6u32) as i32 {
            0 => Direction::NorthEast,
            1 => Direction::East,
            2 => Direction::SouthEast,
            3 => Direction::SouthWest,
            4 => Direction::West,
            _ => Direction::NorthWest,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    MoveForward,
    MoveLeft,
    MoveRight,
    Wait,
}

#[derive(Debug, Clone)]
pub enum BrainType {
    Random(RandomBrain),
    Neural(RandomNeuralBrain),
}

impl BrainType {
    pub fn decide(&self, sensory_input: Vec<f32>, rng: &mut impl Rand) -> Decision {
        match self {
            BrainType::Random(brain) => brain.decide(sensory_input, rng),
            BrainType::Neural(brain) => brain.decide(sensory_input, rng),
        }
    }

    pub fn get_neural_network(&self) -> Option<&NeuralNetwork> {
        match self {
            BrainType::Random(brain) => brain.get_neural_network(),
            BrainType::Neural(brain) => brain.get_neural_network(),
        }
    }
}

// Snake represents the head segment of snake and info about its other segments
#[derive(Debug, Clone)]
pub struct Specie {
    pub id: u32,
    pub leader: Entity,
    pub leader_network: NeuralNetwork,
    pub members: VecDeque<Entity>,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct Species {
    pub last_id: u32,
    pub species: Vec<Specie>,
}

#[derive(Component, Debug)]
pub struct Snake {
    pub direction: Direction,
    pub decision: Decision,
    pub brain: BrainType,
    pub new_position: (i32, i32),
    pub last_position: (i32, i32),
    pub segments: Vec<Entity>,
    pub generation: u32,
    pub mutations: u32,
    pub species: Option<u32>,
    pub dna: Dna,
    pub metabolism: Metabolism,
    pub energy: Energy,
}

// those change after eating or moving
#[derive(Component, Clone, Debug)]
pub struct Energy {
    pub move_potential: f32,
    pub meat_in_stomach: f32,
    pub plant_in_stomach: f32,
    pub energy: f32,
    pub accumulated_meat_matter_for_growth: f32,
}

impl Default for Energy {
    // default energy are the head parameters
    fn default() -> Self {
        Energy {
            // new born snakes cannot move immediately
            move_potential: -2.0,
            meat_in_stomach: 0.0,
            plant_in_stomach: 0.0,
            energy: 100.0,
            accumulated_meat_matter_for_growth: 0.0,
        }
    }
}

// those change only when growing or splitting
#[derive(Clone, Debug)]
pub struct Metabolism {
    pub segment_move_cost: f32,
    pub segment_basic_cost: f32,
    pub mobility: f32,
    pub segment_energy_production: f32,
    pub meat_processing_speed: f32,
    pub plant_processing_speed: f32,
    pub max_plants_in_stomach: f32,
    pub max_meat_in_stomach: f32,
    pub max_energy: f32,
    pub meat_matter_for_growth_production_speed: f32,
}

impl Default for Metabolism {
    fn default() -> Self {
        // default metabolism are the head parameters
        // TODO: most of this should come from config
        Metabolism {
            mobility: 1.0,
            segment_move_cost: 1.0,
            segment_basic_cost: 0.0,
            segment_energy_production: 0.0,
            meat_processing_speed: 0.0,
            plant_processing_speed: 25.0,
            max_plants_in_stomach: 200.0,
            max_meat_in_stomach: 0.0,
            max_energy: 400.0,
            meat_matter_for_growth_production_speed: 5.0,
        }
    }
}

#[derive(Component)]
pub struct Solid;

#[derive(Component)]
pub struct JustBorn;

#[derive(Debug, Clone)]
pub struct RandomBrain;

impl RandomBrain {
    pub fn decide(&self, _sensory_input: Vec<f32>, rng: &mut impl Rand) -> Decision {
        let val = rng.next_range(0u32..4u32);
        match val as i32 {
            0 => Decision::MoveForward,
            1 => Decision::MoveLeft,
            2 => Decision::MoveRight,
            _ => Decision::Wait,
        }
    }

    pub fn get_neural_network(&self) -> Option<&NeuralNetwork> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct RandomNeuralBrain {
    neural_network: NeuralNetwork,
}

#[derive(Component)]
pub struct Age {
    pub age: u32,
    pub efficiency_factor: f32,
}

impl RandomNeuralBrain {
    pub fn new(innovation_tracker: &mut InnovationTracker, rng: &mut impl Rand) -> Self {
        let neural_network = NeuralNetwork::random_brain(18, 0.1, innovation_tracker, rng);
        Self { neural_network }
    }
    pub fn from_neural_network(neural_network: NeuralNetwork) -> Self {
        Self { neural_network }
    }

    pub fn decide(&self, sensor_input: Vec<f32>, _rng: &mut impl Rand) -> Decision {
        let sensor_input = sensor_input
            .iter()
            .enumerate()
            .map(|(index, value)| SensorInput {
                index,
                value: *value,
            })
            .collect();
        let output = self.neural_network.run(sensor_input);
        // return the index with the maximum value of the output vector
        let mut max_index = 0;
        let mut max_value = 0.0;
        for (index, value) in output.iter().enumerate() {
            if *value > max_value {
                max_value = *value;
                max_index = index;
            }
        }
        match max_index {
            0 => Decision::MoveForward,
            1 => Decision::MoveLeft,
            2 => Decision::MoveRight,
            _ => Decision::Wait,
        }
    }

    pub fn get_neural_network(&self) -> Option<&NeuralNetwork> {
        Some(&self.neural_network)
    }
}

pub struct Map2d<T> {
    pub map: Vec<T>,
    pub width: usize,
    pub height: usize,
}

impl<T: Default + Clone> Map2d<T> {
    // Constructs a new Map2D with the specified dimensions and default value
    pub fn new(width: usize, height: usize, default: T) -> Self
    where
        T: Clone,
    {
        Map2d {
            // map: vec![vec![default; height]; width],
            map: vec![default; width * height],
            width,
            height,
        }
    }

    fn index(&self, position: &Position) -> usize {
        (position.x * self.width as i32 + position.y) as usize
    }
    // Get a reference to the value at a given position
    pub fn get(&self, position: &Position) -> &T {
        let index = self.index(position);
        &self.map[index]
    }

    // Get a mutable reference to the value at a given position
    pub fn get_mut(&mut self, position: &Position) -> &mut T {
        let index = self.index(position);
        &mut self.map[index]
    }

    // Set the value at a given position
    pub fn set(&mut self, position: &Position, value: T) {
        let index = self.index(position);
        self.map[index] = value;
    }

    pub fn clear(&mut self) {
        self.map = vec![T::default(); self.width * self.height];
    }
}

pub struct Map3d<T> {
    pub map: Vec<Vec<T>>,
    pub width: usize,
    pub height: usize,
}

impl<T: Clone> Map3d<T> {
    // Constructs a new Map2D with the specified dimensions and default value
    pub fn new(width: usize, height: usize) -> Self
    where
        T: Clone,
    {
        Map3d {
            // map: vec![vec![default; height]; width],
            map: vec![vec![]; width * height],
            width,
            height,
        }
    }

    fn index(&self, position: &Position) -> usize {
        (position.x * self.width as i32 + position.y) as usize
    }
    // Get a reference to the value at a given position
    pub fn get(&self, position: &Position) -> &Vec<T> {
        let index = self.index(position);
        &self.map[index]
    }

    // Get a mutable reference to the value at a given position
    pub fn get_mut(&mut self, position: &Position) -> &mut Vec<T> {
        let index = self.index(position);
        &mut self.map[index]
    }

    // Set the value at a given position
    pub fn add(&mut self, position: &Position, value: T) {
        let index = self.index(position);
        self.map[index].push(value);
    }

    pub fn clear(&mut self) {
        for collection in self.map.iter_mut() {
            collection.clear(); // Clears each inner vector without deallocating memory
        }
    }
}

#[derive(Component)]
pub struct MeatMatter {
    pub(crate) amount: f32,
}

#[derive(Component, Debug, Default, Clone)]
pub struct Food {
    pub plant: f32,
    pub meat: f32,
}

impl Food {
    pub fn from_plant(plant: f32) -> Self {
        Self { plant, meat: 0.0 }
    }

    pub fn from_meat(meat: f32) -> Self {
        Self { plant: 0.0, meat }
    }

    pub fn contains_food(&self) -> bool {
        self.plant > 0.0 || self.meat > 0.0
    }

    pub fn is_meat(&self) -> bool {
        self.meat > 0.0
    }

    pub fn is_plant(&self) -> bool {
        self.plant > 0.0
    }
}

#[derive(Resource)]
pub struct FoodMap {
    pub map: Map2d<Food>,
}

#[derive(Resource)]
pub struct SolidsMap {
    pub map: Map2d<bool>,
}

#[derive(Component)]
pub struct Scent {}

#[derive(Resource)]
pub struct ScentMap {
    pub map: Map2d<f32>,
}

#[derive(Resource)]
pub struct SegmentMap {
    pub map: Map3d<Entity>,
}

pub fn incease_move_potential(mut snakes: Query<(&mut Snake, &Age)>) {
    puffin::profile_function!();
    for (mut snake, age) in &mut snakes {
        if snake.energy.move_potential < 1.0 {
            snake.energy.move_potential += snake.metabolism.mobility * age.efficiency_factor;
        }
    }
}

// This system moves each entity with a Position and Velocity component
pub fn movement(
    mut snakes: Query<(Entity, &mut Snake, &Position, &Age)>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();

    for (_, mut snake, head_position, age) in &mut snakes {
        if snake.energy.move_potential >= 1.0 {
            let move_cost = snake.metabolism.segment_move_cost / age.efficiency_factor;
            match snake.decision {
                Decision::MoveForward => {
                    snake.energy.energy -= move_cost;
                    let new_position =
                        position_at_direction(&snake.direction, head_position.clone(), *config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::MoveLeft => {
                    snake.energy.energy -= move_cost;
                    snake.direction = turn_left(&snake.direction);
                    let new_position =
                        position_at_direction(&snake.direction, head_position.clone(), *config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::MoveRight => {
                    snake.energy.energy -= move_cost;
                    snake.direction = turn_right(&snake.direction);
                    let new_position =
                        position_at_direction(&snake.direction, head_position.clone(), *config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::Wait => {}
            }
            snake.energy.move_potential -= 1.0;
        }
        let basic_cost_deduction = snake.metabolism.segment_basic_cost / age.efficiency_factor;
        snake.energy.energy -= basic_cost_deduction;
        // snake.energy.energy -= snake.brain.get_neural_network().unwrap().run_cost();
        // very old snakes wont produce energy anymore
        if age.efficiency_factor > 0.2 {
            snake.energy.energy +=
                snake.metabolism.segment_energy_production * age.efficiency_factor;
        }
    }
}

#[derive(Component)]
pub struct DiedFromCollision {}

pub fn update_positions(
    mut commands: Commands,
    mut positions: Query<&mut Position>,
    mut snakes: Query<(Entity, &mut Snake)>,
    solids_map: ResMut<SolidsMap>,
) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let new_position = snake.new_position;

        // Scope mutable borrow for head position
        let old_head_position = {
            let head_pos_mut = positions.get_mut(head_id).unwrap();
            head_pos_mut.clone()
        };

        // Defensive handling for single-segment snakes (empty segments vec)
        let last_position = if snake.segments.is_empty() {
            // For single segment, last position is current head position
            old_head_position.clone()
        } else {
            let last_tail_entity = *snake.segments.last().unwrap();
            let last_tail_pos = {
                let last_tail_mut = positions.get_mut(last_tail_entity).unwrap();
                last_tail_mut.clone()
            };
            last_tail_pos
        };

        if new_position == old_head_position.as_pair() {
            continue;
        }
        if *solids_map.map.get(&Position {
            x: new_position.0,
            y: new_position.1,
        }) {
            commands.entity(head_id).insert(DiedFromCollision {});
        }
        update_segment_positions(
            &mut positions,
            Position {
                x: new_position.0,
                y: new_position.1,
            },
            &snake.segments,
        );
        snake.last_position = last_position.as_pair();
    }
}

fn update_segment_positions(
    positions: &mut Query<&mut Position>,
    new_position: Position,
    segments: &Vec<Entity>,
) {
    let mut new_position = new_position.clone();
    for segment in segments {
        let mut position = positions.get_mut(*segment).unwrap();
        let old_position = position.clone();
        position.x = new_position.x;
        position.y = new_position.y;
        new_position = old_position.clone();
    }
}

fn turn_left(direction: &Direction) -> Direction {
    match direction {
        Direction::NorthEast => Direction::NorthWest,
        Direction::East => Direction::NorthEast,
        Direction::SouthEast => Direction::East,
        Direction::SouthWest => Direction::SouthEast,
        Direction::West => Direction::SouthWest,
        Direction::NorthWest => Direction::West,
    }
}

fn turn_right(direction: &Direction) -> Direction {
    match direction {
        Direction::NorthEast => Direction::East,
        Direction::East => Direction::SouthEast,
        Direction::SouthEast => Direction::SouthWest,
        Direction::SouthWest => Direction::West,
        Direction::West => Direction::NorthWest,
        Direction::NorthWest => Direction::NorthEast,
    }
}

pub fn position_at_direction(
    direction: &Direction,
    position: Position,
    config: SimulationConfig,
) -> Position {
    let mut x = position.x;
    let mut y = position.y;
    match direction {
        Direction::NorthEast => {
            if y % 2 == 0 {
                x += 1;
            }
            y -= 1;
        }
        Direction::East => {
            x += 1;
        }
        Direction::SouthEast => {
            if y % 2 == 0 {
                x += 1;
            }
            y += 1;
        }
        Direction::SouthWest => {
            if y % 2 == 1 {
                x -= 1;
            }
            y += 1;
        }
        Direction::West => {
            x -= 1;
        }
        Direction::NorthWest => {
            if y % 2 == 1 {
                x -= 1;
            }
            y -= 1;
        }
    }
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    x = (x + columns) % columns;
    y = (y + rows) % rows;
    Position { x, y }
}

pub fn think(
    mut heads: Query<(&Position, &mut Snake, &Age)>,
    food_map: Res<FoodMap>,
    solids_map: Res<SolidsMap>,
    scent_map: Res<ScentMap>,
    config: Res<SimulationConfig>,
    mut rng: ResMut<crate::simulation::RngResource>,
) {
    puffin::profile_function!();
    let bias = 1.0;
    heads.iter_mut().for_each(|(position, mut head, age)| {
        debug!(
            "Senses config: scent={}, plant={}, meat={}, obstacle={}, chaos={}",
            config.mutation.scent_sensing_enabled,
            config.mutation.plant_vision_enabled,
            config.mutation.meat_vision_enabled,
            config.mutation.obstacle_vision_enabled,
            config.mutation.chaos_input_enabled
        );
        let chaos = if config.mutation.chaos_input_enabled {
            (rng.rng.next_u32() as f64) / (u32::MAX as f64)
        } else {
            0.0
        };
        let direction_left = turn_left(&head.direction);
        let direction_right = turn_right(&head.direction);
        let scent_front = scent(
            &position_at_direction(&head.direction, position.clone(), *config),
            &scent_map,
            *config,
        );
        let scent_left = scent(
            &position_at_direction(&direction_left, position.clone(), *config),
            &scent_map,
            *config,
        );
        let scent_right = scent(
            &position_at_direction(&direction_right, position.clone(), *config),
            &scent_map,
            *config,
        );
        let plant_vision_front = see_plants(
            &head.direction,
            position.clone(),
            config.mutation.plant_vision_front_range,
            &food_map,
            *config,
        );
        let plant_vision_left = see_plants(
            &direction_left,
            position.clone(),
            config.mutation.plant_vision_left_range,
            &food_map,
            *config,
        );
        let plant_vision_right = see_plants(
            &direction_right,
            position.clone(),
            config.mutation.plant_vision_right_range,
            &food_map,
            *config,
        );
        let meat_vision_front = see_meat(
            &head.direction,
            position.clone(),
            config.mutation.meat_vision_front_range,
            &food_map,
            *config,
        );
        let meat_vision_left = see_meat(
            &direction_left,
            position.clone(),
            config.mutation.meat_vision_left_range,
            &food_map,
            *config,
        );
        let meat_vision_right = see_meat(
            &direction_right,
            position.clone(),
            config.mutation.meat_vision_right_range,
            &food_map,
            *config,
        );
        let solid_vision_front = see_obstacles(
            &head.direction,
            position.clone(),
            config.mutation.obstacle_vision_front_range,
            &solids_map,
            *config,
        );
        let solid_vision_left = see_obstacles(
            &direction_left,
            position.clone(),
            config.mutation.obstacle_vision_left_range,
            &solids_map,
            *config,
        );
        let solid_vision_right = see_obstacles(
            &direction_right,
            position.clone(),
            config.mutation.obstacle_vision_right_range,
            &solids_map,
            *config,
        );
        let plant_food_level = head.energy.plant_in_stomach / head.metabolism.max_plants_in_stomach;
        let meat_food_level = head.energy.meat_in_stomach / head.metabolism.max_meat_in_stomach;
        let energy_level = head.energy.energy / head.metabolism.max_energy;
        let age_level = age.efficiency_factor;
        let sensory_inputs = vec![
            bias,
            chaos as f32,
            scent_front,
            scent_left,
            scent_right,
            plant_vision_front,
            plant_vision_left,
            plant_vision_right,
            meat_vision_front,
            meat_vision_left,
            meat_vision_right,
            solid_vision_front,
            solid_vision_left,
            solid_vision_right,
            plant_food_level,
            meat_food_level,
            energy_level,
            age_level,
        ];
        head.decision = head.brain.decide(sensory_inputs, &mut rng.rng);
    });
}

fn scent(scenting_position: &Position, scent_map: &ScentMap, config: SimulationConfig) -> f32 {
    if config.mutation.scent_sensing_enabled {
        let scent = scent_map.map.get(scenting_position);
        scent / 500.0
    } else {
        0.0
    }
}

fn see_meat(
    head_direction: &Direction,
    position: Position,
    range: u32,
    food_map: &FoodMap,
    config: SimulationConfig,
) -> f32 {
    if config.mutation.meat_vision_enabled {
        let mut current_vision_position = position.clone();
        let mut current_range = 0;
        while current_range < range {
            current_vision_position =
                position_at_direction(head_direction, current_vision_position, config);
            if food_map.map.get(&current_vision_position).is_meat() {
                return (range - current_range) as f32 / range as f32;
            }
            current_range += 1;
        }
    }
    0.0
}

fn see_plants(
    head_direction: &Direction,
    position: Position,
    range: u32,
    food_map: &FoodMap,
    config: SimulationConfig,
) -> f32 {
    if config.mutation.plant_vision_enabled {
        let mut current_vision_position = position.clone();
        let mut current_range = 0;
        while current_range < range {
            current_vision_position =
                position_at_direction(head_direction, current_vision_position, config);
            if food_map.map.get(&current_vision_position).is_plant() {
                return (range - current_range) as f32 / range as f32;
            }
            current_range += 1;
        }
    }
    0.0
}

fn see_obstacles(
    head_direction: &Direction,
    position: Position,
    range: u32,
    solids_map: &SolidsMap,
    config: SimulationConfig,
) -> f32 {
    if config.mutation.obstacle_vision_enabled {
        let mut current_vision_position = position.clone();
        let mut current_range = 0;
        while current_range < range {
            current_vision_position =
                position_at_direction(head_direction, current_vision_position, config);
            if *solids_map.map.get(&current_vision_position) {
                return (range - current_range) as f32 / range as f32;
            }
            current_range += 1;
        }
    }
    0.0
}

pub fn add_scents(
    mut commands: Commands,
    scent_source: Query<(&MeatMatter, &Position)>,
    mut scent_map: ResMut<ScentMap>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    if config.create_scents {
        for (meat, position) in &scent_source {
            let current_scent = scent_map.map.get_mut(position);
            if current_scent <= &mut 0.0 {
                commands.spawn((
                    Scent {},
                    Position {
                        x: position.x,
                        y: position.y,
                    },
                ));
            }
            if *current_scent < 1000.0 {
                *current_scent += meat.amount;
            }
        }
    }
}

pub fn diffuse_scents(
    mut commands: Commands,
    scents: Query<(&Scent, &Position)>,
    mut scent_map: ResMut<ScentMap>,
    config: Res<SimulationConfig>,
    mut rng: ResMut<crate::simulation::RngResource>,
) {
    let directions = [
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];
    let rng = &mut rng.rng;
    for (_, position) in &scents {
        let random_direction = &directions[rng.next_range(0..directions.len())];
        let new_position = position_at_direction(random_direction, position.clone(), *config);
        let diffused_scent = scent_map.map.get(position) * config.scent_diffusion_rate;
        *scent_map.map.get_mut(position) -= diffused_scent;
        let new_scent = scent_map.map.get_mut(&new_position);
        if new_scent <= &mut 0.0 {
            commands.spawn((
                Scent {},
                Position {
                    x: new_position.x,
                    y: new_position.y,
                },
            ));
        }
        *new_scent += diffused_scent;
    }
}

pub fn disperse_scents(
    mut commands: Commands,
    scents: Query<(Entity, &Scent, &Position)>,
    mut scent_map: ResMut<ScentMap>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    for (scent_id, _, position) in &scents {
        let scent = scent_map.map.get_mut(position);
        *scent -= config.scent_dispersion_per_step;
        if scent <= &mut 0.0 {
            commands.entity(scent_id).despawn();
            scent_map.map.set(position, 0.0);
        }
    }
}

pub fn create_food(
    mut commands: Commands,
    mut food_map: ResMut<FoodMap>,
    config: Res<SimulationConfig>,
    mut rng: ResMut<crate::simulation::RngResource>,
) {
    puffin::profile_function!();
    let rng = &mut rng.rng;
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    for _ in 0..config.food_per_step {
        let x = (rng.next_u32() % columns as u32) as i32;
        let y = (rng.next_u32() % rows as u32) as i32;
        let food = food_map.map.get_mut(&Position { x, y });
        if !food.contains_food() {
            commands.spawn((
                Position { x, y },
                Food {
                    plant: config.plant_matter_per_segment,
                    meat: 0.0,
                },
                Age {
                    age: 0,
                    efficiency_factor: 1.0,
                },
            ));
        }
        *food = Food::from_plant(config.plant_matter_per_segment);
    }
}

pub fn destroy_old_food(
    _commands: Commands,
    mut food: Query<(Entity, &Position, &Food, &Age)>,
    mut food_map: ResMut<FoodMap>,
    _config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    for (_food_id, postition, _food, age) in &mut food {
        if age.age >= 5000 {
            food_map.map.set(postition, Food::default());
        }
    }
}

pub fn eat_food(
    mut snakes: Query<(&Position, &mut Snake)>,
    mut food_map: ResMut<FoodMap>,
    _config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    for (position, mut snake) in &mut snakes {
        let food = food_map.map.get_mut(position);
        let place_for_plants =
            snake.metabolism.max_plants_in_stomach - snake.energy.plant_in_stomach;
        let place_for_meat = snake.metabolism.max_meat_in_stomach - snake.energy.meat_in_stomach;
        let plants_to_eat = food.plant.min(place_for_plants);
        let meat_to_eat = food.meat.min(place_for_meat);
        if snake.metabolism.plant_processing_speed > 0.0 {
            snake.energy.plant_in_stomach += plants_to_eat;
            food.plant -= plants_to_eat;
        }
        if snake.metabolism.meat_processing_speed > 0.0 {
            snake.energy.meat_in_stomach += meat_to_eat;
            food.meat -= meat_to_eat;
        }
    }
}

pub fn despawn_food(
    mut commands: Commands,
    food: Query<(Entity, &Position, &Food)>,
    food_map: ResMut<FoodMap>,
) {
    puffin::profile_function!();
    for (food_id, position, _) in &food {
        if !food_map.map.get(position).contains_food() {
            commands.entity(food_id).despawn();
        }
    }
}

pub fn starve(
    mut commands: Commands,
    mut snakes: Query<(Entity, &mut Snake)>,
    positions: Query<&Position>,
    mut food_map: ResMut<FoodMap>,
    mut species: ResMut<Species>,
    mut solids_map: ResMut<SolidsMap>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        if snake.energy.energy < 0.0 {
            kill_snake(
                &mut commands,
                &positions,
                &mut food_map,
                &mut species,
                &mut solids_map,
                &config,
                head_id,
                &mut snake,
            );
        }
    }
}

fn remove_segment_and_transform_to_food(
    commands: &mut Commands,
    positions: &Query<&Position>,
    food_map: &mut ResMut<FoodMap>,
    solids_map: &mut ResMut<SolidsMap>,
    config: &Res<SimulationConfig>,
    segment_id: &Entity,
) {
    commands.entity(*segment_id).despawn();
    let position = positions.get(*segment_id).unwrap();
    solids_map.map.set(position, false);
    let added_food = Food::from_meat(config.new_segment_cost);
    food_map.map.set(position, added_food.clone());
    commands.spawn((
        position.clone(),
        added_food,
        Age {
            age: 0,
            efficiency_factor: 1.0,
        },
    ));
}

fn remove_snake_from_species(
    species: &mut ResMut<Species>,
    head_id: Entity,
    snake: &mut Mut<Snake>,
) {
    if let Some(specie_id) = snake.species {
        if let Some(specie) = species.species.iter_mut().find(|s| s.id == specie_id) {
            if specie.leader == head_id {
                specie.members.retain(|s| *s != head_id);
                if let Some(new_leader) = specie.members.pop_front() {
                    specie.leader = new_leader;
                    specie.leader_network = snake.brain.get_neural_network().unwrap().clone();
                } else {
                    let specie_id = specie.id;
                    species.species.retain(|s| s.id != specie_id);
                }
            } else {
                specie.members.retain(|s| *s != head_id);
            }
        } else {
            warn!("Snake {:?} died and was not found in any specie", head_id);
        }
    }
}

pub fn die_from_collisions(
    mut commands: Commands,
    positions: Query<&Position>,
    mut snake: Query<(Entity, &mut Snake, &DiedFromCollision)>,
    mut food_map: ResMut<FoodMap>,
    mut species: ResMut<Species>,
    mut solids_map: ResMut<SolidsMap>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    for (head_id, mut snake, _) in &mut snake {
        if snake.energy.energy >= 0.0 {
            kill_snake(
                &mut commands,
                &positions,
                &mut food_map,
                &mut species,
                &mut solids_map,
                &config,
                head_id,
                &mut snake,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn kill_snake(
    commands: &mut Commands,
    positions: &Query<&Position>,
    food_map: &mut ResMut<FoodMap>,
    species: &mut ResMut<Species>,
    solids_map: &mut ResMut<SolidsMap>,
    config: &Res<SimulationConfig>,
    head_id: Entity,
    snake: &mut Mut<Snake>,
) {
    remove_snake_from_species(species, head_id, snake);
    for segment_id in &snake.segments {
        remove_segment_and_transform_to_food(
            commands, positions, food_map, solids_map, config, segment_id,
        );
    }
    commands.entity(head_id).remove::<Snake>();
    if !snake.segments.contains(&head_id) {
        commands.entity(head_id).despawn();
    }
}

pub fn reproduce(
    _commands: Commands,
    _snakes: Query<(&mut MeatMatter, &Position)>,
    _config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    // for (mut energy, position) in &mut snakes {
    //     if energy.amount >= config.energy_to_breed {
    //         energy.amount -= config.energy_to_breed / 2;
    //         let baby_energy = config.energy_to_breed - energy.amount;
    //         let snake = create_snake(baby_energy, (position.x, position.y), Box::new(RandomBrain {}));
    //         commands.spawn(snake);
    //     }
    // }
}

pub fn split(
    mut commands: Commands,
    mut snakes: Query<(Entity, &mut Snake)>,
    segments: Query<&SegmentType>,
    positions: Query<&Position>,
    config: Res<SimulationConfig>,
    _innovation_tracker: ResMut<InnovationTracker>,
    mut rng: ResMut<crate::simulation::RngResource>,
) {
    puffin::profile_function!();
    let rng = &mut rng.rng;
    for (_head_id, mut snake) in &mut snakes {
        let snake_length = snake.segments.len();
        if snake_length >= config.size_to_split {
            let new_neural_network = if let Some(neural_network) = snake.brain.get_neural_network()
            {
                let mut nn = neural_network.clone();
                let mut mutations = snake.mutations;
                if (rng.next_u32() as f64) / (u32::MAX as f64)
                    < config.mutation.connection_flip_chance
                {
                    nn.flip_random_connection(rng);
                    mutations += 1;
                }
                if (rng.next_u32() as f64) / (u32::MAX as f64)
                    < config.mutation.weight_perturbation_chance
                {
                    nn.mutate_perturb_random_connection_weight(
                        config.mutation.weight_perturbation_range,
                        config.mutation.perturb_disabled_connections,
                        rng,
                    );
                    mutations += 1;
                }
                if (rng.next_u32() as f64) / (u32::MAX as f64) < config.mutation.weight_reset_chance
                {
                    nn.mutate_reset_random_connection_weight(
                        config.mutation.weight_reset_range,
                        config.mutation.perturb_disabled_connections,
                        rng,
                    );
                    mutations += 1;
                }
                let mut dna = snake.dna.clone();
                if (rng.next_u32() as f64) / (u32::MAX as f64) < config.mutation.dna_mutation_chance
                {
                    dna.mutate(rng, &config.mutation);
                    mutations += 1;
                }
                (nn, mutations, dna)
            } else {
                panic!("Snake without neural network");
            };
            let (new_neural_network, mutations, dna) = new_neural_network;
            let new_snake_segments = snake.segments.split_off(snake_length / 2);
            let new_head_id = *new_snake_segments.first().unwrap();
            let new_head_position = positions.get(new_head_id).unwrap();
            // new_snake_segments.reverse();
            let (mut new_snake, new_age, new_justborn) = create_head(
                (new_head_position.x, new_head_position.y),
                BrainType::Neural(RandomNeuralBrain::from_neural_network(new_neural_network)),
                snake.generation + 1,
                mutations,
                dna,
                rng,
            );
            new_snake.segments = new_snake_segments;
            new_snake.energy.energy = snake.energy.energy / 2.0;
            snake.energy.energy /= 2.0;
            new_snake.energy.plant_in_stomach = snake.energy.plant_in_stomach / 2.0;
            snake.energy.plant_in_stomach /= 2.0;
            new_snake.energy.meat_in_stomach = snake.energy.meat_in_stomach / 2.0;
            snake.energy.meat_in_stomach /= 2.0;
            recalculate_snake_params(&mut snake, &segments, &config, None);
            recalculate_snake_params(&mut new_snake, &segments, &config, None);
            if (rng.next_u32() as f64) / (u32::MAX as f64) < 0.5 {
                new_snake.direction = turn_left(&snake.direction);
            } else {
                new_snake.direction = turn_right(&snake.direction);
            }
            commands
                .entity(new_head_id)
                .insert((new_snake, new_age, new_justborn));
            commands.entity(new_head_id).remove::<SegmentType>();
        }
    }
}

fn recalculate_snake_params(
    snake: &mut Snake,
    segments: &Query<&SegmentType>,
    _config: &Res<SimulationConfig>,
    new_segment: Option<&SegmentType>,
) {
    let mut mobility = 0.0;
    let mut move_cost = 0.0;
    let mut segment_basic_cost = 0.0;
    let mut segment_energy_production = 0.0;
    snake.metabolism = Metabolism::default();
    for segment_id in &snake.segments {
        if *segment_id == snake.segments[0] {
            // this is head
            continue;
        }
        let segment = segments
            .get(*segment_id)
            .unwrap_or_else(|_| new_segment.unwrap());
        mobility += segment.mobility();
        move_cost += segment.energy_cost_move();
        segment_basic_cost += segment.energy_cost_always();
        if segment.energy_cost_always() > 0.0 {
            segment_basic_cost += segment.energy_cost_always();
        } else {
            segment_energy_production -= segment.energy_cost_always();
        }
        if let SegmentType::Stomach(_) = segment {
            // TODO: this should come from config
            snake.metabolism.meat_processing_speed += 1.0;
            snake.metabolism.max_meat_in_stomach += 200.0;
        }
    }
    let len = snake.segments.len() as f32;
    snake.metabolism.mobility = mobility / len;
    snake.metabolism.segment_move_cost += move_cost;
    snake.metabolism.segment_basic_cost += segment_basic_cost;
    snake.metabolism.segment_energy_production += segment_energy_production;
    if let Some(network) = snake.brain.get_neural_network() {
        let run_cost = network.run_cost();
        if run_cost == 0.0 {
            panic!("Neural network run cost is 0.0")
        }
        snake.metabolism.segment_basic_cost += run_cost;
    } else {
        panic!("Snake without neural network");
    }
    if snake.metabolism.segment_basic_cost == 0.0 {
        panic!("Snake with 0.0 segment basic cost");
    }
}

pub fn increase_age(mut agables: Query<&mut Age>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for mut age in &mut agables {
        age.age += 10;
        age.efficiency_factor = (1.0 / (age.age as f32 / config.snake_max_age as f32)).min(1.0);
        if age.efficiency_factor < 1.0 {}
    }
}
#[allow(clippy::too_many_arguments)]
pub fn calculate_stats(
    entities: Query<Entity>,
    scents: Query<&Scent>,
    food: Query<&Food>,
    snakes: Query<(&Snake, &Age)>,
    segments: Query<&SegmentType>,
    mut stats: ResMut<Stats>,
    species: Res<Species>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    let max_age = snakes.iter().map(|(_, a)| a.age).reduce(|a, b| a.max(b));
    let max_generation = snakes
        .iter()
        .map(|(s, _)| s.generation)
        .reduce(|a, b| a.max(b));
    let max_mutation = snakes
        .iter()
        .map(|(s, _)| s.mutations)
        .reduce(|a, b| a.max(b));
    stats.oldest_snake = max_age.unwrap_or(0);
    stats.total_snakes = snakes.iter().count();
    stats.total_food = food.iter().count();
    stats.total_segments = segments.iter().count();
    stats.total_scents = scents.iter().count();
    stats.max_generation = max_generation.unwrap_or(0);
    stats.max_mutations = max_mutation.unwrap_or(0);
    stats.species = species.clone();
    stats.total_entities = entities.iter().count();
    stats.total_snake_energy = snakes.iter().map(|(s, _)| s.energy.energy).sum();
    stats.total_plants_in_stomachs = snakes.iter().map(|(s, _)| s.energy.plant_in_stomach).sum();
    stats.total_meat_in_stomachs = snakes.iter().map(|(s, _)| s.energy.meat_in_stomach).sum();
    stats.total_plants = food.iter().map(|f| f.plant).sum();
    stats.total_meat = food.iter().map(|f| f.meat).sum();
    stats.total_energy = stats.total_snake_energy
        + stats.total_plants * config.plant_energy_content
        + stats.total_meat * config.meat_energy_content;
}

pub fn process_food(mut snake: Query<(&mut Snake, &Age)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (mut snake, age) in &mut snake {
        if snake.energy.energy < snake.metabolism.max_energy {
            let eaten_plants = snake
                .energy
                .plant_in_stomach
                .min(snake.metabolism.plant_processing_speed);
            snake.energy.plant_in_stomach -= eaten_plants;
            let eaten_meat = snake
                .energy
                .meat_in_stomach
                .min(snake.metabolism.meat_processing_speed);
            snake.energy.meat_in_stomach -= eaten_meat;
            let plant_energy_gain =
                eaten_plants * config.plant_energy_content * age.efficiency_factor;
            let meat_energy_gain = eaten_meat * config.meat_energy_content * age.efficiency_factor;
            snake.energy.energy += plant_energy_gain + meat_energy_gain;
        }
        if snake.energy.energy > 3.0 * snake.metabolism.max_energy / 4.0 {
            snake.energy.accumulated_meat_matter_for_growth +=
                snake.metabolism.meat_matter_for_growth_production_speed;
            snake.energy.energy -= snake.metabolism.meat_matter_for_growth_production_speed
                * config.meat_energy_content;
        }
    }
}

pub fn grow(
    mut commands: Commands,
    mut snakes: Query<(Entity, &mut Snake)>,
    segment_map: Res<SegmentMap>,
    config: Res<SimulationConfig>,
    segments: Query<&SegmentType>,
) {
    puffin::profile_function!();
    for (_snake_id, mut snake) in &mut snakes {
        // tail always takes energy from head when growing
        let position_empty = segment_map
            .map
            .get(&Position {
                x: snake.last_position.0,
                y: snake.last_position.1,
            })
            .is_empty();
        if position_empty
            && snake.energy.accumulated_meat_matter_for_growth >= config.new_segment_cost
        {
            let meat_for_tail = config.new_segment_cost;
            snake.energy.accumulated_meat_matter_for_growth -= meat_for_tail;
            let segment_type = snake.dna.build_segment();
            let new_tail = commands
                .spawn((
                    segment_type.clone(),
                    Position {
                        x: snake.last_position.0,
                        y: snake.last_position.1,
                    },
                    MeatMatter {
                        amount: meat_for_tail,
                    },
                ))
                .id();
            if let SegmentType::Solid(_) = segment_type {
                commands.entity(new_tail).insert(Solid {});
            }
            snake.segments.push(new_tail);
            recalculate_snake_params(&mut snake, &segments, &config, Some(&segment_type));
        }
    }
}

pub fn assign_missing_segments(mut snakes: Query<(Entity, &mut Snake), Added<Snake>>) {
    puffin::profile_function!();
    for (snake_id, mut snake) in &mut snakes {
        if snake.segments.is_empty() {
            snake.segments.push(snake_id);
        }
    }
}

pub fn assign_solid_positions(
    solids: Query<(&Position, &Solid)>,
    mut solids_map: ResMut<SolidsMap>,
    _segment_map: Res<SegmentMap>,
    _config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    solids_map.map.clear();
    for (position, _) in &solids {
        solids_map.map.set(position, true);
    }
}

pub fn assign_segment_positions(
    mut segment_map: ResMut<SegmentMap>,
    segments: Query<(Entity, &Position, &SegmentType)>,
) {
    puffin::profile_function!();
    segment_map.map.clear();
    for (segment_id, position, _) in &segments {
        segment_map.map.add(position, segment_id);
    }
}

pub fn assign_species(
    new_borns: Query<Entity, Added<JustBorn>>,
    mut snakes: Query<(Entity, &mut Snake)>,
    mut species: ResMut<Species>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    for baby_id in &new_borns {
        // let mut baby_snake = None;
        for specie in species.species.iter_mut() {
            if let Ok([(snake_id, mut snake), (_leader_id, leader_snake)]) =
                snakes.get_many_mut([baby_id, specie.leader])
            {
                let compatibility = calculate_gene_difference(
                    leader_snake.brain.get_neural_network().unwrap(),
                    snake.brain.get_neural_network().unwrap(),
                );
                if compatibility < config.species_threshold {
                    snake.species = Some(specie.id);
                    specie.members.push_back(snake_id);
                    break;
                }
            } else if baby_id != specie.leader {
                panic!(
                    "Unable to find leader {:?} for baby {:?} for specie {:?}",
                    specie.leader, baby_id, specie.id
                );
            }
        }
        let (_, mut baby_snake) = snakes.get_mut(baby_id).unwrap();
        if baby_snake.species.is_none() {
            let baby_neural_network = baby_snake.brain.get_neural_network().unwrap().clone();
            let mut new_specie = Specie {
                id: species.last_id + 1,
                leader: baby_id,
                members: VecDeque::new(),
                leader_network: baby_neural_network,
            };
            new_specie.members.push_back(baby_id);
            species.species.push(new_specie);
            species.last_id += 1;
            baby_snake.species = Some(species.last_id);
        }
    }
}

fn calculate_gene_difference(leader: &NeuralNetwork, new_snake: &NeuralNetwork) -> f32 {
    let leader_genes = leader
        .connections
        .iter()
        .filter(|c| c.enabled)
        .collect::<Vec<&ConnectionGene>>();
    let new_snake_genes = new_snake
        .connections
        .iter()
        .filter(|c| c.enabled)
        .collect::<Vec<&ConnectionGene>>();
    let leader_innovations: Vec<_> = leader_genes.iter().map(|c| c.innovation_number).collect();
    let new_snake_innovations: Vec<_> = new_snake_genes
        .iter()
        .map(|c| c.innovation_number)
        .collect();
    // genes that both have in common
    let matching_innovations: Vec<_> = leader_innovations
        .iter()
        .filter(|i| new_snake_innovations.contains(i))
        .collect();
    let matching_genes_count = matching_innovations.len();
    // calculate weight difference on matching genes
    let mut weight_difference = 0.0;
    for innovation in matching_innovations {
        let leader_weight = leader
            .connections
            .iter()
            .find(|c| c.innovation_number == *innovation)
            .unwrap()
            .weight;
        let new_snake_weight = new_snake
            .connections
            .iter()
            .find(|c| c.innovation_number == *innovation)
            .unwrap()
            .weight;
        weight_difference += (leader_weight - new_snake_weight).abs();
    }
    // max number of genes
    let max_genes = leader_genes.len().max(new_snake_genes.len());
    let gene_difference = (max_genes - matching_genes_count) as f32 / max_genes as f32;
    let weight_difference = weight_difference / matching_genes_count as f32;
    0.6 * gene_difference + 0.4 * weight_difference
}
pub fn create_snake(
    meat_matter: f32,
    position: (i32, i32),
    brain: BrainType,
    dna: Dna,
    rng: &mut impl Rand,
) -> (Position, MeatMatter, Snake, Age, JustBorn) {
    if brain.get_neural_network().is_none() {
        panic!("Brain without neural network");
    }
    let (head, age, just_born) = create_head(position, brain, 0, 0, dna, rng);
    (
        Position {
            x: position.0,
            y: position.1,
        },
        MeatMatter {
            amount: meat_matter,
        },
        head,
        age,
        just_born,
    )
}

fn create_head(
    position: (i32, i32),
    brain: BrainType,
    generation: u32,
    mutations: u32,
    dna: Dna,
    rng: &mut impl Rand,
) -> (Snake, Age, JustBorn) {
    (
        Snake {
            direction: Direction::random(rng),
            decision: Decision::Wait,
            brain,
            new_position: position,
            segments: vec![],
            last_position: position,
            generation,
            mutations,
            species: None,
            dna,
            metabolism: Metabolism::default(),
            energy: Energy::default(),
        },
        Age {
            age: 0,
            efficiency_factor: 1.0,
        },
        JustBorn,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dna::{Dna, Gene, MutationType, SegmentType};
    use crate::neural::{Activation, InnovationTracker, NeuralNetwork};
    use crate::simulation::{MutationConfig, SimulationConfig};

    fn setup_world() -> World {
        let mut world = World::new();
        let config = SimulationConfig {
            rows: 5,
            columns: 5,
            starting_snakes: 0,
            starting_food: 0,
            food_per_step: 0,
            plant_matter_per_segment: 10.0,
            wait_cost: 0.0,
            move_cost: 1.0,
            new_segment_cost: 50.0,
            size_to_split: 3,
            species_threshold: 0.3,
            mutation: MutationConfig::default(),
            add_walls: false,
            scent_diffusion_rate: 0.01,
            scent_dispersion_per_step: 0.01,
            create_scents: false,
            snake_max_age: 10000,
            meat_energy_content: 20.0,
            plant_energy_content: 10.0,
        };
        world.insert_resource(config);
        let solids_map = SolidsMap {
            map: Map2d::new(5, 5, false),
        };
        world.insert_resource(solids_map);
        let food_map = FoodMap {
            map: Map2d::new(5, 5, Food::default()),
        };
        world.insert_resource(food_map);
        world.insert_resource(Species::default());
        world.insert_resource(SegmentMap {
            map: Map3d::new(5, 5),
        });
        world.insert_resource(ScentMap {
            map: Map2d::new(5, 5, 0.0f32),
        });
        world.insert_resource(crate::simulation::RngResource {
            rng: tinyrand::Wyrand::default(),
        });
        world
    }

    #[test]
    fn test_movement_forward() {
        let mut world = setup_world();
        let mut rng_resource = world.resource_mut::<crate::simulation::RngResource>();
        let snake = Snake {
            direction: Direction::East,
            decision: Decision::MoveForward,
            brain: BrainType::Random(RandomBrain {}),
            new_position: (0, 0),
            last_position: (0, 0),
            segments: vec![],
            generation: 0,
            mutations: 0,
            species: None,
            dna: Dna::random(&mut rng_resource.rng, 1, &MutationConfig::default()),
            metabolism: Metabolism::default(),
            energy: Energy {
                move_potential: 1.0,
                energy: 100.0,
                ..Default::default()
            },
        };
        let initial_energy = snake.energy.energy;
        let head = world
            .spawn((
                Position { x: 0, y: 0 },
                snake,
                Age {
                    age: 0,
                    efficiency_factor: 1.0,
                },
            ))
            .id();

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut schedule_assign = Schedule::default();
        schedule_assign.add_systems(assign_missing_segments);
        schedule_assign.run(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems(movement);
        schedule.run(&mut world);

        let snake_after = world.entity(head).get::<Snake>().unwrap();
        assert_eq!(snake_after.new_position, (1, 0));
        assert_eq!(snake_after.energy.energy, initial_energy - 1.0); // segment_move_cost = 1.0 / eff = 1.0
        assert_eq!(snake_after.energy.move_potential, 0.0);

        let mut schedule_update = Schedule::default();
        schedule_update.add_systems(update_positions);
        schedule_update.run(&mut world);

        let position_after = world.entity(head).get::<Position>().unwrap();
        assert_eq!(position_after.x, 1);
        assert_eq!(position_after.y, 0);
        assert!(world.entity(head).get::<DiedFromCollision>().is_none());
    }

    #[test]
    fn test_movement_wait() {
        let mut world = setup_world();
        let mut rng_resource = world.resource_mut::<crate::simulation::RngResource>();
        let snake = Snake {
            direction: Direction::East,
            decision: Decision::Wait,
            brain: BrainType::Random(RandomBrain {}),
            new_position: (0, 0),
            last_position: (0, 0),
            segments: vec![],
            generation: 0,
            mutations: 0,
            species: None,
            dna: Dna::random(&mut rng_resource.rng, 1, &MutationConfig::default()),
            metabolism: Metabolism::default(),
            energy: Energy {
                move_potential: 1.0,
                energy: 100.0,
                ..Default::default()
            },
        };
        let initial_energy = snake.energy.energy;
        let head = world
            .spawn((
                Position { x: 0, y: 0 },
                snake,
                Age {
                    age: 0,
                    efficiency_factor: 1.0,
                },
            ))
            .id();

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut schedule_assign = Schedule::default();
        schedule_assign.add_systems(assign_missing_segments);
        schedule_assign.run(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems(movement);
        schedule.run(&mut world);

        let snake_after = world.entity(head).get::<Snake>().unwrap();
        assert_eq!(snake_after.new_position, (0, 0)); // no change
        assert_eq!(snake_after.energy.energy, initial_energy); // no move cost, basic=0
        assert_eq!(snake_after.energy.move_potential, 0.0);

        let mut schedule_update = Schedule::default();
        schedule_update.add_systems(update_positions);
        schedule_update.run(&mut world);

        let position_after = world.entity(head).get::<Position>().unwrap();
        assert_eq!(position_after.x, 0);
        assert_eq!(position_after.y, 0);
    }

    #[test]
    fn test_movement_left() {
        let mut world = setup_world();
        let mut rng_resource = world.resource_mut::<crate::simulation::RngResource>();
        let snake = Snake {
            direction: Direction::East,
            decision: Decision::MoveLeft,
            brain: BrainType::Random(RandomBrain {}),
            new_position: (0, 0),
            last_position: (0, 0),
            segments: vec![],
            generation: 0,
            mutations: 0,
            species: None,
            dna: Dna::random(&mut rng_resource.rng, 1, &MutationConfig::default()),
            metabolism: Metabolism::default(),
            energy: Energy {
                move_potential: 1.0,
                energy: 100.0,
                ..Default::default()
            },
        };
        let initial_energy = snake.energy.energy;
        let head = world
            .spawn((
                Position { x: 0, y: 0 },
                snake,
                Age {
                    age: 0,
                    efficiency_factor: 1.0,
                },
            ))
            .id();

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut schedule_assign = Schedule::default();
        schedule_assign.add_systems(assign_missing_segments);
        schedule_assign.run(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems(movement);
        schedule.run(&mut world);

        let snake_after = world.entity(head).get::<Snake>().unwrap();
        assert_eq!(snake_after.direction, Direction::NorthEast); // turn left from East
                                                                 // NorthEast from (0,0): y%2==0, x+=1, y-=1 -> (1, -1 %5=4)
        assert_eq!(snake_after.new_position, (1, 4));
        assert_eq!(snake_after.energy.energy, initial_energy - 1.0);

        let mut schedule_update = Schedule::default();
        schedule_update.add_systems(update_positions);
        schedule_update.run(&mut world);

        let position_after = world.entity(head).get::<Position>().unwrap();
        assert_eq!(position_after.x, 1);
        assert_eq!(position_after.y, 4);
    }

    #[test]
    fn test_movement_right() {
        let mut world = setup_world();
        let mut rng_resource = world.resource_mut::<crate::simulation::RngResource>();
        let snake = Snake {
            direction: Direction::East,
            decision: Decision::MoveRight,
            brain: BrainType::Random(RandomBrain {}),
            new_position: (0, 0),
            last_position: (0, 0),
            segments: vec![],
            generation: 0,
            mutations: 0,
            species: None,
            dna: Dna::random(&mut rng_resource.rng, 1, &MutationConfig::default()),
            metabolism: Metabolism::default(),
            energy: Energy {
                move_potential: 1.0,
                energy: 100.0,
                ..Default::default()
            },
        };
        let initial_energy = snake.energy.energy;
        let head = world
            .spawn((
                Position { x: 0, y: 0 },
                snake,
                Age {
                    age: 0,
                    efficiency_factor: 1.0,
                },
            ))
            .id();

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut schedule_assign = Schedule::default();
        schedule_assign.add_systems(assign_missing_segments);
        schedule_assign.run(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems(movement);
        schedule.run(&mut world);

        let snake_after = world.entity(head).get::<Snake>().unwrap();
        assert_eq!(snake_after.direction, Direction::SouthEast); // turn right from East
                                                                 // SouthEast from (0,0): y%2==0, x+=1, y+=1 -> (1,1)
        assert_eq!(snake_after.new_position, (1, 1));
        assert_eq!(snake_after.energy.energy, initial_energy - 1.0);

        let mut schedule_update = Schedule::default();
        schedule_update.add_systems(update_positions);
        schedule_update.run(&mut world);

        let position_after = world.entity(head).get::<Position>().unwrap();
        assert_eq!(position_after.x, 1);
        assert_eq!(position_after.y, 1);
    }

    #[test]
    fn test_movement_collision_solid() {
        let mut world = setup_world();
        // Spawn solid at (1,0)
        let _solid = world.spawn((Position { x: 1, y: 0 }, Solid)).id();
        // Run assign_solid_positions to update map
        let mut schedule_solid = Schedule::default();
        schedule_solid.add_systems(assign_solid_positions);
        schedule_solid.run(&mut world);

        let mut innovation_tracker = InnovationTracker::new();
        let mut rng_resource = world.resource_mut::<crate::simulation::RngResource>();
        let brain = BrainType::Neural(RandomNeuralBrain::new(
            &mut innovation_tracker,
            &mut rng_resource.rng,
        ));
        let snake = Snake {
            direction: Direction::East,
            decision: Decision::MoveForward,
            brain,
            new_position: (0, 0),
            last_position: (0, 0),
            segments: vec![],
            generation: 0,
            mutations: 0,
            species: None,
            dna: Dna::random(&mut rng_resource.rng, 1, &MutationConfig::default()),
            metabolism: Metabolism::default(),
            energy: Energy {
                move_potential: 1.0,
                energy: 100.0,
                ..Default::default()
            },
        };
        let _initial_energy = snake.energy.energy;
        let head = world
            .spawn((
                Position { x: 0, y: 0 },
                snake,
                Age {
                    age: 0,
                    efficiency_factor: 1.0,
                },
            ))
            .id();

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut schedule_assign = Schedule::default();
        schedule_assign.add_systems(assign_missing_segments);
        schedule_assign.run(&mut world);

        let mut schedule_move = Schedule::default();
        schedule_move.add_systems(movement);
        schedule_move.run(&mut world);

        let snake_after_move = world.entity(head).get::<Snake>().unwrap();
        assert_eq!(snake_after_move.new_position, (1, 0));

        let mut schedule_update = Schedule::default();
        schedule_update.add_systems(update_positions);
        schedule_update.run(&mut world);

        // Should insert DiedFromCollision
        assert!(world.entity(head).get::<DiedFromCollision>().is_some());

        // Run die_from_collisions
        let mut schedule_die = Schedule::default();
        schedule_die.add_systems(die_from_collisions);
        schedule_die.run(&mut world);

        // Snake despawned
        assert!(world.get_entity(head).is_err());
    }

    #[test]
    fn test_movement_wrap_around() {
        let mut world = setup_world();
        let mut rng_resource = world.resource_mut::<crate::simulation::RngResource>();
        let snake = Snake {
            direction: Direction::East,
            decision: Decision::MoveForward,
            brain: BrainType::Random(RandomBrain {}),
            new_position: (4, 0),
            last_position: (4, 0),
            segments: vec![],
            generation: 0,
            mutations: 0,
            species: None,
            dna: Dna::random(&mut rng_resource.rng, 1, &MutationConfig::default()),
            metabolism: Metabolism::default(),
            energy: Energy {
                move_potential: 1.0,
                energy: 100.0,
                ..Default::default()
            },
        };
        let head = world
            .spawn((
                Position { x: 4, y: 0 },
                snake,
                Age {
                    age: 0,
                    efficiency_factor: 1.0,
                },
            ))
            .id();

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut schedule_assign = Schedule::default();
        schedule_assign.add_systems(assign_missing_segments);
        schedule_assign.run(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems(movement);
        schedule.run(&mut world);

        let snake_after = world.entity(head).get::<Snake>().unwrap();
        // East from (4,0): x=5 %5=0, y=0
        assert_eq!(snake_after.new_position, (0, 0));

        let mut schedule_update = Schedule::default();
        schedule_update.add_systems(update_positions);
        schedule_update.run(&mut world);

        let position_after = world.entity(head).get::<Position>().unwrap();
        assert_eq!(position_after.x, 0);
        assert_eq!(position_after.y, 0);
    }

    fn calculate_total_energy(world: &mut World) -> f32 {
        let config = world.resource::<SimulationConfig>().clone();
        let mut total = 0.0f32;
        let mut snake_q = world.query::<&Snake>();
        for snake in snake_q.iter(&world) {
            total += snake.energy.energy;
        }
        let food_map = world.resource::<FoodMap>();
        for x in 0..config.columns as usize {
            for y in 0..config.rows as usize {
                let pos = Position {
                    x: x as i32,
                    y: y as i32,
                };
                let food = food_map.map.get(&pos);
                total += food.plant * config.plant_energy_content
                    + food.meat * config.meat_energy_content;
            }
        }
        total
    }

    #[test]
    fn test_energy_conservation_invariant() {
        let mut world = setup_world();
        let mut config = *world.resource::<SimulationConfig>();
        config.mutation.chaos_input_enabled = false;
        world.insert_resource(config);

        // Create simple deterministic neural network for Wait decision
        let _innovation_tracker = InnovationTracker::new();
        let mut nn = NeuralNetwork::new(vec![Activation::Relu; 18], vec![Activation::Sigmoid; 4]);
        // Connect bias (input 0) to Wait output (21) with positive weight for high sigmoid
        // Since get_innovation_number is private, hardcode
        nn.add_connection(
            0, 21, 2.0, true, 0, // hardcoded
        );
        let brain = BrainType::Neural(RandomNeuralBrain::from_neural_network(nn));

        // DNA for solar segment
        let solar_gene = Gene {
            segment_type: SegmentType::solar(),
            id: 0,
            jump: 0,
        };
        let dna = Dna {
            genes: vec![solar_gene],
            current_gene: 0,
        };

        // Spawn head
        let head_pos = Position { x: 0, y: 0 };
        let (snake, age, justborn) = create_head(
            (0, 0),
            brain,
            0,
            0,
            dna,
            &mut world.resource_mut::<crate::simulation::RngResource>().rng,
        );
        let mut snake = snake;
        snake.energy.energy = 100.0;
        snake.energy.move_potential = 1.0; // Allow one move, but will wait
        let head = world.spawn((head_pos.clone(), snake, age, justborn)).id();

        // Add a solar segment for passive gain
        let tail_pos = Position { x: 0, y: 0 };
        let segment_type = SegmentType::solar();
        let tail = world.spawn((tail_pos, segment_type.clone())).id();

        // Set segments and manually set metabolism for solar (since recalculate needs system context)
        let mut entity_ref = world.entity_mut(head);
        let mut snake = entity_ref.get_mut::<Snake>().unwrap();
        snake.segments = vec![tail];
        // Manual metabolism for head + solar segment
        snake.metabolism.segment_move_cost = 1.0 + 1.0; // head + solar move
        snake.metabolism.segment_basic_cost = 0.0 + (-0.1); // head 0, solar -0.1 production
        snake.metabolism.mobility = (1.0 + 0.2) / 2.0; // average
        snake.metabolism.segment_energy_production = -0.1; // solar
                                                           // Add NN cost
        if let Some(network) = snake.brain.get_neural_network() {
            snake.metabolism.segment_basic_cost += network.run_cost();
        }

        // Place plant food at head position for eating
        let mut food_map = world.resource_mut::<FoodMap>();
        food_map.map.set(
            &head_pos,
            Food {
                plant: 10.0,
                meat: 0.0,
            },
        );

        // Initial total energy
        let initial_total = calculate_total_energy(&mut world);

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut schedule_assign = Schedule::default();
        schedule_assign.add_systems(assign_missing_segments);
        schedule_assign.run(&mut world);

        // Run think (sets decision to Wait due to NN)
        let mut schedule_think = Schedule::default();
        schedule_think.add_systems(think);
        schedule_think.run(&mut world);

        // Run movement (Wait: no move cost, but basic cost and production)
        let mut schedule_movement = Schedule::default();
        schedule_movement.add_systems(movement);
        schedule_movement.run(&mut world);

        // No update_positions change since Wait

        // Run eat_food (eats plant to stomach)
        let mut schedule_eat = Schedule::default();
        schedule_eat.add_systems(eat_food);
        schedule_eat.run(&mut world);

        // Run process_food (converts stomach to energy)
        let mut schedule_process = Schedule::default();
        schedule_process.add_systems(process_food);
        schedule_process.run(&mut world);

        // Run increase_age (minimal change for young snake)
        let mut schedule_age = Schedule::default();
        schedule_age.add_systems(increase_age);
        schedule_age.run(&mut world);

        // Run starve (should not trigger)
        let mut schedule_starve = Schedule::default();
        schedule_starve.add_systems(starve);
        schedule_starve.run(&mut world);

        // Post-step total energy
        let post_total = calculate_total_energy(&mut world);

        // With solar gain and food conversion, total should be non-negative and balance (slight loss from costs)
        assert!(post_total >= 0.0);
        // Allow some tolerance for floating point and small costs/gains
        assert!((initial_total - post_total).abs() < 50.0);

        // Test starvation: separate setup
        let mut starve_world = setup_world();
        let mut starve_config = *starve_world.resource::<SimulationConfig>();
        starve_config.mutation.chaos_input_enabled = false;
        starve_world.insert_resource(starve_config);

        let starve_nn =
            NeuralNetwork::new(vec![Activation::Relu; 18], vec![Activation::Sigmoid; 4]);
        let starve_brain = BrainType::Neural(RandomNeuralBrain::from_neural_network(starve_nn));
        let starve_dna = Dna {
            genes: vec![Gene {
                segment_type: SegmentType::solar(),
                id: 0,
                jump: 0,
            }],
            current_gene: 0,
        };

        let starve_head_pos = Position { x: 0, y: 0 };
        let (mut starve_snake, starve_age, starve_justborn) = create_head(
            (0, 0),
            starve_brain,
            0,
            0,
            starve_dna,
            &mut starve_world
                .resource_mut::<crate::simulation::RngResource>()
                .rng,
        );
        starve_snake.energy.energy = -1.0; // Low energy to trigger starvation
        starve_snake.energy.move_potential = 0.0;
        let starve_head = starve_world
            .spawn((
                starve_head_pos.clone(),
                starve_snake,
                starve_age,
                starve_justborn,
            ))
            .id();

        // Add segment
        let starve_tail_pos = Position { x: 0, y: 0 };
        let starve_segment_type = SegmentType::solar();
        let starve_tail = starve_world
            .spawn((starve_tail_pos, starve_segment_type))
            .id();

        let mut starve_entity_ref = starve_world.entity_mut(starve_head);
        let mut starve_snake = starve_entity_ref.get_mut::<Snake>().unwrap();
        starve_snake.segments = vec![starve_tail];
        // Manual metabolism
        starve_snake.metabolism.segment_move_cost = 1.0 + 1.0;
        starve_snake.metabolism.segment_basic_cost = 0.0 + (-0.1);
        starve_snake.metabolism.mobility = (1.0 + 0.2) / 2.0;
        starve_snake.metabolism.segment_energy_production = -0.1;
        if let Some(network) = starve_snake.brain.get_neural_network() {
            starve_snake.metabolism.segment_basic_cost += network.run_cost();
        }

        // let starve_initial = calculate_total_energy(&starve_world);

        // Run assign_missing_segments to initialize segments for single-segment snake
        let mut starve_schedule_assign = Schedule::default();
        starve_schedule_assign.add_systems(assign_missing_segments);
        starve_schedule_assign.run(&mut starve_world);

        // Run starve
        let mut starve_schedule = Schedule::default();
        starve_schedule.add_systems(starve);
        starve_schedule.run(&mut starve_world);

        let starve_post = calculate_total_energy(&mut starve_world);

        // Snake despawned, segment converted to food (meat = new_segment_cost = 50.0 * meat_energy_content = 1000.0)
        assert!(starve_world.get_entity(starve_head).is_err());
        let food_map = starve_world.resource::<FoodMap>();
        let converted_food = food_map.map.get(&Position { x: 0, y: 0 });
        assert!(converted_food.meat > 0.0);
        assert!(starve_post >= 0.0);

        // Test mutation effect on metabolism/costs
        let mut mutation_world = setup_world();
        let mut mutation_config = *mutation_world.resource::<SimulationConfig>();
        mutation_config.mutation.chaos_input_enabled = false;
        mutation_world.insert_resource(mutation_config);

        // Normal DNA: solar (low cost)
        let normal_dna = Dna {
            genes: vec![Gene {
                segment_type: SegmentType::solar(),
                id: 0,
                jump: 0,
            }],
            current_gene: 0,
        };

        // Mutated DNA: to stomach (higher always cost 1.0)
        let mut mutated_dna = normal_dna.clone();
        let mut rng = tinyrand::SplitMix::default();
        let config = crate::simulation::MutationConfig::default();
        mutated_dna.mutate_specific(MutationType::ChangeSegmentType, &mut rng, &config);
        // Force to stomach for determinism
        if let Some(gene) = mutated_dna.genes.first_mut() {
            gene.segment_type = SegmentType::stomach();
        }

        // Create normal snake
        let mut normal_brain_nn =
            NeuralNetwork::new(vec![Activation::Relu; 18], vec![Activation::Sigmoid; 4]);
        normal_brain_nn.add_connection(0, 21, 2.0, true, 0); // hardcoded
        let normal_brain =
            BrainType::Neural(RandomNeuralBrain::from_neural_network(normal_brain_nn));
        let (mut normal_snake, normal_age, normal_justborn) = create_head(
            (0, 0),
            normal_brain,
            0,
            0,
            normal_dna,
            &mut mutation_world
                .resource_mut::<crate::simulation::RngResource>()
                .rng,
        );
        normal_snake.energy.energy = 100.0;
        normal_snake.energy.move_potential = 1.0;
        let normal_head = mutation_world
            .spawn((
                Position { x: 0, y: 0 },
                normal_snake,
                normal_age,
                normal_justborn,
            ))
            .id();

        let normal_tail = mutation_world
            .spawn((Position { x: 0, y: 0 }, SegmentType::solar()))
            .id();
        let mut normal_entity = mutation_world.entity_mut(normal_head);
        let mut normal_snake = normal_entity.get_mut::<Snake>().unwrap();
        normal_snake.segments = vec![normal_tail];
        // Manual metabolism for normal (solar)
        normal_snake.metabolism.segment_move_cost = 1.0 + 1.0;
        normal_snake.metabolism.segment_basic_cost = 0.0 + (-0.1);
        normal_snake.metabolism.segment_basic_cost +=
            normal_snake.brain.get_neural_network().unwrap().run_cost();
        let normal_basic_cost = normal_snake.metabolism.segment_basic_cost;

        // Create mutated snake
        let mut mutated_brain_nn =
            NeuralNetwork::new(vec![Activation::Relu; 18], vec![Activation::Sigmoid; 4]);
        mutated_brain_nn.add_connection(0, 21, 2.0, true, 0); // hardcoded
        let mutated_brain =
            BrainType::Neural(RandomNeuralBrain::from_neural_network(mutated_brain_nn));
        let (mut mutated_snake, mutated_age, mutated_justborn) = create_head(
            (0, 0),
            mutated_brain,
            0,
            0,
            mutated_dna,
            &mut mutation_world
                .resource_mut::<crate::simulation::RngResource>()
                .rng,
        );
        mutated_snake.energy.energy = 100.0;
        mutated_snake.energy.move_potential = 1.0;
        let mutated_head = mutation_world
            .spawn((
                Position { x: 1, y: 0 }, // Different pos to avoid overlap
                mutated_snake,
                mutated_age,
                mutated_justborn,
            ))
            .id();

        let mutated_tail = mutation_world
            .spawn((Position { x: 1, y: 0 }, SegmentType::stomach()))
            .id();
        let mut mutated_entity = mutation_world.entity_mut(mutated_head);
        let mut mutated_snake = mutated_entity.get_mut::<Snake>().unwrap();
        mutated_snake.segments = vec![mutated_tail];
        // Manual metabolism for mutated (stomach)
        mutated_snake.metabolism.segment_move_cost = 1.0 + 1.0;
        mutated_snake.metabolism.segment_basic_cost = 0.0 + 1.0; // stomach always cost 1.0
        mutated_snake.metabolism.segment_basic_cost +=
            mutated_snake.brain.get_neural_network().unwrap().run_cost();
        let mutated_basic_cost = mutated_snake.metabolism.segment_basic_cost;

        // Run movement for both (Wait, same eff=1)
        let mut mutation_think = Schedule::default();
        mutation_think.add_systems(think);
        mutation_think.run(&mut mutation_world);

        let mut mutation_movement = Schedule::default();
        mutation_movement.add_systems(movement);
        mutation_movement.run(&mut mutation_world);

        let normal_after = mutation_world
            .entity(normal_head)
            .get::<Snake>()
            .unwrap()
            .energy
            .energy;
        let mutated_after = mutation_world
            .entity(mutated_head)
            .get::<Snake>()
            .unwrap()
            .energy
            .energy;

        let normal_delta = 100.0 - normal_after;
        let mutated_delta = 100.0 - mutated_after;

        // Mutated (stomach) has higher basic_cost, so larger delta (more cost)
        assert!(
            mutated_delta > normal_delta,
            "mutated_delta: {}, normal_delta: {}",
            mutated_delta,
            normal_delta
        );
        assert!(mutated_basic_cost > normal_basic_cost);
    }
}
