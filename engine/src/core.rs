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

pub enum Decision {
    MoveForward,
    MoveLeft,
    MoveRight,
    Wait,
}

pub trait Brain: Sync + Send {
    fn decide(&self) -> Decision;
}

#[derive(Component)]
pub struct Head {
    pub direction: Direction,
    pub decision: Decision,
    pub brain: Box<dyn Brain>,
    pub new_position: (i32, i32),
}

#[derive(Component)]
pub struct Tail {
    pub length: u32,
    pub last_position: (i32, i32),
    pub head: Option<Entity>,
}

#[derive(Component)]
pub struct Solid;

#[derive(Component, Default)]
pub struct Segment {
    pub front: Option<Entity>,
    pub back: Option<Entity>,
}

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
pub fn movement(mut snakes: Query<(Entity, &mut Position, &mut Energy, &mut Head)>, mut segments: Query<(Entity, &mut Segment)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    for (head_id, mut position, mut energy, mut head) in &mut snakes {
        let old_position = (position.x, position.y);
        match head.decision {
            Decision::MoveForward => {
                energy.amount -= config.move_cost;
                let new_position = move_to_direction(head.direction.clone(), &mut position, &config);
                head.new_position.0 = new_position.x;
                head.new_position.1 = new_position.y;
            }
            Decision::MoveLeft => {
                energy.amount -= config.move_cost;
                head.direction = turn_left(head.direction.clone());

                let new_position = move_to_direction(head.direction.clone(), &mut position, &config);
                head.new_position.0 = new_position.x;
                head.new_position.1 = new_position.y;
            }
            Decision::MoveRight => {
                energy.amount -= config.move_cost;
                head.direction = turn_right(head.direction.clone());

                let new_position = move_to_direction(head.direction.clone(), &mut position, &config);
                head.new_position.0 = new_position.x;
                head.new_position.1 = new_position.y;
            }
            Decision::Wait => {
                energy.amount -= config.wait_cost;
            }
        }
        // if old_position.0 != position.x || old_position.1 != position.y {

        //     let (_, head_segment) = segments.get_mut(head_id).unwrap();
        //     let (mut tail_id, mut tail_segment) = (None, None);
        //     let mut after_head_id= None;
        //     let mut total_segments = 0;
        //     while let Ok((segment_id, segment)) = segments.get_mut(head_segment.back.unwrap()) {
        //         tail_id = Some(segment_id);
        //         tail_segment = Some(segment);
        //         if after_head_id == None {
        //             after_head_id = Some(segment_id);
        //         }
        //         total_segments += 1;
        //     }
        //     if tail_segment.is_some() {
        //         let (_, new_tail_segment) = segments.get_mut(tail_id.unwrap()).unwrap();
        //         new_tail_segment.back = None;
        //         tail_segment.unwrap().front = Some(head_id);
        //         tail_segment.unwrap().back = after_head_id;
        //     }
        //     head_segment.back = tail_id;

        // } else {

        // }
        // TODO remove
        if energy.amount < -10 {
            energy.amount = 0;
        }
    }
}

pub fn update_positions(heads: Query<(Entity, &Head)>, mut segments: Query<(&Segment, &mut Position)>) {
    puffin::profile_function!();
    for (head_id, head) in heads.iter() {
        let mut new_position_x = head.new_position.0;
        let mut new_position_y = head.new_position.1;
        let mut next_segment_id = head_id;
        while let Ok((segment, mut position)) = segments.get_mut(next_segment_id) {
            let old_position_x = position.x;
            let old_position_y = position.y;
            position.x = new_position_x;
            position.y = new_position_y;
            new_position_x = old_position_x;
            new_position_y = old_position_y;
            if let Some(back_id) = segment.back {
                next_segment_id = back_id;
            } else {
                break;
            }
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

pub fn think(mut heads: Query<&mut Head>) {
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

pub fn starve(mut commands: Commands, mut snakes: Query<(Entity, &mut Energy)>) {
    puffin::profile_function!();
    for (snake, mut energy) in &mut snakes {
        if energy.amount <= 0 {
            commands.entity(snake).despawn();
        }
    }
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

pub fn grow(mut commands: Commands, mut tails: Query<(Entity, &mut Tail, &mut Segment)>, mut heads: Query<(Entity, &Head, &mut Energy)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    for (tail_id, mut tail, mut segment) in &mut tails {
        if let Some(head) = tail.head {
            let (_, _, mut energy) = heads.get_mut(head).unwrap();
            // tail always takes energy from head when growing
            if energy.amount >= config.energy_to_grow {
                let energy_for_tail = energy.amount - config.energy_to_grow / 2;
                energy.amount -= energy_for_tail;
                let new_tail = commands.spawn((Position { x: tail.last_position.0, y: tail.last_position.1 }, Solid, Segment { front: Some(tail_id), back: None }, Tail { length: tail.length + 1, last_position: (tail.last_position.0, tail.last_position.1), head: Some(head) }, Energy { amount: energy_for_tail })).id();
                segment.back = Some(new_tail);
                commands.entity(tail_id).remove::<Tail>();
            }
        } else {
            // tail does not now its head yet so initialize it to itself, this must be a small snake
            tail.head = Some(tail_id);
        }
    }
}


pub fn create_snake(energy: i32, position: (i32, i32), brain: Box<dyn Brain>) -> (Position, Energy, Head, Segment, Tail, Solid) {
    (Position { x: position.0, y: position.1 }, Energy { amount: energy }, Head { direction: Direction::SouthWest, decision: Decision::Wait, brain, new_position: position }, Segment::default(), Tail { length: 1, last_position: position, head: None }, Solid)
}