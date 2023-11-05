use std::collections::LinkedList;
use rand::Rng;
use bevy_ecs::prelude::*;
use crate::simulation::SimulationConfig;

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
    fn decide(&self) -> Decision;
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
}

#[derive(Component)]
pub struct Solid;

pub struct RandomBrain;

impl Brain for RandomBrain {
    fn decide(&self) -> Decision {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..=3) {
            0 => Decision::MoveForward,
            1 => Decision::MoveLeft,
            2 => Decision::MoveRight,
            _ => Decision::Wait
        }
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
    println!("move: start");
    for (_, mut energy, mut snake, head_position) in &mut snakes {
        print!("move: snake has {} segments, decides: {:?}", snake.segments.len(), snake.decision);
        match snake.decision {
            Decision::MoveForward => {
                println!("move forward");
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

pub fn update_positions(mut commands: Commands, mut positions: Query<&mut Position>, mut snakes: Query<(Entity, &mut Snake)>) {
    puffin::profile_function!();
    for (head_id, mut snake) in &mut snakes {
        let mut new_position = snake.new_position;
        let mut head_position = positions.get_mut(head_id).unwrap();
        let old_head_position = (head_position.x, head_position.y);
        if new_position == old_head_position {
            println!("update_positions: snake {:?} is waiting", head_id);
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
    for mut head in &mut heads {
        head.decision = head.brain.decide();
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

pub fn starve(mut commands: Commands, mut snakes: Query<(Entity, &mut Energy, &mut Snake, &Position)>) {
    puffin::profile_function!();
    // for (tail_id, mut energy, mut tail, position) in &mut snakes {
    //     if energy.amount <= 0 {
    //         commands.entity(tail_id).despawn();
    //         println!("Entity {:?} died at position ({}, {})\n", tail_id, position.x, position.y);
    //         tail.segments.pop();
    //         if tail.segments.len() == 0 {
    //             continue;
    //         }
    //         let new_tail_id = *tail.segments.last().unwrap();
    //         commands.entity(new_tail_id).insert(Tail { segments: tail.segments.clone(), last_position: (position.x, position.y) });
    //     }
    // }
}

pub fn reproduce(mut commands: Commands, mut snakes: Query<(&mut Energy, &Position)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (mut energy, position) in &mut snakes {
        if energy.amount >= config.energy_to_breed {
            energy.amount -= config.energy_to_breed / 2;
            let baby_energy = config.energy_to_breed - energy.amount;
            let snake = create_snake(baby_energy, (position.x, position.y), Box::new(RandomBrain {}));
            commands.spawn(snake);
        }
    }
}

pub fn split(mut commands: Commands, mut tails: Query<(Entity, &mut Snake, &Position)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (tail_id, mut tail, position) in &mut tails {
        // total snake length is 1 + segments.len()
        if tail.segments.len() == 5 {
            // // tail becomes head
            // commands.entity(tail_id).remove::<Tail>();
            // // TODO put real brain here when ready!!
            // commands.entity(tail_id).insert(create_head((position.x, position.y), Box::new(RandomBrain {})));
            // // find cutoff point
            // let (mut segment_id, mut segment) = segments.get_mut(tail_id).unwrap();
            // segment.flip();
            // let (mut segment_id, mut segment) = segments.get_mut(segment.back.unwrap()).unwrap();
            // segment.flip();
            // let (mut segment_id, mut segment) = segments.get_mut(segment.back.unwrap()).unwrap();
            // segment.flip();
            // // TODO position bad on purpose
            // commands.entity(segment_id).insert(Tail { length: 3, last_position: (50, 50), head: Some(tail_id) });
            // let back = segment.back;
            // // cut the snake
            // segment.back = None;
            // let (mut new_tail_id, segment) = segments.get(back.unwrap()).unwrap();
            // let (mut new_tail_id, segment) = segments.get(segment.front.unwrap()).unwrap();
            // let (mut head_id, segment) = segments.get(segment.front.unwrap()).unwrap();
            // commands.entity(new_tail_id).insert(Tail { length: 3, last_position: (50, 50), head: Some(head_id) });
        }
    }
}

pub fn grow(mut commands: Commands, mut snakes: Query<(Entity, &mut Snake, &mut Energy)>, positions: Query<&Position>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (snake_id, mut snake, mut energy) in &mut snakes {
        // tail always takes energy from head when growing
        if energy.amount >= config.energy_to_grow {
            let energy_for_tail = energy.amount - config.energy_to_grow / 2;
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

pub fn create_snake(energy: i32, position: (i32, i32), brain: Box<dyn Brain>) -> (Position, Energy, Snake, Solid) {
    (Position { x: position.0, y: position.1 }, Energy { amount: energy }, create_head(position, brain), Solid)
}

fn create_head(position: (i32, i32), brain: Box<dyn Brain>) -> Snake {
    Snake { direction: Direction::West, decision: Decision::Wait, brain, new_position: position, segments: vec![], last_position: position }
}