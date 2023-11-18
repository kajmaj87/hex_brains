use rand::prelude::SliceRandom;
use std::cell::RefCell;
use std::collections::{HashMap, LinkedList, VecDeque};
use bevy_ecs::prelude::*;
use std::clone::Clone;
use bevy_ecs::query::QueryParIter;
use tracing::{debug, info};
use crate::neural::{ConnectionGene, InnovationTracker, NeuralNetwork, SensorInput};
use crate::simulation::{SimulationConfig, Stats};
use rand::Rng;
use crate::core::Direction::{East, NorthEast, NorthWest, SouthEast, SouthWest, West};
use crate::dna::{Dna, SegmentType};

#[derive(Component, Clone, Default)]
#[derive(Debug)]
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

#[derive(Debug)]
pub enum Decision {
    MoveForward,
    MoveLeft,
    MoveRight,
    Wait,
}

pub trait Brain: Sync + Send {
    fn decide(&self, sensory_input: Vec<SensorInput>) -> Decision;
    fn get_neural_network(&self) -> Option<&NeuralNetwork>;
}

// Snake represents the head segment of snake and info about its other segments
#[derive(Debug)]
#[derive(Clone)]
pub struct Specie {
    pub id: u32,
    pub leader: Entity,
    pub members: VecDeque<Entity>,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct Species {
    pub last_id: u32,
    pub species: Vec<Specie>,
}

#[derive(Component)]
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
    pub move_potential: f32,
    pub segment_move_cost: f32,
    pub segment_basic_cost: f32,
    pub mobility: f32,
    pub segment_energy_production: f32,
    // f32 so we can track if more segments are added
    pub meat_processing_speed: f32,
    pub plant_processing_speed: f32,
    pub meat_in_stomach: f32,
    pub plant_in_stomach: f32,
    pub max_plants_in_stomach: f32,
    pub max_meat_in_stomach: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub accumulated_meat_matter_for_growth: f32,
    pub meat_matter_for_growth_production_speed: f32,
}

#[derive(Component)]
pub struct Solid;

#[derive(Component)]
pub struct JustBorn;

pub struct RandomBrain;

pub struct RandomNeuralBrain {
    neural_network: NeuralNetwork,
}

#[derive(Component)]
pub struct Age {
    pub age: u32,
    pub efficiency_factor: f32,
}

impl Brain for RandomBrain {
    fn decide(&self, _: Vec<SensorInput>) -> Decision {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..=3) {
            0 => Decision::MoveForward,
            1 => Decision::MoveLeft,
            2 => Decision::MoveRight,
            _ => Decision::Wait
        }
    }

    fn get_neural_network(&self) -> Option<&NeuralNetwork> {
        None
    }
}

impl RandomNeuralBrain {
    pub(crate) fn new(innovation_tracker: &mut InnovationTracker) -> Self {
        let neural_network = NeuralNetwork::random_brain(14, 0.5, innovation_tracker);
        Self {
            neural_network
        }
    }
    pub(crate) fn from_neural_network(neural_network: NeuralNetwork) -> Self {
        Self {
            neural_network
        }
    }
}

impl Brain for RandomNeuralBrain {
    fn decide(&self, sensor_input: Vec<SensorInput>) -> Decision {
        debug!("Neural network input: {:?}", sensor_input);
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
            _ => Decision::Wait
        };
        debug!("Network architecture: {:?}", self.neural_network.get_active_connections());
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
        self.map = vec![vec![]; self.width * self.height];
    }
}

#[derive(Component)]
pub struct MeatMatter {
    pub(crate) amount: f32,
}

#[derive(Component, Default)]
#[derive(Clone)]
pub struct Food {
    pub plant: f32,
    pub meat: f32,
}

impl Food {
    pub fn from_plant(plant: f32) -> Self {
        Self {
            plant,
            meat: 0.0,
        }
    }

    pub fn from_meat(meat: f32) -> Self {
        Self {
            plant: 0.0,
            meat,
        }
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
        if snake.move_potential < 1.0 {
            snake.move_potential += snake.mobility * age.efficiency_factor;
        }
    }
}

// This system moves each entity with a Position and Velocity component
pub fn movement(mut snakes: Query<(Entity, &mut Snake, &Position, &Age)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();

    for (_, mut snake, head_position, age) in &mut snakes {
        if snake.move_potential >= 1.0 {
            let move_cost = snake.segment_move_cost / age.efficiency_factor;
            match snake.decision {
                Decision::MoveForward => {
                    snake.energy -= move_cost;
                    let new_position = position_at_direction(&snake.direction, &head_position, &config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::MoveLeft => {
                    snake.energy -= move_cost;
                    snake.direction = turn_left(&snake.direction);
                    let new_position = position_at_direction(&snake.direction, &head_position, &config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::MoveRight => {
                    snake.energy -= move_cost;
                    snake.direction = turn_right(&snake.direction);
                    let new_position = position_at_direction(&snake.direction, &head_position, &config);
                    snake.new_position.0 = new_position.x;
                    snake.new_position.1 = new_position.y;
                }
                Decision::Wait => {}
            }
            snake.move_potential -= 1.0;
        }
        if age.efficiency_factor < 1.0 {
            debug!("Removing more energy for thinking: {}", snake.segment_basic_cost / age.efficiency_factor);
        }
        snake.energy -= snake.segment_basic_cost / age.efficiency_factor;
        snake.energy += snake.segment_energy_production * age.efficiency_factor;
    }
}

#[derive(Component)]
pub struct DiedFromCollision {}

pub fn update_positions(mut commands: Commands, mut positions: Query<&mut Position>, mut snakes: Query<(Entity, &mut Snake)>, mut solids_map: ResMut<SolidsMap>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let new_position = snake.new_position;
        let last_position = positions.get_mut(*snake.segments.last().unwrap()).unwrap().clone();
        let mut head_position = positions.get_mut(head_id).unwrap();
        debug!("Snake {:?} with {} segements is moving from {:?} to {:?} (last tail position: {:?})", head_id, snake.segments.len(), head_position, new_position, snake.last_position);
        let old_head_position = head_position.clone();
        if new_position == old_head_position.as_pair() {
            debug!("Snake is not moving");
            continue;
        }
        if *solids_map.map.get(&Position { x: new_position.0, y: new_position.1 }) {
            debug!("Snake has hit something, he will soon die");
            commands.entity(head_id).insert(DiedFromCollision {});
        }
        update_segment_positions(&mut positions, Position { x: new_position.0, y: new_position.1 }, &snake.segments);
        debug!("Removing snake head {:?} from position {:?}", head_id, old_head_position);
        snake.last_position = last_position.as_pair();
    }
}

fn update_segment_positions(mut positions: &mut Query<&mut Position>, new_position: Position, segments: &Vec<Entity>) {
    let mut new_position = new_position.clone();
    for segment in segments {
        let mut position = positions.get_mut(*segment).unwrap();
        let old_position = position.clone();
        debug!("Updating segment {:?} to position {:?} to position {:?}", segment, position, new_position);
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

fn position_at_direction(direction: &Direction, position: &Position, config: &Res<SimulationConfig>) -> Position {
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

pub fn think(mut heads: Query<(&Position, &mut Snake)>, food_map: Res<FoodMap>, solids_map: Res<SolidsMap>, scent_map: Res<ScentMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    let bias = SensorInput { value: 1.0, index: 0 };
    heads.par_iter_mut().for_each_mut(|(position, mut head)| {
        let mut rng = rand::thread_rng();
        let chaos = if config.mutation.chaos_input_enabled {
            SensorInput { value: rng.gen_range(0.0..1.0), index: 1 }
        } else {
            SensorInput { value: 0.0, index: 1 }
        };
        let direction_left = turn_left(&head.direction);
        let direction_right = turn_right(&head.direction);
        let scent_front;
        let scent_left;
        let scent_right;
        if config.mutation.scent_sensing_enabled {
            scent_front = scent(&position_at_direction(&head.direction, &position, &config), &scent_map, 2);
            scent_left = scent(&position_at_direction(&direction_left, &position, &config), &scent_map, 3);
            scent_right = scent(&position_at_direction(&direction_right, &position, &config), &scent_map, 4);
        } else {
            scent_front = SensorInput { value: 0.0, index: 2 };
            scent_left = SensorInput { value: 0.0, index: 3 };
            scent_right = SensorInput { value: 0.0, index: 4 };
        }
        let plant_vision_front;
        let plant_vision_left;
        let plant_vision_right;
        if config.mutation.plant_vision_enabled {
            plant_vision_front = see_plants(&head.direction, &position, config.mutation.plant_vision_front_range, &food_map, &config, 5);
            plant_vision_left = see_plants(&direction_left, &position, config.mutation.plant_vision_left_range, &food_map, &config, 6);
            plant_vision_right = see_plants(&direction_right, &position, config.mutation.plant_vision_right_range, &food_map, &config, 7);
        } else {
            plant_vision_front = SensorInput { value: 0.0, index: 5 };
            plant_vision_left = SensorInput { value: 0.0, index: 6 };
            plant_vision_right = SensorInput { value: 0.0, index: 7 };
        }
        let meat_vision_front;
        let meat_vision_left;
        let meat_vision_right;
        if config.mutation.meat_vision_enabled {
            meat_vision_front = see_meat(&head.direction, &position, config.mutation.meat_vision_front_range, &food_map, &config, 8);
            meat_vision_left = see_meat(&direction_left, &position, config.mutation.meat_vision_left_range, &food_map, &config, 9);
            meat_vision_right = see_meat(&direction_right, &position, config.mutation.meat_vision_right_range, &food_map, &config, 10);
        } else {
            meat_vision_front = SensorInput { value: 0.0, index: 5 };
            meat_vision_left = SensorInput { value: 0.0, index: 6 };
            meat_vision_right = SensorInput { value: 0.0, index: 7 };
        }
        let solid_vision_front;
        let solid_vision_left;
        let solid_vision_right;
        if config.mutation.obstacle_vision_enabled {
            solid_vision_front = see_obstacles(&head.direction, &position, config.mutation.obstacle_vision_front_range, &solids_map, &config, 11);
            solid_vision_left = see_obstacles(&direction_left, &position, config.mutation.obstacle_vision_left_range, &solids_map, &config, 12);
            solid_vision_right = see_obstacles(&direction_right, &position, config.mutation.obstacle_vision_right_range, &solids_map, &config, 13);
        } else {
            solid_vision_front = SensorInput { value: 0.0, index: 8 };
            solid_vision_left = SensorInput { value: 0.0, index: 9 };
            solid_vision_right = SensorInput { value: 0.0, index: 10 };
        }
        head.decision = head.brain.decide(vec![bias.clone(), chaos, scent_front, scent_left, scent_right, plant_vision_front, plant_vision_left, plant_vision_right, meat_vision_front, meat_vision_left, meat_vision_right, solid_vision_front, solid_vision_left, solid_vision_right]);
    });
}

fn scent(scenting_position: &Position, scent_map: &Res<ScentMap>, index: usize) -> SensorInput {
    let scent = scent_map.map.get(scenting_position);
    SensorInput { value: scent / 500.0, index }
}

fn see_meat(head_direction: &Direction, position: &Position, range: u32, food_map: &Res<FoodMap>, config: &Res<SimulationConfig>, index: usize) -> SensorInput {
    let current_vision_position = position;
    let mut current_range = 0;
    while current_range < range {
        let current_vision_position = &position_at_direction(head_direction, &current_vision_position, &config).clone();
        if food_map.map.get(current_vision_position).is_meat() {
            return SensorInput { value: (range - current_range) as f32 / range as f32, index };
        }
        current_range += 1;
    }
    SensorInput { value: 0.0, index }
}
fn see_plants(head_direction: &Direction, position: &Position, range: u32, food_map: &Res<FoodMap>, config: &Res<SimulationConfig>, index: usize) -> SensorInput {
    let current_vision_position = position;
    let mut current_range = 0;
    while current_range < range {
        let current_vision_position = &position_at_direction(head_direction, &current_vision_position, &config).clone();
        if food_map.map.get(current_vision_position).is_plant() {
            return SensorInput { value: (range - current_range) as f32 / range as f32, index };
        }
        current_range += 1;
    }
    SensorInput { value: 0.0, index }
}

fn see_obstacles(head_direction: &Direction, position: &Position, range: u32, solids_map: &Res<SolidsMap>, config: &Res<SimulationConfig>, index: usize) -> SensorInput {
    let mut current_vision_position = position.clone();
    let mut current_range = 0;
    while current_range < range {
        current_vision_position = position_at_direction(head_direction, &current_vision_position, &config).clone();
        if *solids_map.map.get(&current_vision_position) {
            return SensorInput { value: (range - current_range) as f32 / range as f32, index };
        }
        current_range += 1;
    }
    SensorInput { value: 0.0, index }
}


pub fn add_scents(mut commands: Commands, scent_source: Query<(&MeatMatter, &Position)>, mut scent_map: ResMut<ScentMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    if config.create_scents {
        for (meat, position) in &scent_source {
            debug!("Adding scent at position {:?} with energy {}", position, meat.amount);
            let mut current_scent = scent_map.map.get_mut(position);
            if current_scent <= &mut 0.0 {
                debug!("Adding scent at position {:?} with energy {}", position, meat.amount);
                commands.spawn((Scent {}, Position { x: position.x, y: position.y }));
            } else {
                debug!("Scent already there, increasing amount at position {:?} with energy {}", position, meat.amount);
            }
            if *current_scent < 1000.0 {
                *current_scent += meat.amount;
            }
        }
    }
}

pub fn diffuse_scents(mut commands: Commands, scents: Query<(&Scent, &Position)>, mut scent_map: ResMut<ScentMap>, config: Res<SimulationConfig>) {
    let directions = [NorthEast, East, SouthEast, SouthWest, West, NorthWest];
    let mut rng = rand::thread_rng();
    for (_, position) in &scents {
        let random_direction = directions.choose(&mut rng).unwrap();
        let new_position = &position_at_direction(random_direction, &position, &config);
        let diffused_scent = scent_map.map.get(position) * config.scent_diffusion_rate;
        *scent_map.map.get_mut(position) -= diffused_scent;
        let mut new_scent = scent_map.map.get_mut(new_position);
        if new_scent <= &mut 0.0 {
            debug!("Adding scent throuhg diffusion at position {:?} with energy {}", new_position, diffused_scent);
            commands.spawn((Scent {}, Position { x: new_position.x, y: new_position.y }));
        } else {
            debug!("Scent already diffused there, increasing amount at position {:?} with energy {}", new_position, diffused_scent);
        }
        *new_scent += diffused_scent;
        debug!("New scent {}", *new_scent);
    }
}

pub fn disperse_scents(mut commands: Commands, scents: Query<(Entity, &Scent, &Position)>, mut scent_map: ResMut<ScentMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (scent_id, _, position) in &scents {
        let mut scent = scent_map.map.get_mut(position);
        *scent -= config.scent_dispersion_per_step;
        if scent <= &mut 0.0 {
            debug!("Removing scent at position {:?} with energy {}", position, scent);
            commands.entity(scent_id).despawn();
            scent_map.map.set(position, 0.0);
        }
    }
}

pub fn create_food(mut commands: Commands, mut food_map: ResMut<FoodMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    let mut rng = rand::thread_rng();
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    for _ in 0..config.food_per_step {
        let x = rng.gen_range(0..columns);
        let y = rng.gen_range(0..rows);
        let mut food = food_map.map.get_mut(&Position { x, y });
        if !food.contains_food() {
            commands.spawn((Position { x, y }, Food { plant: config.energy_per_segment, meat: 0.0 }, Age { age: 0, efficiency_factor: 1.0 }));
        }
        *food = Food::from_plant(config.energy_per_segment);
    }
}

pub fn destroy_old_food(mut commands: Commands, mut food: Query<(Entity, &Position, &Food, &Age)>, mut food_map: ResMut<FoodMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (food_id, postition, food, age) in &mut food {
        if age.age >= 5000 {
            food_map.map.set(postition, Food::default());
        }
    }
}

pub fn eat_food(mut snakes: Query<(&Position, &mut Snake)>, mut food_map: ResMut<FoodMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (position, mut snake) in &mut snakes {
        let food = food_map.map.get_mut(position);
        if snake.plant_processing_speed > 0.0 && food.plant + snake.plant_in_stomach <= snake.max_plants_in_stomach {
            snake.plant_in_stomach += food.plant;
            food.plant = 0.0;
        }
        if snake.meat_processing_speed > 0.0 && food.meat + snake.meat_in_stomach <= snake.max_meat_in_stomach {
            snake.meat_in_stomach += food.meat;
            food.meat = 0.0;
        }
    }
}

pub fn despawn_food(mut commands: Commands, food: Query<(Entity, &Position, &Food)>, mut food_map: ResMut<FoodMap>) {
    puffin::profile_function!();
    for (food_id, position, _) in &food {
        if !food_map.map.get(position).contains_food() {
            commands.entity(food_id).despawn();
        }
    }
}

pub fn starve(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake)>, positions: Query<&Position>, mut food_map: ResMut<FoodMap>, mut species: ResMut<Species>, mut solids_map: ResMut<SolidsMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        info!("Snake {:?} has energy {} and plants {} and meat {} in stomach", head_id, snake.energy, snake.plant_in_stomach, snake.meat_in_stomach);
        if snake.energy < 0.0 {
            info!("Snake {:?} starved to death", head_id);
            snake.segments.iter().for_each(|segment_id| {
                commands.entity(*segment_id).despawn();
                let position = positions.get(*segment_id).unwrap();
                solids_map.map.set(position, false);
                food_map.map.set(position, Food::from_meat(config.new_segment_cost * config.meat_energy_content));
                commands.spawn((position.clone(), Food::from_meat(config.energy_per_segment * 5.0), Age { age: 0, efficiency_factor: 1.0 }));
            });
            remove_snake_from_species(&mut species, head_id, &mut snake);
            continue;
        }
    }
}

fn adjust_snake_params_after_starve(mut snake: &mut Snake, segment_type: &SegmentType) {
    let len = snake.segments.len() as f32;
    snake.mobility = (snake.mobility * len - segment_type.mobility()) / (len - 1.0);
    snake.segment_move_cost -= segment_type.energy_cost_move();
    if segment_type.energy_cost_always() > 0.0 {
        snake.segment_basic_cost -= segment_type.energy_cost_always();
    } else {
        snake.segment_energy_production += segment_type.energy_cost_always();
    }
    match segment_type {
        SegmentType::Stomach(_) => {
            snake.meat_processing_speed -= 1.0;
            snake.max_meat_in_stomach -= 200.0;
        }
        _ => {}
    }
}

fn remove_snake_from_species(species: &mut ResMut<Species>, head_id: Entity, snake: &mut Mut<Snake>) {
    let specie = snake.species.unwrap();
    let mut specie = species.species.iter_mut().find(|s| s.id == specie).unwrap();
    if specie.leader == head_id {
        specie.members.retain(|s| *s != head_id);
        if let Some(new_leader) = specie.members.pop_front() {
            specie.leader = new_leader;
            debug!("New leader for specie {:?}: {:?}", specie.id, specie.leader);
        } else {
            let specie_id = specie.id;
            debug!("Specie {:?} is extinct", specie_id);
            species.species.retain(|s| s.id != specie_id);
        }
    } else {
        specie.members.retain(|s| *s != head_id);
        debug!("Snake {:?} died and was removed from specie {:?}", head_id, specie.id);
    }
}

pub fn die_from_collisions(mut commands: Commands, positions: Query<&Position>, mut snake: Query<(Entity, &mut Snake, &DiedFromCollision)>, mut food_map: ResMut<FoodMap>, mut species: ResMut<Species>, mut solids_map: ResMut<SolidsMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (head_id, mut snake, _) in &mut snake {
        debug!("Snake {:?} collided with something solid", head_id);
        commands.entity(head_id).remove::<Snake>();
        commands.entity(head_id).remove::<Solid>();
        let solid_position = positions.get(head_id).unwrap();
        solids_map.map.set(solid_position, false);
        food_map.map.set(solid_position, Food::from_meat(config.energy_per_segment * 5.0));
        commands.entity(head_id).insert((Food::from_meat(config.energy_per_segment * 5.0), Age { age: 0, efficiency_factor: 1.0 }));
        remove_snake_from_species(&mut species, head_id, &mut snake);
        for segment_id in &snake.segments {
            commands.entity(*segment_id).remove::<Solid>();
            commands.entity(*segment_id).insert((Food::from_meat(config.energy_per_segment * 5.0), Age { age: 0, efficiency_factor: 1.0 }));
            let solid_position = positions.get(*segment_id).unwrap();
            solids_map.map.set(solid_position, false);
            food_map.map.set(solid_position, Food::from_meat(config.energy_per_segment * 5.0));
        }
    }
}

pub fn reproduce(mut commands: Commands, mut snakes: Query<(&mut MeatMatter, &Position)>, config: Res<SimulationConfig>) {
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

pub fn split(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake)>, segments: Query<&SegmentType>, positions: Query<&Position>, config: Res<SimulationConfig>, mut innovation_tracker: ResMut<InnovationTracker>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let snake_length = snake.segments.len();
        if snake_length >= config.size_to_split {
            let mut new_snake_segments = snake.segments.split_off(snake_length / 2);
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
                    new_neural_network.mutate_random_connection_weight(config.mutation.weight_perturbation_range);
                    mutations += 1;
                }
                let mut dna = snake.dna.clone();
                if rng.gen_bool(config.mutation.dna_mutation_chance) {
                    dna.mutate();
                    mutations += 1;
                }
                debug!("New neural network: {:?}", new_neural_network);
                new_head = create_head((new_head_position.x, new_head_position.y), Box::new(RandomNeuralBrain::from_neural_network(new_neural_network.clone())), snake.generation + 1, mutations, dna);
                new_head.0.segments = new_snake_segments;
                new_head.0.energy = snake.energy / 2.0;
                snake.energy = snake.energy / 2.0;
                recalculate_snake_params(&mut new_head.0, &segments);
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

fn recalculate_snake_params(snake: &mut Snake, segments: &Query<&SegmentType>) {
    let mut mobility = 0.0;
    let mut move_cost = 0.0;
    let mut always_cost = 0.0;
    for segment_id in &snake.segments {
        let segment = segments.get(*segment_id).unwrap();
        mobility += segment.mobility();
        move_cost += segment.energy_cost_move();
        always_cost += segment.energy_cost_always();
        match segment {
            SegmentType::Stomach(_) => {
                snake.meat_processing_speed += 1.0;
                snake.max_meat_in_stomach += 200.0;
            }
            _ => {}
        }
    }
    let len = snake.segments.len() as f32;
    snake.mobility = mobility / len;
    // take old value, those come from head
    snake.segment_move_cost += move_cost;
    snake.segment_basic_cost += always_cost;
}

pub fn increase_age(mut agables: Query<&mut Age>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for mut age in &mut agables {
        age.age += 1;
        age.efficiency_factor = (1.0 / (age.age as f32 / config.snake_max_age as f32)).min(1.0);
        if age.efficiency_factor < 1.0 {
            debug!("Snake is getting old, efficiency factor is {}", age.efficiency_factor);
        }
    }
}

pub fn calculate_stats(entities: Query<Entity>, scents: Query<&Scent>, food: Query<&Food>, snakes: Query<(&Snake, &Age)>, segments: Query<&SegmentType>, mut stats: ResMut<Stats>, species: Res<Species>) {
    puffin::profile_function!();
    let max_age = snakes.iter().map(|(_, a)| a.age).reduce(|a, b| a.max(b));
    let max_generation = snakes.iter().map(|(s, _)| s.generation).reduce(|a, b| a.max(b));
    let max_mutation = snakes.iter().map(|(s, _)| s.mutations).reduce(|a, b| a.max(b));
    stats.oldest_snake = max_age.unwrap_or(0);
    stats.total_snakes = snakes.iter().count();
    stats.total_food = food.iter().count();
    stats.total_segments = segments.iter().count();
    stats.total_scents = scents.iter().count();
    stats.max_generation = max_generation.unwrap_or(0);
    stats.max_mutations = max_mutation.unwrap_or(0);
    stats.species = species.clone();
    stats.total_entities = entities.iter().count();
}

pub fn process_food(mut snake: Query<&mut Snake>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for mut snake in &mut snake {
        debug!("Snake energy at start: {}", snake.energy);
        if snake.energy < snake.max_energy {
            debug!("Snake is processing food");
            let eaten_plants = snake.plant_in_stomach.min(snake.plant_processing_speed);
            snake.plant_in_stomach -= eaten_plants;
            let eaten_meat = snake.meat_in_stomach.min(snake.meat_processing_speed);
            snake.meat_in_stomach -= eaten_meat;
            snake.energy += eaten_plants * config.plant_energy_content + eaten_meat * config.meat_energy_content;
        }
        if snake.energy > 3.0 * snake.max_energy / 4.0 {
            debug!("Snake is processing food for growth");
            snake.accumulated_meat_matter_for_growth += snake.meat_matter_for_growth_production_speed;
            snake.energy -= snake.meat_matter_for_growth_production_speed * config.meat_energy_content;
        }
        debug!("Snake energy at end: {}", snake.energy);
    }
}

pub fn grow(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake)>, segment_map: Res<SegmentMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (snake_id, mut snake) in &mut snakes {
        // tail always takes energy from head when growing
        let position_empty = segment_map.map.get(&Position { x: snake.last_position.0, y: snake.last_position.1 }).is_empty();
        if position_empty && snake.accumulated_meat_matter_for_growth >= config.new_segment_cost {
            let meat_for_tail = config.new_segment_cost;
            snake.accumulated_meat_matter_for_growth -= meat_for_tail;
            let segment_type = snake.dna.build_segment();
            let new_tail = commands.spawn((segment_type.clone(), Position { x: snake.last_position.0, y: snake.last_position.1 }, MeatMatter { amount: meat_for_tail })).id();
            match segment_type {
                SegmentType::Solid(_) => {
                    commands.entity(new_tail).insert(Solid {});
                }
                _ => {}
            }
            adjust_snake_params_after_grow(&mut snake, &segment_type);
            snake.segments.push(new_tail);
        }
    }
}

fn adjust_snake_params_after_grow(mut snake: &mut Snake, segment_type: &SegmentType) {
    let len = snake.segments.len() as f32;
    snake.mobility = (snake.mobility * len + segment_type.mobility()) / (len + 1.0);
    snake.segment_move_cost += segment_type.energy_cost_move();
    if segment_type.energy_cost_always() > 0.0 {
        snake.segment_basic_cost += segment_type.energy_cost_always();
    } else {
        snake.segment_energy_production -= segment_type.energy_cost_always();
    }
    match segment_type {
        SegmentType::Stomach(_) => {
            snake.meat_processing_speed += 1.0;
            snake.max_meat_in_stomach += 200.0;
        }
        _ => {}
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

pub fn assign_solid_positions(mut solids: Query<(&Position, &Solid)>, mut solids_map: ResMut<SolidsMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    solids_map.map.clear();
    for (position, _) in &mut solids {
        solids_map.map.set(position, true);
    }
}

pub fn assign_segment_positions(mut segment_map: ResMut<SegmentMap>, segments: Query<(Entity, &Position, &SegmentType)>) {
    puffin::profile_function!();
    segment_map.map.clear();
    for (segment_id, position, _) in &segments {
        segment_map.map.add(position, segment_id);
    }
}

pub fn assign_species(new_borns: Query<Entity, Added<JustBorn>>, mut snakes: Query<(Entity, &mut Snake)>, mut species: ResMut<Species>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for baby_id in &new_borns {
        // let mut baby_snake = None;
        for specie in species.species.iter_mut() {
            if let Ok([(snake_id, mut snake), (leader_id, leader_snake)]) = snakes.get_many_mut([baby_id, specie.leader]) {
                let compatibility = calculate_gene_difference(&leader_snake.brain.get_neural_network().unwrap(), &snake.brain.get_neural_network().unwrap());
                debug!("Difference: {}", compatibility);
                if compatibility < config.species_threshold {
                    debug!("Snake {:?} is in specie {:?}", snake_id, specie.id);
                    snake.species = Some(specie.id);
                    specie.members.push_back(snake_id);
                    break;
                }
            } else {
                if baby_id != specie.leader {
                    panic!("Unable to find leader {:?} for baby {:?} for specie {:?}", specie.leader, baby_id, specie.id);
                }
            }
        }
        let (_, mut baby_snake) = snakes.get_mut(baby_id).unwrap();
        if baby_snake.species.is_none() {
            let mut new_specie = Specie { id: species.last_id + 1, leader: baby_id, members: VecDeque::new() };
            new_specie.members.push_back(baby_id);
            species.species.push(new_specie);
            species.last_id += 1;
            baby_snake.species = Some(species.last_id);
            debug!("Snake {:?} is a new specie: {}", baby_id, species.last_id);
        }
    }
}

fn calculate_gene_difference(leader: &NeuralNetwork, new_snake: &NeuralNetwork) -> f32 {
    let leader_genes = leader.connections.iter().filter(|c| c.enabled).map(|c| c).collect::<Vec<&ConnectionGene>>();
    let new_snake_genes = new_snake.connections.iter().filter(|c| c.enabled).map(|c| c).collect::<Vec<&ConnectionGene>>();
    let leader_innovations: Vec<_> = leader_genes.iter().map(|c| c.innovation_number).collect();
    let new_snake_innovations: Vec<_> = new_snake_genes.iter().map(|c| c.innovation_number).collect();
    // genes that both have in common
    let matching_innovations: Vec<_> = leader_innovations.iter().filter(|i| new_snake_innovations.contains(i)).collect();
    let matching_genes_count = matching_innovations.len();
    // calculate weight difference on matching genes
    let mut weight_difference = 0.0;
    for innovation in matching_innovations {
        let leader_weight = leader.connections.iter().find(|c| c.innovation_number == *innovation).unwrap().weight;
        let new_snake_weight = new_snake.connections.iter().find(|c| c.innovation_number == *innovation).unwrap().weight;
        weight_difference += (leader_weight - new_snake_weight).abs();
    }
    // max number of genes
    let max_genes = leader_genes.len().max(new_snake_genes.len());
    let gene_difference = (max_genes - matching_genes_count) as f32 / max_genes as f32;
    let weight_difference = weight_difference / matching_genes_count as f32;
    debug!("Matching genes: {}, max genes: {}, gene difference: {}, weight difference: {}", matching_genes_count, max_genes, gene_difference, weight_difference);
    0.6 * gene_difference + 0.4 * weight_difference
}

pub fn create_snake(meat_matter: f32, position: (i32, i32), brain: Box<dyn Brain>, dna: Dna) -> (Position, MeatMatter, Snake, Age, JustBorn) {
    let (head, age, just_born) = create_head(position, brain, 0, 0, dna);
    (Position { x: position.0, y: position.1 }, MeatMatter { amount: meat_matter }, head, age, just_born)
}

fn create_head(position: (i32, i32), brain: Box<dyn Brain>, generation: u32, mutations: u32, dna: Dna) -> (Snake, Age, JustBorn) {
    (Snake {
        direction: West,
        decision: Decision::Wait,
        brain,
        new_position: position,
        segments: vec![],
        last_position: position,
        generation,
        mutations,
        species: None,
        dna,
        mobility: 1.0,
        move_potential: -2.0,
        segment_move_cost: 1.0,
        segment_basic_cost: 1.0,
        segment_energy_production: 0.0,
        meat_processing_speed: 0.0,
        plant_processing_speed: 25.0,
        meat_in_stomach: 0.0,
        plant_in_stomach: 0.0,
        max_plants_in_stomach: 200.0,
        max_meat_in_stomach: 0.0,
        energy: 100.0,
        max_energy: 400.0,
        accumulated_meat_matter_for_growth: 0.0,
        meat_matter_for_growth_production_speed: 5.0,
    }, Age { age: 0, efficiency_factor: 1.0 }, JustBorn)
}