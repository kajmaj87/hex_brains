use crate::core::Direction::{East, NorthEast, NorthWest, SouthEast, SouthWest, West};
use crate::dna::{Dna, SegmentType};
use crate::neural::{ConnectionGene, InnovationTracker, NeuralNetwork, SensorInput};
use crate::simulation::{SimulationConfig, Stats};
use bevy_ecs::prelude::*;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::clone::Clone;
use std::collections::VecDeque;
use std::fmt::Debug;
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

#[derive(Clone, Debug)]
pub enum Direction {
    NorthEast,
    East,
    SouthEast,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..=5) {
            0 => NorthEast,
            1 => East,
            2 => SouthEast,
            3 => SouthWest,
            4 => West,
            _ => NorthWest,
        }
    }
}

#[derive(Debug)]
pub enum Decision {
    MoveForward,
    MoveLeft,
    MoveRight,
    Wait,
}

pub trait Brain: Sync + Send + Debug {
    fn decide(&self, sensory_input: Vec<f32>) -> Decision;
    fn get_neural_network(&self) -> Option<&NeuralNetwork>;
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
    pub brain: Box<dyn Brain>,
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
#[derive(Debug)]
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
#[derive(Debug)]
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

#[derive(Debug)]
pub struct RandomBrain;

#[derive(Debug, Clone)]
pub struct RandomNeuralBrain {
    neural_network: NeuralNetwork,
}

#[derive(Component)]
pub struct Age {
    pub age: u32,
    pub efficiency_factor: f32,
}

impl Brain for RandomBrain {
    fn decide(&self, _: Vec<f32>) -> Decision {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..=3) {
            0 => Decision::MoveForward,
            1 => Decision::MoveLeft,
            2 => Decision::MoveRight,
            _ => Decision::Wait,
        }
    }

    fn get_neural_network(&self) -> Option<&NeuralNetwork> {
        None
    }
}

impl RandomNeuralBrain {
    pub(crate) fn new(innovation_tracker: &mut InnovationTracker) -> Self {
        let neural_network = NeuralNetwork::random_brain(18, 0.1, innovation_tracker);
        Self { neural_network }
    }
    pub(crate) fn from_neural_network(neural_network: NeuralNetwork) -> Self {
        Self { neural_network }
    }
}

impl Brain for RandomNeuralBrain {
    fn decide(&self, sensor_input: Vec<f32>) -> Decision {
        debug!("Neural network input: {:?}", sensor_input);
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
        let decision = match max_index {
            0 => Decision::MoveForward,
            1 => Decision::MoveLeft,
            2 => Decision::MoveRight,
            _ => Decision::Wait,
        };
        debug!(
            "Network architecture: {:?}",
            self.neural_network.get_active_connections()
        );
        debug!("Output: {:?}, decision: {:?}", output, decision);
        decision
    }

    fn get_neural_network(&self) -> Option<&NeuralNetwork> {
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
        debug!(
            "Energy before move: {:?}, (eff: {}, age: {})",
            snake.energy.energy, age.efficiency_factor, age.age
        );
        if snake.energy.move_potential >= 1.0 {
            let move_cost = snake.metabolism.segment_move_cost / age.efficiency_factor;
            match snake.decision {
                Decision::MoveForward => {
                    snake.energy.energy -= move_cost;
                    let new_position =
                        position_at_direction(&snake.direction, &head_position, &config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::MoveLeft => {
                    snake.energy.energy -= move_cost;
                    snake.direction = turn_left(&snake.direction);
                    let new_position =
                        position_at_direction(&snake.direction, &head_position, &config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::MoveRight => {
                    snake.energy.energy -= move_cost;
                    snake.direction = turn_right(&snake.direction);
                    let new_position =
                        position_at_direction(&snake.direction, &head_position, &config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::Wait => {}
            }
            snake.energy.move_potential -= 1.0;
        }
        snake.energy.energy -= snake.metabolism.segment_basic_cost / age.efficiency_factor;
        // snake.energy.energy -= snake.brain.get_neural_network().unwrap().run_cost();
        // very old snakes wont produce energy anymore
        if age.efficiency_factor > 0.2 {
            snake.energy.energy +=
                snake.metabolism.segment_energy_production * age.efficiency_factor;
        } else {
            debug!("Snake {:#?} is too old to produce energy", snake);
        }
        debug!(
            "Energy after move: {:?}, (eff: {}, age: {})",
            snake.energy.energy, age.efficiency_factor, age.age
        );
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
        let last_position = positions
            .get_mut(*snake.segments.last().unwrap())
            .unwrap()
            .clone();
        let head_position = positions.get_mut(head_id).unwrap();
        debug!(
            "Snake {:?} with {} segements is moving from {:?} to {:?} (last tail position: {:?})",
            head_id,
            snake.segments.len(),
            head_position,
            new_position,
            snake.last_position
        );
        let old_head_position = head_position.clone();
        if new_position == old_head_position.as_pair() {
            debug!("Snake is not moving");
            continue;
        }
        if *solids_map.map.get(&Position {
            x: new_position.0,
            y: new_position.1,
        }) {
            debug!("Snake has hit something, he will soon die");
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
        debug!(
            "Removing snake head {:?} from position {:?}",
            head_id, old_head_position
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
        debug!(
            "Updating segment {:?} to position {:?} to position {:?}",
            segment, position, new_position
        );
        position.x = new_position.x;
        position.y = new_position.y;
        new_position = old_position.clone();
    }
}

fn turn_left(direction: &Direction) -> Direction {
    match direction {
        NorthEast => NorthWest,
        East => NorthEast,
        SouthEast => East,
        SouthWest => SouthEast,
        West => SouthWest,
        NorthWest => West,
    }
}

fn turn_right(direction: &Direction) -> Direction {
    match direction {
        NorthEast => East,
        East => SouthEast,
        SouthEast => SouthWest,
        SouthWest => West,
        West => NorthWest,
        NorthWest => NorthEast,
    }
}

fn flip_direction(direction: &Direction) -> Direction {
    match direction {
        NorthEast => SouthWest,
        East => West,
        SouthEast => NorthWest,
        SouthWest => NorthEast,
        West => East,
        NorthWest => SouthEast,
    }
}

fn position_at_direction(
    direction: &Direction,
    position: &Position,
    config: &Res<SimulationConfig>,
) -> Position {
    let mut x = position.x;
    let mut y = position.y;
    match direction {
        NorthEast => {
            if y % 2 == 0 {
                x += 1;
            }
            y -= 1;
        }
        East => {
            x += 1;
        }
        SouthEast => {
            if y % 2 == 0 {
                x += 1;
            }
            y += 1;
        }
        SouthWest => {
            if y % 2 == 1 {
                x -= 1;
            }
            y += 1;
        }
        West => {
            x -= 1;
        }
        NorthWest => {
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
) {
    puffin::profile_function!();
    let bias = 1.0;
    heads.par_iter_mut().for_each(|(position, mut head, age)| {
        let mut rng = rand::thread_rng();
        let chaos = if config.mutation.chaos_input_enabled {
            rng.gen_range(0.0..1.0)
        } else {
            0.0
        };
        let direction_left = turn_left(&head.direction);
        let direction_right = turn_right(&head.direction);
        let scent_front = scent(
            &position_at_direction(&head.direction, &position, &config),
            &scent_map,
            &config,
        );
        let scent_left = scent(
            &position_at_direction(&direction_left, &position, &config),
            &scent_map,
            &config,
        );
        let scent_right = scent(
            &position_at_direction(&direction_right, &position, &config),
            &scent_map,
            &config,
        );
        let plant_vision_front = see_plants(
            &head.direction,
            &position,
            config.mutation.plant_vision_front_range,
            &food_map,
            &config,
        );
        let plant_vision_left = see_plants(
            &direction_left,
            &position,
            config.mutation.plant_vision_left_range,
            &food_map,
            &config,
        );
        let plant_vision_right = see_plants(
            &direction_right,
            &position,
            config.mutation.plant_vision_right_range,
            &food_map,
            &config,
        );
        let meat_vision_front = see_meat(
            &head.direction,
            &position,
            config.mutation.meat_vision_front_range,
            &food_map,
            &config,
        );
        let meat_vision_left = see_meat(
            &direction_left,
            &position,
            config.mutation.meat_vision_left_range,
            &food_map,
            &config,
        );
        let meat_vision_right = see_meat(
            &direction_right,
            &position,
            config.mutation.meat_vision_right_range,
            &food_map,
            &config,
        );
        let solid_vision_front = see_obstacles(
            &head.direction,
            &position,
            config.mutation.obstacle_vision_front_range,
            &solids_map,
            &config,
        );
        let solid_vision_left = see_obstacles(
            &direction_left,
            &position,
            config.mutation.obstacle_vision_left_range,
            &solids_map,
            &config,
        );
        let solid_vision_right = see_obstacles(
            &direction_right,
            &position,
            config.mutation.obstacle_vision_right_range,
            &solids_map,
            &config,
        );
        let plant_food_level = head.energy.plant_in_stomach / head.metabolism.max_plants_in_stomach;
        let meat_food_level = head.energy.meat_in_stomach / head.metabolism.max_meat_in_stomach;
        let energy_level = head.energy.energy / head.metabolism.max_energy;
        let age_level = age.efficiency_factor;
        head.decision = head.brain.decide(vec![
            bias.clone(),
            chaos,
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
        ]);
    });
}

fn scent(
    scenting_position: &Position,
    scent_map: &Res<ScentMap>,
    config: &Res<SimulationConfig>,
) -> f32 {
    if config.mutation.scent_sensing_enabled {
        let scent = scent_map.map.get(scenting_position);
        scent / 500.0
    } else {
        0.0
    }
}

fn see_meat(
    head_direction: &Direction,
    position: &Position,
    range: u32,
    food_map: &Res<FoodMap>,
    config: &Res<SimulationConfig>,
) -> f32 {
    if config.mutation.meat_vision_enabled {
        let current_vision_position = position;
        let mut current_range = 0;
        while current_range < range {
            let current_vision_position =
                &position_at_direction(head_direction, &current_vision_position, &config).clone();
            if food_map.map.get(current_vision_position).is_meat() {
                return (range - current_range) as f32 / range as f32;
            }
            current_range += 1;
        }
    }
    0.0
}

fn see_plants(
    head_direction: &Direction,
    position: &Position,
    range: u32,
    food_map: &Res<FoodMap>,
    config: &Res<SimulationConfig>,
) -> f32 {
    if config.mutation.plant_vision_enabled {
        let current_vision_position = position;
        let mut current_range = 0;
        while current_range < range {
            let current_vision_position =
                &position_at_direction(head_direction, &current_vision_position, &config).clone();
            if food_map.map.get(current_vision_position).is_plant() {
                return (range - current_range) as f32 / range as f32;
            }
            current_range += 1;
        }
    }
    0.0
}

fn see_obstacles(
    head_direction: &Direction,
    position: &Position,
    range: u32,
    solids_map: &Res<SolidsMap>,
    config: &Res<SimulationConfig>,
) -> f32 {
    if config.mutation.obstacle_vision_enabled {
        let mut current_vision_position = position.clone();
        let mut current_range = 0;
        while current_range < range {
            current_vision_position =
                position_at_direction(head_direction, &current_vision_position, &config).clone();
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
            debug!(
                "Adding scent at position {:?} with energy {}",
                position, meat.amount
            );
            let current_scent = scent_map.map.get_mut(position);
            if current_scent <= &mut 0.0 {
                debug!(
                    "Adding scent at position {:?} with energy {}",
                    position, meat.amount
                );
                commands.spawn((
                    Scent {},
                    Position {
                        x: position.x,
                        y: position.y,
                    },
                ));
            } else {
                debug!(
                    "Scent already there, increasing amount at position {:?} with energy {}",
                    position, meat.amount
                );
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
) {
    let directions = [NorthEast, East, SouthEast, SouthWest, West, NorthWest];
    let mut rng = rand::thread_rng();
    for (_, position) in &scents {
        let random_direction = directions.choose(&mut rng).unwrap();
        let new_position = &position_at_direction(random_direction, &position, &config);
        let diffused_scent = scent_map.map.get(position) * config.scent_diffusion_rate;
        *scent_map.map.get_mut(position) -= diffused_scent;
        let new_scent = scent_map.map.get_mut(new_position);
        if new_scent <= &mut 0.0 {
            debug!(
                "Adding scent throuhg diffusion at position {:?} with energy {}",
                new_position, diffused_scent
            );
            commands.spawn((
                Scent {},
                Position {
                    x: new_position.x,
                    y: new_position.y,
                },
            ));
        } else {
            debug!(
                "Scent already diffused there, increasing amount at position {:?} with energy {}",
                new_position, diffused_scent
            );
        }
        *new_scent += diffused_scent;
        debug!("New scent {}", *new_scent);
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
            debug!(
                "Removing scent at position {:?} with energy {}",
                position, scent
            );
            commands.entity(scent_id).despawn();
            scent_map.map.set(position, 0.0);
        }
    }
}

pub fn create_food(
    mut commands: Commands,
    mut food_map: ResMut<FoodMap>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    let mut rng = rand::thread_rng();
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    for _ in 0..config.food_per_step {
        let x = rng.gen_range(0..columns);
        let y = rng.gen_range(0..rows);
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
    commands: Commands,
    mut food: Query<(Entity, &Position, &Food, &Age)>,
    mut food_map: ResMut<FoodMap>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    for (food_id, postition, food, age) in &mut food {
        if age.age >= 5000 {
            food_map.map.set(postition, Food::default());
        }
    }
}

pub fn eat_food(
    mut snakes: Query<(&Position, &mut Snake)>,
    mut food_map: ResMut<FoodMap>,
    config: Res<SimulationConfig>,
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
        debug!(
            "Snake {:?} has energy {} and plants {} and meat {} in stomach",
            head_id,
            snake.energy.energy,
            snake.energy.plant_in_stomach,
            snake.energy.meat_in_stomach
        );
        if snake.energy.energy < 0.0 {
            debug!("Snake {:?} starved to death", head_id);
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
    debug!("Segment is becoming food now: {:?}", added_food);
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
    let specie = snake.species.unwrap();
    if let Some(specie) = species.species.iter_mut().find(|s| s.id == specie) {
        if specie.leader == head_id {
            specie.members.retain(|s| *s != head_id);
            if let Some(new_leader) = specie.members.pop_front() {
                specie.leader = new_leader;
                specie.leader_network = snake.brain.get_neural_network().unwrap().clone();
                debug!("New leader for specie {:?}: {:?}", specie.id, specie.leader);
            } else {
                let specie_id = specie.id;
                debug!("Specie {:?} is extinct", specie_id);
                species.species.retain(|s| s.id != specie_id);
            }
        } else {
            specie.members.retain(|s| *s != head_id);
            debug!(
                "Snake {:?} died and was removed from specie {:?}",
                head_id, specie.id
            );
        }
    } else {
        warn!("Snake {:?} died and was not found in any specie", head_id);
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
        debug!("Snake {:?} collided with something solid", head_id);
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

fn kill_snake(
    mut commands: &mut Commands,
    positions: &Query<&Position>,
    mut food_map: &mut ResMut<FoodMap>,
    mut species: &mut ResMut<Species>,
    mut solids_map: &mut ResMut<SolidsMap>,
    config: &Res<SimulationConfig>,
    head_id: Entity,
    mut snake: &mut Mut<Snake>,
) {
    commands.entity(head_id).remove::<Snake>();
    remove_snake_from_species(&mut species, head_id, &mut snake);
    for segment_id in &snake.segments {
        remove_segment_and_transform_to_food(
            &mut commands,
            &positions,
            &mut food_map,
            &mut solids_map,
            &config,
            segment_id,
        );
    }
}

pub fn reproduce(
    commands: Commands,
    snakes: Query<(&mut MeatMatter, &Position)>,
    config: Res<SimulationConfig>,
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
    innovation_tracker: ResMut<InnovationTracker>,
) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let snake_length = snake.segments.len();
        if snake_length >= config.size_to_split {
            debug!("Snake splits: {:#?}, {:#?}", snake.metabolism, snake.energy);
            let new_snake_segments = snake.segments.split_off(snake_length / 2);
            let new_head_id = new_snake_segments.first().unwrap();
            let new_head_position = positions.get(*new_head_id).unwrap();
            // new_snake_segments.reverse();
            let mut new_head;
            if let Some(neural_network) = snake.brain.get_neural_network() {
                debug!("Snake {:?} is splitting with neural network", head_id);
                let mut new_neural_network = neural_network.clone();
                let mut rng = rand::thread_rng();
                let mut mutations = snake.mutations;
                if rng.gen_bool(config.mutation.connection_flip_chance) {
                    new_neural_network.flip_random_connection();
                    mutations += 1;
                }
                if rng.gen_bool(config.mutation.weight_perturbation_chance) {
                    new_neural_network.mutate_perturb_random_connection_weight(
                        config.mutation.weight_perturbation_range,
                        config.mutation.perturb_disabled_connections,
                    );
                    mutations += 1;
                }
                if rng.gen_bool(config.mutation.weight_reset_chance) {
                    new_neural_network.mutate_reset_random_connection_weight(
                        config.mutation.weight_reset_range,
                        config.mutation.perturb_reset_connections,
                    );
                    mutations += 1;
                }
                let mut dna = snake.dna.clone();
                if rng.gen_bool(config.mutation.dna_mutation_chance) {
                    dna.mutate();
                    mutations += 1;
                }
                debug!("New neural network: {:?}", new_neural_network);
                new_head = create_head(
                    (new_head_position.x, new_head_position.y),
                    Box::new(RandomNeuralBrain::from_neural_network(
                        new_neural_network.clone(),
                    )),
                    snake.generation + 1,
                    mutations,
                    dna,
                );
                new_head.0.segments = new_snake_segments;
                new_head.0.energy.energy = snake.energy.energy / 2.0;
                snake.energy.energy = snake.energy.energy / 2.0;
                new_head.0.energy.plant_in_stomach = snake.energy.plant_in_stomach / 2.0;
                snake.energy.plant_in_stomach = snake.energy.plant_in_stomach / 2.0;
                new_head.0.energy.meat_in_stomach = snake.energy.meat_in_stomach / 2.0;
                snake.energy.meat_in_stomach = snake.energy.meat_in_stomach / 2.0;
                recalculate_snake_params(&mut snake, &segments, &config, None);
                recalculate_snake_params(&mut new_head.0, &segments, &config, None);
                debug!(
                    "Old snake after split: {:#?}, {:#?}",
                    snake.metabolism, snake.energy
                );
                debug!(
                    "New snake after split: {:#?}, {:#?}",
                    new_head.0.metabolism, new_head.0.energy
                );
                let new_head_id = new_head.0.segments[0];
                if rng.gen_bool(0.5) {
                    new_head.0.direction = turn_left(&snake.direction);
                } else {
                    new_head.0.direction = turn_right(&snake.direction);
                }
                commands.entity(new_head_id).insert(new_head);
                commands.entity(new_head_id).remove::<SegmentType>();
            } else {
                panic!("Snake without neural network");
            }
        }
    }
}

fn recalculate_snake_params(
    snake: &mut Snake,
    segments: &Query<&SegmentType>,
    config: &Res<SimulationConfig>,
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
        match segment {
            SegmentType::Stomach(_) => {
                // TODO: this should come from config
                snake.metabolism.meat_processing_speed += 1.0;
                snake.metabolism.max_meat_in_stomach += 200.0;
            }
            _ => {}
        }
    }
    let len = snake.segments.len() as f32;
    snake.metabolism.mobility = mobility / len;
    snake.metabolism.segment_move_cost += move_cost;
    snake.metabolism.segment_basic_cost += segment_basic_cost;
    snake.metabolism.segment_energy_production += segment_energy_production;
    if let Some(network) = snake.brain.get_neural_network() {
        if network.run_cost() == 0.0 {
            panic!("Neural network run cost is 0.0")
        }
        snake.metabolism.segment_basic_cost += network.run_cost();
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
        if age.efficiency_factor < 1.0 {
            debug!(
                "Snake is getting old, efficiency factor is {}",
                age.efficiency_factor
            );
        }
    }
}
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
        debug!("Snake energy at start: {}", snake.energy.energy);
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
            debug!(
                "Snake ate {} plants and {} meat and now has {} plants and {} meat in stomach",
                eaten_plants,
                eaten_meat,
                snake.energy.plant_in_stomach,
                snake.energy.meat_in_stomach
            );
            let plant_energy_gain =
                eaten_plants * config.plant_energy_content * age.efficiency_factor;
            let meat_energy_gain = eaten_meat * config.meat_energy_content * age.efficiency_factor;
            debug!(
                "Snake energy gain: {} from plants and {} from meat (eff: {}, age: {})",
                plant_energy_gain, meat_energy_gain, age.efficiency_factor, age.age
            );
            snake.energy.energy += plant_energy_gain + meat_energy_gain;
        }
        if snake.energy.energy > 3.0 * snake.metabolism.max_energy / 4.0 {
            snake.energy.accumulated_meat_matter_for_growth +=
                snake.metabolism.meat_matter_for_growth_production_speed;
            snake.energy.energy -= snake.metabolism.meat_matter_for_growth_production_speed
                * config.meat_energy_content;
            debug!("Snake used up {} energy to produce meat matter for growth and has accumulated {} meat matter for growth", snake.metabolism.meat_matter_for_growth_production_speed * config.meat_energy_content, snake.energy.accumulated_meat_matter_for_growth);
        }
        debug!("Snake energy at end: {}", snake.energy.energy);
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
    for (snake_id, mut snake) in &mut snakes {
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
            match segment_type {
                SegmentType::Solid(_) => {
                    commands.entity(new_tail).insert(Solid {});
                }
                _ => {}
            }
            snake.segments.push(new_tail);
            recalculate_snake_params(&mut snake, &segments, &config, Some(&segment_type));
        }
    }
}

pub fn assign_missing_segments(mut snakes: Query<(Entity, &mut Snake), Added<Snake>>) {
    puffin::profile_function!();
    for (snake_id, mut snake) in &mut snakes {
        if snake.segments.len() == 0 {
            snake.segments.push(snake_id);
        }
    }
}

pub fn assign_solid_positions(
    mut solids: Query<(&Position, &Solid)>,
    mut solids_map: ResMut<SolidsMap>,
    segment_map: Res<SegmentMap>,
    config: Res<SimulationConfig>,
) {
    puffin::profile_function!();
    solids_map.map.clear();
    for (position, _) in &mut solids {
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
            if let Ok([(snake_id, mut snake), (leader_id, leader_snake)]) =
                snakes.get_many_mut([baby_id, specie.leader])
            {
                let compatibility = calculate_gene_difference(
                    &leader_snake.brain.get_neural_network().unwrap(),
                    &snake.brain.get_neural_network().unwrap(),
                );
                debug!("Difference: {}", compatibility);
                if compatibility < config.species_threshold {
                    debug!("Snake {:?} is in specie {:?}", snake_id, specie.id);
                    snake.species = Some(specie.id);
                    specie.members.push_back(snake_id);
                    break;
                }
            } else {
                if baby_id != specie.leader {
                    panic!(
                        "Unable to find leader {:?} for baby {:?} for specie {:?}",
                        specie.leader, baby_id, specie.id
                    );
                }
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
            debug!("Snake {:?} is a new specie: {}", baby_id, species.last_id);
        }
    }
}

fn calculate_gene_difference(leader: &NeuralNetwork, new_snake: &NeuralNetwork) -> f32 {
    let leader_genes = leader
        .connections
        .iter()
        .filter(|c| c.enabled)
        .map(|c| c)
        .collect::<Vec<&ConnectionGene>>();
    let new_snake_genes = new_snake
        .connections
        .iter()
        .filter(|c| c.enabled)
        .map(|c| c)
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
    debug!(
        "Matching genes: {}, max genes: {}, gene difference: {}, weight difference: {}",
        matching_genes_count, max_genes, gene_difference, weight_difference
    );
    0.6 * gene_difference + 0.4 * weight_difference
}
pub fn create_snake(
    meat_matter: f32,
    position: (i32, i32),
    brain: Box<dyn Brain>,
    dna: Dna,
) -> (Position, MeatMatter, Snake, Age, JustBorn) {
    if brain.get_neural_network().is_none() {
        panic!("Brain without neural network");
    }
    let (head, age, just_born) = create_head(position, brain, 0, 0, dna);
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
    brain: Box<dyn Brain>,
    generation: u32,
    mutations: u32,
    dna: Dna,
) -> (Snake, Age, JustBorn) {
    (
        Snake {
            direction: Direction::random(),
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
