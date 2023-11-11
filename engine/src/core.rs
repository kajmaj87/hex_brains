use std::collections::LinkedList;
use rand::Rng;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryParIter;
use tracing::debug;
use crate::neural::{InnovationTracker, NeuralNetwork, SensorInput};
use crate::simulation::{SimulationConfig, Stats};

#[derive(Component)]
pub struct Position {
    pub x: i32,
    pub y: i32,
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
#[derive(Component)]
pub struct Snake {
    pub direction: Direction,
    pub decision: Decision,
    pub brain: Box<dyn Brain>,
    pub new_position: (i32, i32),
    pub last_position: (i32, i32),
    pub segments: Vec<Entity>,
    pub generation: u32,
}

#[derive(Component)]
pub struct Solid;

pub struct RandomBrain;

pub struct RandomNeuralBrain {
    neural_network: NeuralNetwork,
}

#[derive(Component)]
pub struct Age(u32);

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
        let neural_network = NeuralNetwork::random_brain(2, 0.5, innovation_tracker);
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

#[derive(Component)]
pub struct Energy {
    pub(crate) amount: i32,
}

#[derive(Component)]
pub struct Food {}

#[derive(Resource)]
pub struct EntityMap {
    pub map: Vec<Vec<Option<Entity>>>,
}

// This system moves each entity with a Position and Velocity component
pub fn movement(mut snakes: Query<(Entity, &mut Energy, &mut Snake, &Position)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();

    for (_, mut energy, mut snake, head_position) in &mut snakes {
        match snake.decision {
            Decision::MoveForward => {
                energy.amount -= config.move_cost;
                let new_position = move_to_direction(snake.direction.clone(), &head_position, &config);
                snake.new_position.0 = new_position.x;
                snake.new_position.1 = new_position.y;
            }
            Decision::MoveLeft => {
                energy.amount -= config.move_cost;
                snake.direction = turn_left(snake.direction.clone());

                let new_position = move_to_direction(snake.direction.clone(), &head_position, &config);
                snake.new_position.0 = new_position.x;
                snake.new_position.1 = new_position.y;
            }
            Decision::MoveRight => {
                energy.amount -= config.move_cost;
                snake.direction = turn_right(snake.direction.clone());

                let new_position = move_to_direction(snake.direction.clone(), &head_position, &config);
                snake.new_position.0 = new_position.x;
                snake.new_position.1 = new_position.y;
            }
            Decision::Wait => {
                energy.amount -= config.wait_cost;
            }
        }
        // if energy.amount < -10 {
        //     energy.amount = 0;
        // }
    }
}

pub fn update_positions(mut positions: Query<&mut Position>, mut snakes: Query<(Entity, &mut Snake)>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let mut new_position = snake.new_position;
        let mut head_position = positions.get_mut(head_id).unwrap();
        let old_head_position = (head_position.x, head_position.y);
        if new_position == old_head_position {
            continue;
        }
        head_position.x = new_position.0;
        head_position.y = new_position.1;
        if snake.segments.len() >= 2 {
            let tail_id = snake.segments.pop().unwrap();
            let mut tail_position = positions.get_mut(tail_id).unwrap();
            let last_position = (tail_position.x, tail_position.y);
            tail_position.x = old_head_position.0;
            tail_position.y = old_head_position.1;
            // move the snake right behind the head to avoid recalculating all positions
            snake.segments.insert(1, tail_id);
            snake.last_position = last_position;
        }
    }
}

fn turn_left(direction: Direction) -> Direction {
    match direction {
        Direction::NorthEast => Direction::NorthWest,
        Direction::East => Direction::NorthEast,
        Direction::SouthEast => Direction::East,
        Direction::SouthWest => Direction::SouthEast,
        Direction::West => Direction::SouthWest,
        Direction::NorthWest => Direction::West,
    }
}

fn turn_right(direction: Direction) -> Direction {
    match direction {
        Direction::NorthEast => Direction::East,
        Direction::East => Direction::SouthEast,
        Direction::SouthEast => Direction::SouthWest,
        Direction::SouthWest => Direction::West,
        Direction::West => Direction::NorthWest,
        Direction::NorthWest => Direction::NorthEast,
    }
}

fn flip_direction(direction: Direction) -> Direction {
    match direction {
        Direction::NorthEast => Direction::SouthWest,
        Direction::East => Direction::West,
        Direction::SouthEast => Direction::NorthWest,
        Direction::SouthWest => Direction::NorthEast,
        Direction::West => Direction::East,
        Direction::NorthWest => Direction::SouthEast,
    }
}

fn move_to_direction(direction: Direction, position: &Position, config: &Res<SimulationConfig>) -> Position {
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

pub fn think(mut heads: Query<&mut Snake>) {
    puffin::profile_function!();
    let mut rng = rand::thread_rng();
    for mut head in &mut heads {
        head.decision = head.brain.decide(vec![SensorInput{ value: 1.0, index: 0 }, SensorInput{ value: rng.gen_range(0.0..1.0), index: 1 }]);
    }
}

pub fn create_food(mut commands: Commands, mut entities: ResMut<EntityMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    let mut rng = rand::thread_rng();
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    for _ in 0..config.food_per_step {
        let x = rng.gen_range(0..columns);
        let y = rng.gen_range(0..rows);
        if entities.map[x as usize][y as usize].is_none() {
            let entity = commands.spawn((Position { x, y }, Food {})).id();
            entities.map[x as usize][y as usize] = Some(entity);
        }
    }
}

pub fn eat_food(mut commands: Commands, mut food: ResMut<EntityMap>, mut snakes: Query<(&Position, &mut Energy), Without<Food>>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (position, mut energy) in &mut snakes {
        if let Some(food_entity) = food.map[position.x as usize][position.y as usize] {
            commands.entity(food_entity).despawn();
            food.map[position.x as usize][position.y as usize] = None;
            energy.amount += config.energy_per_segment;
        }
    }
}

pub fn starve(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake)>, mut energies: Query<&mut Energy>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let tail_id = snake.segments.last().unwrap();
        if *tail_id == head_id {
            let head_energy = energies.get_mut(head_id).unwrap();
            if head_energy.amount <= 0 {
                commands.entity(head_id).despawn();
            }
        } else {
            if let Ok([mut head_energy, mut tail_energy]) = energies.get_many_mut([head_id, *tail_id]) {
                if head_energy.amount <= 0 {
                    head_energy.amount += tail_energy.amount;
                    commands.entity(*tail_id).despawn();
                    snake.segments.pop();
                }
            }
        }
    }
}

pub fn reproduce(mut commands: Commands, mut snakes: Query<(&mut Energy, &Position)>, config: Res<SimulationConfig>) {
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

pub fn split(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake)>, positions: Query<&Position>, config: Res<SimulationConfig>, mut innovation_tracker: ResMut<InnovationTracker>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let snake_length = snake.segments.len();
        if snake_length >= config.size_to_split {
            let mut new_snake_segments = snake.segments.split_off(snake_length / 2);
            let new_head_id = new_snake_segments.last().unwrap();
            let new_head_position = positions.get(*new_head_id).unwrap();
            new_snake_segments.reverse();
            let mut new_head;
            if let Some(neural_network) = snake.brain.get_neural_network() {
                debug!("Splitting snake with neural network");
                let mut new_neural_network = neural_network.clone();
                let mut rng = rand::thread_rng();
                // if rng.gen_bool(0.1) {
                //     new_neural_network.flip_random_connection();
                // }
                debug!("New neural network: {:?}", new_neural_network);
                new_head = create_head((new_head_position.x, new_head_position.y), Box::new(RandomNeuralBrain::from_neural_network(new_neural_network.clone())), snake.generation + 1);
                new_head.0.segments = new_snake_segments;
                let new_head_id = new_head.0.segments[0];
                new_head.0.direction = flip_direction(snake.direction.clone());
                commands.entity(new_head_id).insert(new_head);
            } else {
                panic!("Snake without neural network");
            }

        }
    }
}

pub fn increase_age(mut agables: Query<&mut Age>) {
    puffin::profile_function!();
    for mut age in &mut agables {
        age.0 += 1;
    }
}

pub fn calculate_stats(food: Query<&Food>, snakes: Query<(&Snake, &Age)>, solids: Query<&Solid>, mut stats: ResMut<Stats>) {
    puffin::profile_function!();
    let max_age = snakes.iter().map(|(_, a)| a.0).reduce(|a, b| a.max(b));
    let max_generation = snakes.iter().map(|(s, _)| s.generation).reduce(|a, b| a.max(b));
    stats.oldest_snake = max_age.unwrap_or(0);
    stats.total_snakes = snakes.iter().count();
    stats.total_food = food.iter().count();
    stats.total_solids = solids.iter().count();
    stats.max_generation = max_generation.unwrap_or(0);
}

pub fn grow(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake, &mut Energy)>, positions: Query<&Position>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (snake_id, mut snake, mut energy) in &mut snakes {
        // tail always takes energy from head when growing
        if energy.amount >= config.energy_to_grow {
            let energy_for_tail = energy.amount / 2;
            energy.amount -= energy_for_tail;
            let new_tail = commands.spawn((Position { x: snake.last_position.0, y: snake.last_position.1 }, Solid, Energy { amount: energy_for_tail })).id();

            snake.segments.push(new_tail);
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

pub fn create_snake(energy: i32, position: (i32, i32), brain: Box<dyn Brain>) -> (Position, Energy, Snake, Age, Solid) {
    let (head, age) = create_head(position, brain, 0);
    (Position { x: position.0, y: position.1 }, Energy { amount: energy }, head, age, Solid)
}

fn create_head(position: (i32, i32), brain: Box<dyn Brain>, generation: u32) -> (Snake, Age) {
    (Snake { direction: Direction::West, decision: Decision::Wait, brain, new_position: position, segments: vec![], last_position: position, generation }, Age(0))
}