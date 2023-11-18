use crate::core::{assign_segment_positions, despawn_food, Food, incease_move_potential, Map2d, Map3d, process_food, ScentMap, SegmentMap};
use std::sync::Arc;
use std::f32::consts::PI;
use crate::core::{add_scents, assign_solid_positions, destroy_old_food, diffuse_scents, disperse_scents, Solid};
use crate::core::{die_from_collisions};
use crate::core::SolidsMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::time::Instant;
use bevy_ecs::prelude::{IntoSystemConfigs, Res, ResMut, Resource, Schedule, World};
use rand::{Rng, thread_rng};
use crate::core::{create_food, create_snake, Decision, Direction, eat_food, MeatMatter, FoodMap, grow, Snake, movement, Position, RandomBrain, reproduce, split, starve, think, update_positions, assign_missing_segments, increase_age, calculate_stats, RandomNeuralBrain, assign_species, Species};
use crate::dna::{Dna, SegmentType};
use crate::neural::InnovationTracker;

pub struct Simulation {
    first_schedule: Schedule,
    core_schedule: Schedule,
    secondary_schedule: Schedule,
    gui_schedule: Schedule,
    world: World,
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
    SnakeHead {
        specie: u32,
    },
    SnakeTail,
    Scent {
        value: f32,
    },
    Segment {
        segment_type: SegmentType
    },
    Meat,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct Stats {
    pub total_snakes: usize,
    pub total_food: usize,
    pub total_energy: i32,
    pub oldest_snake: u32,
    pub food: usize,
    pub total_segments: usize,
    pub max_generation: u32,
    pub max_mutations: u32,
    pub species: Species,
    pub total_entities: usize,
    pub total_scents: usize,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    SimulationFinished { steps: u32, name: String, duration: u128 },
    DrawData { hexes: Vec<Hex>, stats: Stats },
    FrameDrawn { updates_left: f32, updates_done: u32 },
}

#[derive(Debug, Resource, Clone, Copy)]
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
    pub connection_flip_chance: f64,
    pub dna_mutation_chance: f64,
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
            weight_perturbation_range: 0.2,
            weight_perturbation_chance: 0.3,
            connection_flip_chance: 0.1,
            dna_mutation_chance: 0.1,
            meat_vision_front_range: 5,
            meat_vision_left_range: 3,
            meat_vision_right_range: 3,
            meat_vision_enabled: true,
        }
    }
}

type EnergyValue = f32;

#[derive(Debug, Resource, Clone, Copy)]
pub struct SimulationConfig {
    pub rows: usize,
    pub columns: usize,
    pub starting_snakes: usize,
    pub starting_food: usize,
    pub food_per_step: usize,
    pub energy_per_segment: f32,
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
    pub plant_energy_content: f32
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
}

#[derive(Debug, Resource)]
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
    engine_state.ignore_speed_limit || engine_state.speed_limit.is_none() || (engine_state.running && engine_state.frames_left > 0.0)
}

impl Simulation {
    pub fn new(name: String, engine_events: Sender<EngineEvent>, engine_commands: Option<Arc<Mutex<Receiver<EngineCommand>>>>, config: SimulationConfig) -> Self {
        let mut world = World::new();
        let innovation_tracker = InnovationTracker::new();
        // for _ in 0..config.starting_snakes {
        //     world.spawn(create_snake(config.energy_per_segment, (50, 50), Box::new(RandomNeuralBrain::new(&mut innovation_tracker))));
        // }
        // for _ in 0..config.starting_food {
        //     world.spawn(
        // }
        let mut solids = SolidsMap { map: Map2d::new(config.columns, config.rows, false) };
        if config.add_walls {
            for x in 0..config.columns {
                let middle = config.rows / 2;
                if x != middle && x != middle + 1 && x != middle - 1 {
                    let position = Position { x: x as i32, y: (config.rows / 4) as i32 };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                    let position = Position { x: x as i32, y: (2 * config.rows / 4) as i32 };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                    let position = Position { x: x as i32, y: (3 * config.rows / 4) as i32 };
                    solids.map.set(&position, true);
                    world.spawn((Solid, position));
                }
            }
        }
        world.insert_resource(config);
        world.insert_resource(Stats::default());
        world.insert_resource(FoodMap { map: Map2d::new(config.columns, config.rows, Food::default()) });
        world.insert_resource(solids);
        world.insert_resource(ScentMap { map: Map2d::new(config.columns, config.rows, 0.0) });
        world.insert_resource(SegmentMap { map: Map3d::new(config.columns, config.rows) });
        world.insert_resource(EngineEvents { events: Mutex::new(engine_events.clone()) });
        world.insert_resource(innovation_tracker);
        world.insert_resource(Species::default());
        let mut first_schedule = Schedule::default();
        let mut core_schedule = Schedule::default();
        let mut secondary_schedule = Schedule::default();
        first_schedule.add_systems((assign_species, (assign_missing_segments, create_food, incease_move_potential, process_food), die_from_collisions, add_scents).chain().run_if(should_simulate_frame));
        core_schedule.add_systems(((think, increase_age, calculate_stats, diffuse_scents), (movement, update_positions, split).chain(), eat_food, destroy_old_food).chain().run_if(should_simulate_frame));
        secondary_schedule.add_systems(((assign_solid_positions, assign_segment_positions), (grow, starve,turn_counter, disperse_scents, despawn_food)).chain().run_if(should_simulate_frame));
        let gui_schedule = Schedule::default();
        Simulation { first_schedule, core_schedule, secondary_schedule, gui_schedule, world, name, engine_events, engine_commands, has_gui: false }
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
            if let Some(commands) = match &self.engine_commands {
                Some(arc_mutex) => arc_mutex.lock().ok(),
                None => None
            } {
                commands.try_iter().for_each(|command| {
                    let mut engine_state = self.world.get_resource_mut::<EngineState>().unwrap();
                    match command {
                        EngineCommand::RepaintRequested => {
                            engine_state.repaint_needed = true;
                        }
                        EngineCommand::IncreaseSpeed => {
                            engine_state.speed_limit = engine_state.speed_limit.map(|limit| limit * 2.0).or(Some(0.02));
                        }
                        EngineCommand::DecreaseSpeed => {
                            engine_state.speed_limit = engine_state.speed_limit.map(|limit| limit / 2.0).or(Some(0.02));
                        }
                        EngineCommand::IgnoreSpeedLimit => {
                            engine_state.ignore_speed_limit = !engine_state.ignore_speed_limit;
                        }
                        EngineCommand::FlipRunningState => {
                            engine_state.running = !engine_state.running;
                        }
                        EngineCommand::CreateSnakes(amount) => {
                            // let config = self.world.get_resource::<SimulationConfig>().unwrap();
                            let mut brains = vec![];
                            for _ in 0..amount {
                                let mut innovation_tracker = self.world.get_resource_mut::<InnovationTracker>().unwrap();
                                brains.push(RandomNeuralBrain::new(&mut innovation_tracker));
                            }
                            for brain in brains {
                                let mut rng = thread_rng();
                                let config = self.world.get_resource::<SimulationConfig>().unwrap();
                                let x = rng.gen_range(0..config.columns) as i32;
                                let y = rng.gen_range(0..config.rows) as i32;
                                {
                                    self.world.spawn(create_snake(100.0, (x, y), Box::new(brain), Dna::random(8)));
                                }
                            }
                        }
                        EngineCommand::StopSimulation => {
                            engine_state.finished = true;
                        }
                        EngineCommand::UpdateSimulationConfig(new_config) => {
                            self.world.remove_resource::<SimulationConfig>();
                            self.world.insert_resource(new_config);
                        }
                    }
                });
            }
            self.step();
            let mut engine_state = self.world.get_resource_mut::<EngineState>().unwrap();
            if engine_state.repaint_needed && engine_state.running {
                engine_state.frames_left += engine_state.speed_limit.unwrap_or(0.00);
                self.engine_events.send(EngineEvent::FrameDrawn { updates_left: engine_state.frames_left, updates_done: engine_state.updates_done }).unwrap();
                engine_state.updates_done = 0;
            }
            engine_state.repaint_needed = false;
        }
        let duration = start_time.elapsed().as_millis();

        let engine_state = self.world.get_resource::<EngineState>().unwrap();
        let result = EngineEvent::SimulationFinished { steps: engine_state.frames, name: self.name.clone(), duration };
        self.engine_events.send(result.clone());
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