use std::cell::RefCell;
use std::collections::{LinkedList, VecDeque};
use rand::Rng;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryParIter;
use tracing::{debug, info};
use crate::neural::{ConnectionGene, InnovationTracker, NeuralNetwork, SensorInput};
use crate::simulation::{SimulationConfig, Stats};

#[derive(Component, Clone)]
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
        let neural_network = NeuralNetwork::random_brain(8, 0.5, innovation_tracker);
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
                let new_position = position_at_direction(&snake.direction, &head_position, &config);
                snake.new_position.0 = new_position.x;
                snake.new_position.1 = new_position.y;
            }
            Decision::MoveLeft => {
                energy.amount -= config.move_cost;
                snake.direction = turn_left(&snake.direction);

                let new_position = position_at_direction(&snake.direction, &head_position, &config);
                snake.new_position.0 = new_position.x;
                snake.new_position.1 = new_position.y;
            }
            Decision::MoveRight => {
                energy.amount -= config.move_cost;
                snake.direction = turn_right(&snake.direction);

                let new_position = position_at_direction(&snake.direction, &head_position, &config);
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

fn flip_direction(direction: &Direction) -> Direction {
    match direction {
        Direction::NorthEast => Direction::SouthWest,
        Direction::East => Direction::West,
        Direction::SouthEast => Direction::NorthWest,
        Direction::SouthWest => Direction::NorthEast,
        Direction::West => Direction::East,
        Direction::NorthWest => Direction::SouthEast,
    }
}

fn position_at_direction(direction: &Direction, position: &Position, config: &Res<SimulationConfig>) -> Position {
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

pub fn think(mut heads: Query<(&Position, &mut Snake)>, foodMap: Res<EntityMap>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    let mut rng = rand::thread_rng();
    let bias = SensorInput { value: 1.0, index: 0 };
    for (position, mut head) in &mut heads {
        let chaos = SensorInput { value: rng.gen_range(0.0..1.0), index: 1 };
        let food_smell_front = sense_food(&position_at_direction(&head.direction, &position, &config), &foodMap, 2);
        let food_smell_left = sense_food(&position_at_direction(&turn_left(&head.direction), &position, &config), &foodMap, 3);
        let food_smell_right = sense_food(&position_at_direction(&turn_right(&head.direction), &position, &config), &foodMap, 4);
        let food_vision_front = see_food(&head.direction, &position, 5, &foodMap, &config, 5);
        let food_vision_left = see_food(&head.direction, &position, 3, &foodMap, &config, 6);
        let food_vision_right = see_food(&head.direction, &position, 3, &foodMap, &config, 7);
        head.decision = head.brain.decide(vec![bias.clone(), chaos, food_smell_front, food_smell_left, food_smell_right, food_vision_front, food_vision_left, food_vision_right]);
    }
}

fn see_food(head_direction: &Direction, position: &Position, range: u32, food_map: &Res<EntityMap>, config: &Res<SimulationConfig>, index: usize) -> SensorInput {
    let mut current_vision_position = position.clone();
    let mut current_range = 0;
    while current_range < range {
        current_vision_position = position_at_direction(head_direction, &current_vision_position, &config).clone();
        if food_map.map[current_vision_position.x as usize][current_vision_position.y as usize].is_some() {
            return SensorInput { value: (range - current_range) as f32 / range as f32, index };
        }
        current_range += 1;
    }
    SensorInput { value: 0.0, index }
}

fn sense_food(position: &Position, foodMap: &Res<EntityMap>, index: usize) -> SensorInput {
    if foodMap.map[position.x as usize][position.y as usize].is_some() {
        SensorInput { value: 1.0, index }
    } else {
        SensorInput { value: 0.0, index }
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

pub fn starve(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake)>, mut energies: Query<&mut Energy>, mut species: ResMut<Species>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let tail_id = snake.segments.last().unwrap();
        if *tail_id == head_id {
            let head_energy = energies.get_mut(head_id).unwrap();
            if head_energy.amount <= 0 {
                commands.entity(head_id).despawn();
                let specie = snake.species.unwrap();
                let mut specie = species.species.iter_mut().find(|s| s.id == specie).unwrap();
                if specie.leader == head_id {
                    specie.members.retain(|s| *s != head_id);
                    if let Some(new_leader) = specie.members.pop_front() {
                        specie.leader = new_leader;
                        debug!("New leader for specie {:?}: {:?}", specie.id, specie.leader);
                    } else {
                        let specie_id = specie.id;
                        info!("Specie {:?} is extinct", specie_id);
                        species.species.retain(|s| s.id != specie_id);
                    }
                } else {
                    specie.members.retain(|s| *s != head_id);
                    debug!("Snake {:?} died and was removed from specie {:?}", head_id, specie.id);
                }
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
                let mut mutations = snake.mutations;
                if rng.gen_bool(0.1) {
                    new_neural_network.flip_random_connection();
                    mutations += 1;
                }
                if rng.gen_bool(0.3) {
                    new_neural_network.mutate_random_connection_weight();
                    mutations += 1;
                }
                debug!("New neural network: {:?}", new_neural_network);
                new_head = create_head((new_head_position.x, new_head_position.y), Box::new(RandomNeuralBrain::from_neural_network(new_neural_network.clone())), snake.generation + 1, mutations);
                new_head.0.segments = new_snake_segments;
                let new_head_id = new_head.0.segments[0];
                new_head.0.direction = flip_direction(&snake.direction);
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

pub fn calculate_stats(food: Query<&Food>, snakes: Query<(&Snake, &Age)>, solids: Query<&Solid>, mut stats: ResMut<Stats>, species: Res<Species>) {
    puffin::profile_function!();
    let max_age = snakes.iter().map(|(_, a)| a.0).reduce(|a, b| a.max(b));
    let max_generation = snakes.iter().map(|(s, _)| s.generation).reduce(|a, b| a.max(b));
    let max_mutation = snakes.iter().map(|(s, _)| s.mutations).reduce(|a, b| a.max(b));
    stats.oldest_snake = max_age.unwrap_or(0);
    stats.total_snakes = snakes.iter().count();
    stats.total_food = food.iter().count();
    stats.total_solids = solids.iter().count();
    stats.max_generation = max_generation.unwrap_or(0);
    stats.max_mutations = max_mutation.unwrap_or(0);
    stats.species = species.clone();
}

pub fn grow(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake, &mut Energy)>, config: Res<SimulationConfig>) {
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

pub fn create_snake(energy: i32, position: (i32, i32), brain: Box<dyn Brain>) -> (Position, Energy, Snake, Age, JustBorn, Solid) {
    let (head, age, just_born) = create_head(position, brain, 0, 0);
    (Position { x: position.0, y: position.1 }, Energy { amount: energy }, head, age, just_born, Solid)
}

fn create_head(position: (i32, i32), brain: Box<dyn Brain>, generation: u32, mutations: u32) -> (Snake, Age, JustBorn) {
    (Snake { direction: Direction::West, decision: Decision::Wait, brain, new_position: position, segments: vec![], last_position: position, generation, mutations, species: None }, Age(0), JustBorn)
}