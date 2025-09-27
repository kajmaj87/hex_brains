use bevy_ecs::prelude::*;
use hex_brains_engine::core::ScentMap as SimScentMap;
use hex_brains_engine::core::{
    assign_missing_segments, assign_segment_positions, assign_species, create_snake, grow, split,
    Food, FoodMap as SimFoodMap, Map2d, Map3d, Position, RandomNeuralBrain, SegmentMap, Snake,
    SolidsMap, Species,
};
use hex_brains_engine::dna::{Dna, Gene, MutationType, SegmentType};
use hex_brains_engine::neural::{InnovationTracker, NeuralNetwork};
use hex_brains_engine::simulation::{
    EngineEvent, EngineState, MutationConfig, Simulation, SimulationConfig, Stats,
};
use tinyrand::Seeded;
#[test]
fn test_agent_reproduction_and_splitting_size_one() {
    let mut world = World::new();

    let config = SimulationConfig {
        rows: 10,
        columns: 10,
        size_to_split: 2,
        mutation: MutationConfig {
            dna_mutation_chance: 1.0,
            connection_flip_chance: 1.0,
            weight_perturbation_chance: 1.0,
            weight_perturbation_range: 0.1,
            ..Default::default()
        },
        ..Default::default()
    };

    world.insert_resource(config);
    world.insert_resource(Species::default());
    world.insert_resource(InnovationTracker::new());
    world.insert_resource(SimFoodMap {
        map: Map2d::new(10, 10, Food::default()),
    });
    world.insert_resource(SolidsMap {
        map: Map2d::new(10, 10, false),
    });
    world.insert_resource(SimScentMap {
        map: Map2d::new(10, 10, 0.0),
    });
    world.insert_resource(SegmentMap {
        map: Map3d::new(10, 10),
    });
    world.insert_resource(hex_brains_engine::simulation::RngResource {
        rng: tinyrand::Wyrand::seed(42),
    });
    let mut innovation_tracker = world.get_resource_mut::<InnovationTracker>().unwrap();

    // Create and spawn initial snake with sufficient energy
    let mut rng = tinyrand::Wyrand::seed(42);
    let initial_dna = Dna::random(&mut rng, 5, &MutationConfig::default());
    let brain = Box::new(RandomNeuralBrain::new(&mut innovation_tracker, &mut rng));
    let (pos, meat, snake, age, justborn) = create_snake(
        200.0,  // High energy to test halving
        (5, 5), // Central position
        brain,
        initial_dna.clone(),
        &mut world
            .resource_mut::<hex_brains_engine::simulation::RngResource>()
            .rng,
    );
    let head_id = world.spawn((pos, meat, snake, age, justborn)).id();

    // Spawn an additional segment for splitting
    let segment_pos = Position { x: 6, y: 5 };
    let segment_type = SegmentType::muscle();
    let segment_id = world.spawn((segment_pos, segment_type)).id();

    // Record initial state in limited scope to avoid borrow across mut ops
    let (
        initial_energy,
        initial_plant,
        initial_meat,
        initial_neural,
        initial_generation,
        initial_mutations,
    ): (f32, f32, f32, NeuralNetwork, u32, u32) = {
        let mut initial_snake_query = world.query::<&Snake>();
        let initial_snake = initial_snake_query.get(&world, head_id).unwrap();
        (
            initial_snake.energy.energy,
            initial_snake.energy.plant_in_stomach,
            initial_snake.energy.meat_in_stomach,
            initial_snake.brain.get_neural_network().unwrap().clone(),
            initial_snake.generation,
            initial_snake.mutations,
        )
    };

    // Create schedule for initial setup systems
    let mut setup_schedule = Schedule::default();
    setup_schedule.add_systems(assign_missing_segments);
    setup_schedule.add_systems(assign_species);
    setup_schedule.run(&mut world);

    // Add the segment to the snake's segments
    {
        let mut entity = world.entity_mut(head_id);
        let mut snake = entity.get_mut::<Snake>().unwrap();
        snake.segments.push(segment_id);
    }

    // Now run split system using a schedule
    let mut split_schedule = Schedule::default();
    split_schedule.add_systems(split);
    split_schedule.run(&mut world);

    // Run setup systems again for child
    let mut child_setup_schedule = Schedule::default();
    child_setup_schedule.add_systems(assign_missing_segments);
    child_setup_schedule.add_systems(assign_species);
    child_setup_schedule.run(&mut world);

    // Now verify: two snakes
    let mut final_snake_query = world.query::<(&Snake, &Position)>();
    let snakes: Vec<(&Snake, &Position)> = final_snake_query.iter(&world).collect();
    assert_eq!(snakes.len(), 2, "Exactly two snakes after split");

    // Identify parent and child (parent at original position (5,5), child at new)
    let (parent_snake, parent_pos) = if snakes[0].1.x == 5 && snakes[0].1.y == 5 {
        (snakes[0].0, snakes[0].1)
    } else {
        (snakes[1].0, snakes[1].1)
    };
    let parent_energy = parent_snake.energy.clone();
    let (child_snake, child_pos) = if snakes[0].1.x != 5 || snakes[0].1.y != 5 {
        (snakes[0].0, snakes[0].1)
    } else {
        (snakes[1].0, snakes[1].1)
    };
    let child_energy = child_snake.energy.clone();

    // Verify positions: child at valid new position, different from parent
    assert_ne!(parent_pos.as_pair(), child_pos.as_pair());
    assert!(child_pos.x >= 0 && child_pos.x < 10);
    assert!(child_pos.y >= 0 && child_pos.y < 10);

    // Verify energy halved and conserved
    let half_energy = initial_energy / 2.0;
    assert!((parent_energy.energy - half_energy).abs() < f32::EPSILON);
    assert!((child_energy.energy - half_energy).abs() < f32::EPSILON);
    assert_eq!(parent_energy.energy + child_energy.energy, initial_energy);

    let half_plant = initial_plant / 2.0;
    assert!((parent_energy.plant_in_stomach - half_plant).abs() < f32::EPSILON);
    assert!((child_energy.plant_in_stomach - half_plant).abs() < f32::EPSILON);

    let half_meat = initial_meat / 2.0;
    assert!((parent_energy.meat_in_stomach - half_meat).abs() < f32::EPSILON);
    assert!((child_energy.meat_in_stomach - half_meat).abs() < f32::EPSILON);

    // Verify mutated DNA (different from parent)
    assert_ne!(parent_snake.dna, child_snake.dna);

    // Verify mutated neural network (parent unchanged, child mutated)
    let parent_nn = parent_snake.brain.get_neural_network().unwrap();
    let child_nn = child_snake.brain.get_neural_network().unwrap();
    assert_eq!(parent_nn, &initial_neural);
    assert_ne!(child_nn, &initial_neural);

    // Verify species updated: both have species (likely same due to small mutations)
    assert!(parent_snake.species.is_some());
    assert!(child_snake.species.is_some());
    // Optional: check same species
    // assert_eq!(parent_snake.species, child_snake.species);

    // Verify child generation increased
    assert_eq!(child_snake.generation, initial_generation + 1);

    // Verify mutations count increased for child
    assert!(child_snake.mutations > initial_mutations);
}

#[test]
fn test_simulation_lifecycle_zero_agents() {
    let config = SimulationConfig {
        rows: 5,
        columns: 5,
        starting_snakes: 0,
        food_per_step: 1,
        ..Default::default()
    };

    let (tx, rx) = std::sync::mpsc::channel::<EngineEvent>();

    let mut sim = Simulation::new("zero_agents".to_string(), tx, None, config);

    sim.world.insert_resource(EngineState {
        finished: false,
        ..Default::default()
    });

    // Run one step to test systems with zero agents
    sim.step();

    // Verify no snakes (no think/movement execution)
    let mut snake_query = sim.world.query::<&Snake>();
    assert_eq!(snake_query.iter(&sim.world).count(), 0);

    // Verify empty species list
    let species_res = sim.world.resource::<Species>();
    assert!(species_res.species.is_empty());

    // Verify food map populated via create_food, no consumption
    let mut food_query = sim.world.query::<&Food>();
    assert!(
        food_query.iter(&sim.world).count() >= 1,
        "Expected at least one food entity created"
    );

    let food_map = sim.world.resource::<SimFoodMap>();
    let mut has_food = false;
    for food in food_map.map.map.iter() {
        if food.plant > 0.0 || food.meat > 0.0 {
            has_food = true;
            break;
        }
    }
    assert!(has_food, "Expected food in map");

    // Verify scent diffusion runs but empty (no scents added since no agents)
    let scent_map = sim.world.resource::<SimScentMap>();
    let all_zero = scent_map.map.map.iter().all(|&s| s == 0.0);
    assert!(all_zero);

    // Set finished to trigger end
    let mut engine_state = sim.world.resource_mut::<EngineState>();
    engine_state.finished = true;

    // Run to trigger EngineEvent::SimulationFinished (ends immediately)
    let event = sim.run();

    if let EngineEvent::SimulationFinished { steps, name, .. } = event {
        assert_eq!(steps, 1); // one step executed
        assert_eq!(name, "zero_agents");
    } else {
        panic!("Expected SimulationFinished event");
    }

    // Verify event sent via channel
    let received = rx.recv().unwrap();
    if let EngineEvent::SimulationFinished { steps, name, .. } = received {
        assert_eq!(steps, 1);
        assert_eq!(name, "zero_agents");
    } else {
        panic!("Expected received SimulationFinished");
    }

    // Verify stats reflect zero snakes/species
    let stats = sim.world.resource::<Stats>();
    assert_eq!(stats.total_snakes, 0);
    assert!(stats.species.species.is_empty());

    // No panics (test completed successfully)
}

#[test]
fn test_grow_panic_index_out_of_bounds() {
    let mut world = World::new();

    let config = SimulationConfig {
        rows: 5,
        columns: 5,
        new_segment_cost: 50.0,
        ..Default::default()
    };
    world.insert_resource(config);
    world.insert_resource(SegmentMap {
        map: Map3d::new(5, 5),
    });
    world.insert_resource(InnovationTracker::new());

    let mut dna = Dna {
        genes: vec![
            Gene {
                segment_type: SegmentType::muscle(),
                id: 0,
                jump: 1,
            },
            Gene {
                segment_type: SegmentType::solid(),
                id: 1,
                jump: 2,
            },
            Gene {
                segment_type: SegmentType::solar(),
                id: 2,
                jump: 3,
            },
            Gene {
                segment_type: SegmentType::stomach(),
                id: 3,
                jump: 4,
            },
            Gene {
                segment_type: SegmentType::muscle(),
                id: 4,
                jump: 0,
            },
        ],
        current_gene: 0,
    };

    // Use build_segment 4 times to increment current_gene to 4
    for _ in 0..4 {
        dna.build_segment();
    }

    // Apply mutate_specific to remove gene at index 0
    let mut rng = tinyrand::Wyrand::seed(42);
    let config = MutationConfig::default();
    dna.mutate_specific(MutationType::RemoveGene, &mut rng, &config);
    // Now dna has 4 genes, current_gene=4 is out of bounds

    let mut rng = tinyrand::Wyrand::seed(42);
    let brain = Box::new(RandomNeuralBrain::new(
        world.resource_mut::<InnovationTracker>().as_mut(),
        &mut rng,
    ));
    let (pos, meat, mut snake, age, justborn) = create_snake(0.0, (0, 0), brain, dna, &mut rng);

    snake.energy.accumulated_meat_matter_for_growth = 50.0;
    snake.last_position = (2, 2);

    let head_id = world.spawn((pos, meat, snake, age, justborn)).id();

    // Run setup systems
    let mut setup_schedule = Schedule::default();
    setup_schedule.add_systems(assign_missing_segments);
    setup_schedule.add_systems(assign_segment_positions);
    setup_schedule.run(&mut world);

    // Record segments length after setup
    let segments_len_after_setup = {
        let snake = world.query::<&Snake>().single(&world);
        snake.segments.len()
    };

    // Run grow system, which should not panic after fix
    let mut grow_schedule = Schedule::default();
    grow_schedule.add_systems(grow);
    grow_schedule.run(&mut world);

    // Assertions for expected behavior
    {
        let snake = world.query::<&Snake>().single(&world);
        assert_eq!(snake.energy.accumulated_meat_matter_for_growth, 0.0);
        assert_eq!(snake.segments.len(), segments_len_after_setup + 1);
        assert_eq!(snake.dna.current_gene, 1);
    }
}
