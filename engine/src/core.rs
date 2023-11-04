use rand::Rng;
use bevy_ecs::prelude::*;
use crate::simulation::SimulationConfig;

#[derive(Component)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct Energy {
    pub(crate) amount: usize
}

#[derive(Component)]
pub struct Food {
}

#[derive(Resource)]
pub struct EntityMap {
    pub map: Vec<Vec<Option<Entity>>>,
}

// This system moves each entity with a Position and Velocity component
pub fn movement(mut query: Query<(&mut Position, &mut Energy), Without<Food>>, config: Res<SimulationConfig>) {
    puffin::profile_function!();
    let mut rng = rand::thread_rng();
    let rows = config.rows as i32;
    let columns = config.columns as i32;
    for (mut position, mut energy) in &mut query {
        if energy.amount > 0 {
            position.x = (position.x + rng.gen_range(-1..=1) + columns) % columns;
            position.y = (position.y + rng.gen_range(-1..=1) + rows) % rows;
            energy.amount -= 1;
        }
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

pub fn reproduce(mut commands: Commands, mut snakes: Query<(&mut Energy, &Position)>) {
    puffin::profile_function!();
    for (mut energy, position) in &mut snakes {
        if energy.amount >= 20 {
            energy.amount -= 10;
            commands.spawn((Position { x: position.x, y: position.y }, Energy { amount: 10 }));
        }
    }
}
