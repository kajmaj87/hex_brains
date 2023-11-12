use std::sync::Arc;
use std::f32::consts::PI;
use crate::core::{assign_new_occupied_solid_positions, remove_occupied_solid_positions, Solid};
use crate::core::{assign_new_food_positions, die_from_collisions, remove_eaten_food};
use crate::core::SolidsMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::time::Instant;
use bevy_ecs::prelude::*;
use rand::{Rng, thread_rng};
use crate::core::{create_food, create_snake, Decision, Direction, eat_food, Energy, FoodMap, grow, Snake, movement, Position, RandomBrain, reproduce, split, starve, think, update_positions, assign_missing_segments, increase_age, calculate_stats, RandomNeuralBrain, assign_species, Species};
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
    SnakeTail
}

#[derive(Resource, Default, Debug, Clone)]
pub struct Stats {
    pub total_snakes: usize,
    pub total_food: usize,
    pub total_energy: i32,
    pub oldest_snake: u32,
    pub food: usize,
    pub total_solids: usize,
    pub max_generation: u32,
    pub max_mutations: u32,
    pub species: Species
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    SimulationFinished { steps: u32, name: String, duration: u128 },
    DrawData { hexes: Vec<Hex>, stats: Stats },
    FrameDrawn { updates_left: f32, updates_done: u32 },
}

#[derive(Debug, Resource, Clone, Copy)]
pub struct SimulationConfig {
    pub rows: usize,
    pub columns: usize,
    pub starting_snakes: usize,
    pub starting_food: usize,
    pub food_per_step: usize,
    pub energy_per_segment: i32,
    pub wait_cost: i32,
    pub move_cost: i32,
    pub energy_to_grow: i32,
    pub size_to_split: usize,
    pub species_threshold: f32,
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
    pub ignore_speed_limit: bool
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
        // for x in 0..config.columns {
        //     let middle = config.rows / 2;
        //     if x != middle && x != middle + 1 && x != middle - 1 {
        //         world.spawn((Solid, Position { x: x as i32, y: (config.rows / 4) as i32 }));
        //         world.spawn((Solid, Position { x: x as i32, y: (2 * config.rows / 4) as i32 }));
        //         world.spawn((Solid, Position { x: x as i32, y: (3 * config.rows / 4) as i32 }));
        //     }
        // }
        world.insert_resource(config);
        world.insert_resource(Stats::default());
        world.insert_resource(FoodMap{ map: vec![vec![vec![]; config.columns]; config.rows] });
        world.insert_resource(SolidsMap{ map: vec![vec![vec![]; config.columns]; config.rows] });
        world.insert_resource(EngineEvents { events: Mutex::new(engine_events.clone()) });
        world.insert_resource(innovation_tracker);
        world.insert_resource(Species::default());
        let mut first_schedule = Schedule::default();
        let mut core_schedule = Schedule::default();
        let mut secondary_schedule = Schedule::default();
        first_schedule.add_systems((assign_species, (assign_missing_segments, assign_new_occupied_solid_positions, create_food), die_from_collisions).chain().run_if(should_simulate_frame));
        core_schedule.add_systems(((think, increase_age, calculate_stats, assign_new_food_positions), (movement, update_positions, split).chain(), eat_food).chain().run_if(should_simulate_frame));
        secondary_schedule.add_systems((grow, starve, remove_eaten_food, remove_occupied_solid_positions, turn_counter).run_if(should_simulate_frame));
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
            }{
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
                                    self.world.spawn(create_snake(100, (x, y), Box::new(brain)));
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