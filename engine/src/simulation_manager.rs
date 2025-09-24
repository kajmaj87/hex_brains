use crate::simulation::{EngineEvent, Simulation};
use rayon::prelude::*;
use std::sync::mpsc::{channel, Sender};

pub fn simulate_batch(simulations: Vec<Simulation>) -> Vec<EngineEvent> {
    // let simulations: Vec<Simulation> = (0..num_simulations)
    //     .map(|i| Simulation::new(format!("Simulation {}", i), tx.clone())
    //     .collect();
    // // rayon::ThreadPoolBuilder::new()
    // //     .num_threads(1)
    // //     .build_global()
    // //     .unwrap();
    simulations
        .into_par_iter()
        .map(|mut simulation| simulation.run())
        .collect()
}
