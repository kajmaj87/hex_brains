use crate::core::die_from_collisions;
use crate::core::SolidsMap;
use crate::core::{
    add_scents, assign_solid_positions, destroy_old_food, diffuse_scents, disperse_scents, Solid,
};
use crate::core::{
    assign_missing_segments, assign_species, calculate_stats, create_food, create_snake, eat_food,
    grow, increase_age, movement, split, starve, think, update_positions, FoodMap, Position,
    RandomNeuralBrain, Species,
};
use crate::core::{
    assign_segment_positions, despawn_food, incease_move_potential, process_food, BrainType, Food,
    Map2d, Map3d, ScentMap, SegmentMap,
};
use crate::dna::{Dna, SegmentType};
use crate::neural::InnovationTracker;
use bevy_ecs::prelude::{IntoSystemConfigs, Res, ResMut, Resource, Schedule, World};
use tinyrand::{RandRange, Wyrand};

#[derive(Resource)]
pub struct RngResource(pub Wyrand);
use parking_lot::Mutex;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

pub struct Simulation {
    first_schedule: Schedule,
    core_schedule: Schedule,
    secondary_schedule: Schedule,
    gui_schedule: Schedule,
    pub world: World,
    pub name: String,
    engine_events: Sender<EngineEvent>,
    // only the main simulation may receive commands
    engine_commands: Option<Arc<Mutex<Receiver<EngineCommand>>>>,
    has_gui: bool,
}

#[derive(Debug, Clone)]
pub struct Hex {
    pub x: usize,
    pub y: usize,
    pub hex_type: HexType,
}

#[derive(Debug, Clone)]
pub enum HexType {
    Food,
    SnakeHead { specie: u32 },
    SnakeTail,
    Scent { value: f32 },
    Segment { segment_type: SegmentType },
    Meat,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct Stats {
    pub total_snakes: usize,
    pub total_food: usize,
    pub oldest_snake: u32,
    pub food: usize,
    pub total_segments: usize,
    pub max_generation: u32,
    pub max_mutations: u32,
    pub species: Species,
    pub total_entities: usize,
    pub total_scents: usize,
    pub total_snake_energy: f32,
    pub total_plants_in_stomachs: f32,
    pub total_meat_in_stomachs: f32,
    pub total_plants: f32,
    pub total_meat: f32,
    pub total_energy: f32,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    SimulationFinished {
        steps: u32,
        name: String,
        duration: u128,
    },
    DrawData {
        hexes: Vec<Hex>,
        stats: Stats,
    },
    FrameDrawn {
        updates_left: f32,
        updates_done: u32,
    },
}

#[derive(Debug, Resource, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MutationConfig {
    pub scent_sensing_enabled: bool,
    pub plant_vision_enabled: bool,
    pub meat_vision_enabled: bool,
    pub obstacle_vision_enabled: bool,
    pub chaos_input_enabled: bool,
    pub plant_vision_front_range: u32,
    pub plant_vision_left_range: u32,
    pub plant_vision_right_range: u32,
    pub meat_vision_front_range: u32,
    pub meat_vision_left_range: u32,
    pub meat_vision_right_range: u32,
    pub obstacle_vision_front_range: u32,
    pub obstacle_vision_left_range: u32,
    pub obstacle_vision_right_range: u32,
    pub weight_perturbation_range: f32,
    pub weight_perturbation_chance: f64,
    pub perturb_disabled_connections: bool,
    pub connection_flip_chance: f64,
    pub dna_mutation_chance: f64,
    pub weight_reset_chance: f64,
    pub weight_reset_range: f32,
    pub perturb_reset_connections: bool,
}

impl Default for MutationConfig {
    fn default() -> Self {
        MutationConfig {
            scent_sensing_enabled: true,
            plant_vision_enabled: true,
            obstacle_vision_enabled: true,
            chaos_input_enabled: true,
            plant_vision_front_range: 5,
            plant_vision_left_range: 3,
            plant_vision_right_range: 3,
            obstacle_vision_front_range: 5,
            obstacle_vision_left_range: 3,
            obstacle_vision_right_range: 3,
            weight_perturbation_range: 0.1,
            weight_perturbation_chance: 0.75,
            perturb_disabled_connections: false,
            connection_flip_chance: 0.3,
            dna_mutation_chance: 0.5,
            weight_reset_chance: 0.1,
            weight_reset_range: 1.0,
            perturb_reset_connections: true,
            meat_vision_front_range: 5,
            meat_vision_left_range: 3,
            meat_vision_right_range: 3,
            meat_vision_enabled: true,
        }
    }
}

#[derive(Debug, Resource, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SimulationConfig {
    pub rows: usize,
    pub columns: usize,
    pub starting_snakes: usize,
    pub starting_food: usize,
    pub food_per_step: usize,
    pub plant_matter_per_segment: f32,
    pub wait_cost: f32,
    pub move_cost: f32,
    pub new_segment_cost: f32,
    pub size_to_split: usize,
    pub species_threshold: f32,
    pub mutation: MutationConfig,
    pub add_walls: bool,
    pub scent_diffusion_rate: f32,
    pub scent_dispersion_per_step: f32,
    pub create_scents: bool,
    pub snake_max_age: u32,
    pub meat_energy_content: f32,
    pub plant_energy_content: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig {
            rows: 10,
            columns: 10,
            starting_snakes: 0,
            starting_food: 0,
            food_per_step: 1,
            plant_matter_per_segment: 10.0,
            wait_cost: 0.0,
            move_cost: 1.0,
            new_segment_cost: 50.0,
            size_to_split: 2,
            species_threshold: 0.3,
            mutation: MutationConfig::default(),
            add_walls: false,
            scent_diffusion_rate: 0.01,
            scent_dispersion_per_step: 0.01,
            create_scents: false,
            snake_max_age: 10000,
            meat_energy_content: 20.0,
            plant_energy_content: 10.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    RepaintRequested,
    IncreaseSpeed,
    DecreaseSpeed,
    IgnoreSpeedLimit,
    FlipRunningState,
    CreateSnakes(usize),
    StopSimulation,
    UpdateSimulationConfig(SimulationConfig),
    AdvanceOneFrame,
    ResetWorld,
}

#[derive(Default, Debug, Resource, Clone, Copy)]
pub struct EngineState {
    pub repaint_needed: bool,
    pub speed_limit: Option<f32>,
    pub running: bool,
    pub frames_left: f32,
    pub frames: u32,
    pub updates_done: u32,
    pub finished: bool,
    pub ignore_speed_limit: bool,
}

#[derive(Resource)]
pub struct EngineEvents {
    pub events: Mutex<Sender<EngineEvent>>,
}

fn turn_counter(mut engine_state: ResMut<EngineState>) {
    puffin::profile_function!();
    if engine_state.speed_limit.is_some() && !engine_state.ignore_speed_limit {
        engine_state.frames_left -= 1.0;
    }
    engine_state.updates_done += 1;
    engine_state.frames += 1;
}

fn should_simulate_frame(engine_state: Res<EngineState>) -> bool {
    let result = engine_state.ignore_speed_limit
        || engine_state.speed_limit.is_none()
        || (engine_state.running && engine_state.frames_left > 0.0);
    if result && !engine_state.running {
        tracing::warn!(
            "Simulating frame while not running: ignore={}, speed_limit={:?}, frames_left={}",
            engine_state.ignore_speed_limit,
            engine_state.speed_limit,
            engine_state.frames_left
        );
    }
    result
}

fn should_calculate_stats(_engine_state: Res<EngineState>) -> bool {
    true
}
fn should_despawn_food(engine_state: Res<EngineState>) -> bool {
    engine_state.frames % 10 == 0
}

fn should_increase_age(engine_state: Res<EngineState>) -> bool {
    engine_state.frames % 10 == 0
}

impl Simulation {
    pub fn new(
        name: String,
        engine_events: Sender<EngineEvent>,
        engine_commands: Option<Arc<Mutex<Receiver<EngineCommand>>>>,
        config: SimulationConfig,
    ) -> Self {
        let mut world = World::new();
        let innovation_tracker = InnovationTracker::new();
        // for _ in 0..config.starting_snakes {
        //     world.spawn(create_snake(config.energy_per_segment, (50, 50), Box::new(RandomNeuralBrain::new(&mut innovation_tracker))));
        // }
        // for _ in 0..config.starting_food {
        //     world.spawn(
        // }
        let mut solids = SolidsMap {
            map: Map2d::new(config.columns, config.rows, false),
        };
        if config.add_walls {
            for x in 0..config.columns {
                let middle = config.rows / 2;
                if x != middle && x != middle + 1 && x != middle - 1 {
                    let position = Position {
                        x: x as i32,
                        y: (config.rows / 4) as i32,
                    };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                    let position = Position {
                        x: x as i32,
                        y: (2 * config.rows / 4) as i32,
                    };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                    let position = Position {
                        x: x as i32,
                        y: (3 * config.rows / 4) as i32,
                    };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                }
            }
        }
        world.insert_resource(config);
        world.insert_resource(Stats::default());
        world.insert_resource(FoodMap {
            map: Map2d::new(config.columns, config.rows, Food::default()),
        });
        world.insert_resource(solids);
        world.insert_resource(ScentMap {
            map: Map2d::new(config.columns, config.rows, 0.0),
        });
        world.insert_resource(SegmentMap {
            map: Map3d::new(config.columns, config.rows),
        });
        world.insert_resource(EngineEvents {
            events: Mutex::new(engine_events.clone()),
        });
        world.insert_resource(innovation_tracker);
        world.insert_resource(Species::default());
        let rng = RngResource(Wyrand::default());
        world.insert_resource(rng);
        let mut first_schedule = Schedule::default();
        let mut core_schedule = Schedule::default();
        let mut secondary_schedule = Schedule::default();
        first_schedule.add_systems(
            (
                assign_species,
                starve,
                (
                    assign_missing_segments,
                    create_food,
                    incease_move_potential,
                    process_food,
                ),
                die_from_collisions,
                grow,
                add_scents,
            )
                .chain()
                .run_if(should_simulate_frame),
        );
        core_schedule.add_systems(
            (
                (
                    think,
                    increase_age.run_if(should_increase_age),
                    calculate_stats.run_if(should_calculate_stats),
                    diffuse_scents,
                ),
                (movement, update_positions, split).chain(),
                eat_food,
                destroy_old_food,
            )
                .chain()
                .run_if(should_simulate_frame),
        );
        secondary_schedule.add_systems(
            (
                (assign_solid_positions, assign_segment_positions),
                (
                    turn_counter,
                    disperse_scents,
                    despawn_food.run_if(should_despawn_food),
                ),
            )
                .chain()
                .run_if(should_simulate_frame),
        );
        let gui_schedule = Schedule::default();
        Simulation {
            first_schedule,
            core_schedule,
            secondary_schedule,
            gui_schedule,
            world,
            name,
            engine_events,
            engine_commands,
            has_gui: false,
        }
    }

    pub fn step(&mut self) {
        puffin::profile_function!();
        self.first_schedule.run(&mut self.world);
        self.core_schedule.run(&mut self.world);
        self.secondary_schedule.run(&mut self.world);
    }

    pub fn is_done(&mut self) -> bool {
        let engine_state = self.world.get_resource::<EngineState>().unwrap();
        engine_state.finished
    }

    pub fn run(&mut self) -> EngineEvent {
        let start_time = Instant::now();
        while !self.is_done() {
            if let Some(commands) = self
                .engine_commands
                .as_ref()
                .map(|arc_mutex| arc_mutex.lock())
            {
                commands.try_iter().for_each(|command| {
                    match command {
                        EngineCommand::RepaintRequested => {
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.repaint_needed = true;
                        }
                        EngineCommand::IncreaseSpeed => {
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.speed_limit = engine_state
                                .speed_limit
                                .map(|limit| limit.max(0.01) * 2.0)
                                .or(Some(0.02));
                        }
                        EngineCommand::DecreaseSpeed => {
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.speed_limit = engine_state
                                .speed_limit
                                .map(|limit| limit.max(0.04) / 2.0)
                                .or(Some(0.02));
                        }
                        EngineCommand::IgnoreSpeedLimit => {
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.ignore_speed_limit = !engine_state.ignore_speed_limit;
                        }
                        EngineCommand::FlipRunningState => {
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.running = !engine_state.running;
                        }
                        EngineCommand::CreateSnakes(amount) => {
                            // let config = self.world.get_resource::<SimulationConfig>().unwrap();
                            let config = *self.world.get_resource::<SimulationConfig>().unwrap();
                            let mut snake_data = vec![];
                            for _ in 0..amount {
                                let mut rng_temp =
                                    self.world.get_resource_mut::<RngResource>().unwrap();
                                let x = rng_temp.0.next_range(0..config.columns) as i32;
                                let y = rng_temp.0.next_range(0..config.rows) as i32;
                                let dna = Dna::random(&mut rng_temp.0, 8);
                                snake_data.push((x, y, dna));
                            }
                            let mut entities_to_spawn = vec![];
                            let mut innovation_tracker =
                                self.world.remove_resource::<InnovationTracker>().unwrap();
                            let mut rng_resource =
                                self.world.remove_resource::<RngResource>().unwrap();
                            {
                                for (x, y, dna) in snake_data {
                                    let brain = BrainType::Neural(RandomNeuralBrain::new(
                                        &mut innovation_tracker,
                                        &mut rng_resource.0,
                                    ));
                                    let (a, b, mut c, d, e) = create_snake(
                                        100.0,
                                        (x, y),
                                        brain,
                                        dna,
                                        &mut rng_resource.0,
                                    );
                                    c.metabolism.segment_basic_cost =
                                        c.brain.get_neural_network().unwrap().run_cost();
                                    entities_to_spawn.push((a, b, c, d, e));
                                }
                            }
                            self.world.insert_resource(innovation_tracker);
                            self.world.insert_resource(rng_resource);
                            for (a, b, c, d, e) in entities_to_spawn {
                                self.world.spawn((a, b, c, d, e));
                            }
                        }
                        EngineCommand::StopSimulation => {
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.finished = true;
                        }
                        EngineCommand::UpdateSimulationConfig(new_config) => {
                            self.world.remove_resource::<SimulationConfig>();
                            self.world.insert_resource(new_config);
                        }
                        EngineCommand::AdvanceOneFrame => {
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.ignore_speed_limit = false;
                            engine_state.speed_limit = Some(0.0);
                            engine_state.frames_left += 1.0;
                        }
                        EngineCommand::ResetWorld => {
                            // Despawn all entities
                            let entities: Vec<bevy_ecs::entity::Entity> = self
                                .world
                                .query::<bevy_ecs::entity::Entity>()
                                .iter(&self.world)
                                .collect();
                            for entity in entities {
                                self.world.despawn(entity);
                            }
                            // Get config
                            let config = *self.world.get_resource::<SimulationConfig>().unwrap();
                            // Recreate solids
                            let mut solids = SolidsMap {
                                map: Map2d::new(config.columns, config.rows, false),
                            };
                            if config.add_walls {
                                for x in 0..config.columns {
                                    let middle = config.rows / 2;
                                    if x != middle && x != middle + 1 && x != middle - 1 {
                                        for &y_offset in &[
                                            config.rows / 4,
                                            2 * config.rows / 4,
                                            3 * config.rows / 4,
                                        ] {
                                            let position = Position {
                                                x: x as i32,
                                                y: y_offset as i32,
                                            };
                                            solids.map.set(&position, true);
                                            self.world.spawn((Solid, position));
                                        }
                                    }
                                }
                            }
                            self.world.insert_resource(solids);
                            // Reset other resources
                            *self.world.get_resource_mut::<Stats>().unwrap() = Stats::default();
                            self.world.insert_resource(FoodMap {
                                map: Map2d::new(config.columns, config.rows, Food::default()),
                            });
                            self.world.insert_resource(ScentMap {
                                map: Map2d::new(config.columns, config.rows, 0.0),
                            });
                            self.world.insert_resource(SegmentMap {
                                map: Map3d::new(config.columns, config.rows),
                            });
                            self.world.insert_resource(Species::default());
                            let mut engine_state =
                                self.world.get_resource_mut::<EngineState>().unwrap();
                            engine_state.frames = 0;
                            engine_state.updates_done = 0;
                        }
                    }
                });
            }
            self.step();
            let mut engine_state = self.world.get_resource_mut::<EngineState>().unwrap();
            if engine_state.repaint_needed && engine_state.running {
                let increment = engine_state.speed_limit.unwrap_or(0.00);
                engine_state.frames_left += increment;
                if let Some(limit) = engine_state.speed_limit {
                    engine_state.frames_left = engine_state.frames_left.min(limit);
                }
                self.engine_events
                    .send(EngineEvent::FrameDrawn {
                        updates_left: engine_state.frames_left,
                        updates_done: engine_state.updates_done,
                    })
                    .unwrap();
                engine_state.updates_done = 0;
            }
            engine_state.repaint_needed = false;
        }
        let duration = start_time.elapsed().as_millis();

        let engine_state = self.world.get_resource::<EngineState>().unwrap();
        let result = EngineEvent::SimulationFinished {
            steps: engine_state.frames,
            name: self.name.clone(),
            duration,
        };
        let _ = self.engine_events.send(result.clone());
        result
    }

    pub fn add_system<M>(&mut self, system: impl IntoSystemConfigs<M>) {
        self.core_schedule.add_systems(system);
    }

    pub fn add_gui_system<M>(&mut self, system: impl IntoSystemConfigs<M>) {
        self.gui_schedule.add_systems(system);
        self.has_gui = true;
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.world.insert_resource(resource);
    }
}
