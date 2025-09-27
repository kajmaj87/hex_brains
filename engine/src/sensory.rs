use crate::core::{
    position_at_direction, turn_left, turn_right, Direction, FoodMap, Position, ScentMap, SolidsMap,
};
use crate::simulation::SimulationConfig;

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

pub fn collect_scent_inputs(
    scent_map: &ScentMap,
    config: &SimulationConfig,
    position: Position,
    direction: &Direction,
) -> [f32; 3] {
    let direction_left = turn_left(direction);
    let direction_right = turn_right(direction);
    let scent_front = scent(
        &position_at_direction(direction, position.clone(), *config),
        scent_map,
        *config,
    );
    let scent_left = scent(
        &position_at_direction(&direction_left, position.clone(), *config),
        scent_map,
        *config,
    );
    let scent_right = scent(
        &position_at_direction(&direction_right, position.clone(), *config),
        scent_map,
        *config,
    );
    [scent_front, scent_left, scent_right]
}

pub fn collect_vision_inputs(
    food_map: &FoodMap,
    solids_map: &SolidsMap,
    config: &SimulationConfig,
    position: Position,
    direction: &Direction,
) -> [f32; 9] {
    let direction_left = turn_left(direction);
    let direction_right = turn_right(direction);
    let plant_vision_front = see_plants(
        direction,
        position.clone(),
        config.mutation.plant_vision_front_range,
        food_map,
        *config,
    );
    let plant_vision_left = see_plants(
        &direction_left,
        position.clone(),
        config.mutation.plant_vision_left_range,
        food_map,
        *config,
    );
    let plant_vision_right = see_plants(
        &direction_right,
        position.clone(),
        config.mutation.plant_vision_right_range,
        food_map,
        *config,
    );
    let meat_vision_front = see_meat(
        direction,
        position.clone(),
        config.mutation.meat_vision_front_range,
        food_map,
        *config,
    );
    let meat_vision_left = see_meat(
        &direction_left,
        position.clone(),
        config.mutation.meat_vision_left_range,
        food_map,
        *config,
    );
    let meat_vision_right = see_meat(
        &direction_right,
        position.clone(),
        config.mutation.meat_vision_right_range,
        food_map,
        *config,
    );
    let solid_vision_front = see_obstacles(
        direction,
        position.clone(),
        config.mutation.obstacle_vision_front_range,
        solids_map,
        *config,
    );
    let solid_vision_left = see_obstacles(
        &direction_left,
        position.clone(),
        config.mutation.obstacle_vision_left_range,
        solids_map,
        *config,
    );
    let solid_vision_right = see_obstacles(
        &direction_right,
        position.clone(),
        config.mutation.obstacle_vision_right_range,
        solids_map,
        *config,
    );
    [
        plant_vision_front,
        plant_vision_left,
        plant_vision_right,
        meat_vision_front,
        meat_vision_left,
        meat_vision_right,
        solid_vision_front,
        solid_vision_left,
        solid_vision_right,
    ]
}
