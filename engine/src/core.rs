use rand::Rng;
use bevy_ecs::prelude::*;
use crate::simulation::SimulationConfig;

#[derive(Component)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone)]
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
pub fn movement(mut query: Query<(&mut Position, &mut Energy, &mut Head)>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    let mut rng = rand::thread_rng();
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    for (mut position, mut energy, mut head) in &mut query {
        match head.decision {
            Decision::MoveForward => {
                energy.amount -= config.move_cost;
                move_to_direction(head.direction.clone(), &mut position, &config);
            }
            Decision::MoveLeft => {
                energy.amount -= config.move_cost;
                head.direction = turn_left(head.direction.clone());
                move_to_direction(head.direction.clone(), &mut position, &config);
            }
            Decision::MoveRight => {
                energy.amount -= config.move_cost;
                head.direction = turn_right(head.direction.clone());
                move_to_direction(head.direction.clone(), &mut position, &config);
            }
            Decision::Wait => {
                energy.amount -= config.wait_cost;
            }
        }
        position.x = (position.x + rng.gen_range(-1..=1) + columns) % columns;
        position.y = (position.y + rng.gen_range(-1..=1) + rows) % rows;
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
fn move_to_direction(direction: Direction, mut position: &mut Position, config: &Res<SimulationConfig>) {
    match direction {
        Direction::NorthEast => {
            position.x += 1;
            position.y += 1;
        }
        Direction::East => {
            position.x += 1;
        }
        Direction::SouthEast => {
            position.x += 1;
            position.y -= 1;
        }
        Direction::SouthWest => {
            position.x -= 1;
            position.y -= 1;
        }
        Direction::West => {
            position.x -= 1;
        }
        Direction::NorthWest => {
            position.x -= 1;
            position.y += 1;
        }
    }
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    position.x = (position.x + columns) % columns;
    position.y = (position.y + rows) % rows;
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
            energy.amount += config.energy_per_food;
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


pub fn create_snake(energy: i32, position: (i32, i32), brain: Box<dyn Brain>) -> (Position, Energy, Head) {
    (Position { x: position.0, y: position.1 }, Energy { amount: energy }, Head { direction: Direction::NorthEast, decision: Decision::Wait, brain })
}