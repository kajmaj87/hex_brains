use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;
use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Velocity {
    x: f32,
    y: f32,
}

pub struct Simulation {
    schedule: Schedule,
    world: World,
    pub name: String,
    engine_events: Sender<EngineEvent>,
    // only the main simulation may receive commands
    engine_commands: Option<Receiver<EngineCommand>>,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    SimulationFinished { steps: u32, name: String, duration: u128 },
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    RepaintRequested,
    IncreaseSpeed,
    DecreaseSpeed,
    RemoveSpeedLimit,
    FlipRunningState,
}

#[derive(Debug, Resource)]
pub struct EngineState {
    pub repaint_needed: bool,
    pub speed_limit: Option<f32>,
    pub running: bool,
    pub frames_left: f32,
    pub frames: u32,
}

// This system moves each entity with a Position and Velocity component
fn movement(mut query: Query<(&mut Position, &Velocity)>) {
    puffin::profile_function!();
    for (mut position, velocity) in &mut query {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

fn turn_counter(mut engine_state: ResMut<EngineState>) {
    puffin::profile_function!();
    if engine_state.speed_limit.is_some() {
        engine_state.frames_left -= 1.0;
    }
    engine_state.frames += 1;
}

fn should_simulate_frame(engine_state: Res<EngineState>) -> bool {
    engine_state.speed_limit.is_none() || (engine_state.running && engine_state.frames_left > 0.0)
}

impl Simulation {
    pub fn new(name: String, engine_events: Sender<EngineEvent>, engine_commands: Option<Receiver<EngineCommand>>) -> Self {
        let mut world = World::new();
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            Velocity { x: 1.0, y: 0.0 },
        ));
        let mut schedule = Schedule::default();
        schedule.add_systems((movement, turn_counter).run_if(should_simulate_frame));
        Simulation { schedule, world, name, engine_events, engine_commands }
    }

    pub fn step(&mut self) {
        puffin::profile_function!();
        self.schedule.run(&mut self.world);
    }

    pub fn is_done(&mut self) -> bool {
        self.world.query::<&Position>().iter(&self.world).next().unwrap().x > 10_000.0
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
                    }
                });
            }
            self.step();
            let mut engine_state = self.world.get_resource_mut::<EngineState>().unwrap();
            if engine_state.repaint_needed && engine_state.running {
                engine_state.frames_left += engine_state.speed_limit.unwrap_or(0.00);
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

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.world.insert_resource(resource);
    }
}