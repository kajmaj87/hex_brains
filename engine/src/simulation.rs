use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;
use bevy_ecs::prelude::*;
use rand::{Rng, thread_rng};
use crate::core::{create_food, create_snake, Decision, Direction, eat_food, Energy, EntityMap, grow, Head, movement, Position, RandomBrain, reproduce, starve, think, update_positions};

pub struct Simulation {
    schedule: Schedule,
    gui_schedule: Schedule,
    world: World,
    pub name: String,
    engine_events: Sender<EngineEvent>,
    // only the main simulation may receive commands
    engine_commands: Option<Receiver<EngineCommand>>,
    has_gui: bool
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    SimulationFinished { steps: u32, name: String, duration: u128 },
    FrameDrawn { updates_left: f32, updates_done: u32 },
}

#[derive(Debug, Resource)]
pub struct SimulationConfig {
    pub rows: usize,
    pub columns: usize,
    pub food_per_step: usize,
    pub energy_per_segment: i32,
    pub wait_cost: i32,
    pub move_cost: i32,
    pub energy_to_breed: i32,
    pub energy_to_grow: i32,
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    RepaintRequested,
    IncreaseSpeed,
    DecreaseSpeed,
    RemoveSpeedLimit,
    FlipRunningState,
    CreateSnakes(usize),
}

#[derive(Debug, Resource)]
pub struct EngineState {
    pub repaint_needed: bool,
    pub speed_limit: Option<f32>,
    pub running: bool,
    pub frames_left: f32,
    pub frames: u32,
    pub updates_done: u32,
}

fn turn_counter(mut engine_state: ResMut<EngineState>) {
    puffin::profile_function!();
    if engine_state.speed_limit.is_some() {
        engine_state.frames_left -= 1.0;
    }
    engine_state.updates_done += 1;
    engine_state.frames += 1;
}

fn should_simulate_frame(engine_state: Res<EngineState>) -> bool {
    engine_state.speed_limit.is_none() || (engine_state.running && engine_state.frames_left > 0.0)
}

impl Simulation {
    pub fn new(name: String, engine_events: Sender<EngineEvent>, engine_commands: Option<Receiver<EngineCommand>>, rows: usize, columns: usize) -> Self {
        let mut world = World::new();
        let config = SimulationConfig { rows, columns, food_per_step: 10, energy_per_segment: 100, wait_cost: 1, move_cost: 10, energy_to_breed: 120, energy_to_grow: 120 };
        for i in 0..6 {
            world.spawn(create_snake(config.energy_per_segment, (50, 50), Box::new(RandomBrain {})));
        }
        world.insert_resource(config);
        world.insert_resource(EntityMap { map: vec![vec![None; columns]; rows] });
        let mut schedule = Schedule::default();
        schedule.add_systems((think, (movement, update_positions, grow).chain(), (eat_food, create_food).chain(), turn_counter).run_if(should_simulate_frame));
        let gui_schedule = Schedule::default();
        Simulation { schedule, gui_schedule, world, name, engine_events, engine_commands, has_gui: false }
    }

    pub fn step(&mut self) {
        puffin::profile_function!();
        self.schedule.run(&mut self.world);
        // if self.has_gui {
        //     let updates = {
        //         let mut engine_state = self.world.get_resource_mut::<EngineState>().unwrap();
        //         if engine_state.repaint_needed && engine_state.running {
        //             engine_state.frames_left += engine_state.speed_limit.unwrap_or(0.00);
        //             let updates_left = engine_state.frames_left;
        //             let updates_done = engine_state.updates_done;
        //             Some((updates_left, updates_done))
        //         } else {
        //             None
        //         }
        //     };
        //
        //     if let Some((updates_left, updates_done)) = updates {
        //         self.gui_schedule.run(&mut self.world);
        //         let mut engine_state = self.world.get_resource_mut::<EngineState>().unwrap();
        //         engine_state.updates_done = 0;
        //         engine_state.repaint_needed = false;
        //         self.engine_events.send(EngineEvent::FrameDrawn { updates_left, updates_done }).unwrap();
        //     }
        // }
    }

    pub fn is_done(&mut self) -> bool {
        self.world.query::<&Position>().iter(&self.world).next().unwrap().x > 10_000
    }

    pub fn run(&mut self) -> EngineEvent {
        let start_time = Instant::now();
        while !self.is_done() {
            if let Some(commands) = self.engine_commands.as_ref() {
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
                        EngineCommand::RemoveSpeedLimit => {
                            engine_state.speed_limit = None;
                        }
                        EngineCommand::FlipRunningState => {
                            engine_state.running = !engine_state.running;
                        }
                        EngineCommand::CreateSnakes(amount) => {
                           // let config = self.world.get_resource::<SimulationConfig>().unwrap();
                           for _ in 0..amount {
                               let mut rng = thread_rng();
                               let x = rng.gen_range(0..100);
                               let y = rng.gen_range(0..100);
                               self.world.spawn(create_snake(100, (x, y), Box::new(RandomBrain {})));
                           }
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
        self.schedule.add_systems(system);
    }

    pub fn add_gui_system<M>(&mut self, system: impl IntoSystemConfigs<M>) {
        self.gui_schedule.add_systems(system);
        self.has_gui = true;
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.world.insert_resource(resource);
    }
}