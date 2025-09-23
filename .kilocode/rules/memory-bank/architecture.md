# System Architecture: Hex Brains

## Overview
Hex Brains is a Rust workspace comprising two crates:
- **engine**: Core simulation logic using Bevy ECS for entity-component-system management. Handles world simulation, agent behavior, evolution, and batch processing.
- **gui**: Interactive frontend using egui/eframe, integrating the engine for real-time visualization, configuration, and control.

The architecture separates concerns: the engine provides headless simulation capabilities (single or batch runs), while the GUI wraps it for user interaction. Communication between GUI and engine uses channels for events (e.g., stats updates) and commands (e.g., add snakes).

## Source Code Paths
- **engine/src/**:
  - `core.rs`: Defines ECS components (e.g., [`Snake`](engine/src/core.rs:82), [`Food`](engine/src/core.rs:336), [`Scent`](engine/src/core.rs:336)), resources (e.g., [`FoodMap`](engine/src/core.rs:235), [`ScentMap`](engine/src/core.rs:235)), and core systems (e.g., `think`, `movement`, `reproduce`, `split`).
  - `dna.rs`: Manages genetic aspects with [`Dna`](engine/src/dna.rs:98) and [`SegmentType`](engine/src/dna.rs:12) enums (muscle, solid, solar, stomach), including mutation logic.
  - `neural.rs`: Implements neural evolution with [`NeuralNetwork`](engine/src/neural.rs:92) (NEAT-inspired, with nodes/connections), [`InnovationTracker`](engine/src/neural.rs:17) for topology evolution, and activation functions.
  - `simulation.rs`: Orchestrates simulation lifecycle with [`Simulation`](engine/src/simulation.rs:16) struct, Bevy schedules (core, secondary, GUI), configs ([`SimulationConfig`](engine/src/simulation.rs:137), [`MutationConfig`](engine/src/simulation.rs:80)), and enums for events/commands.
  - `simulation_manager.rs`: Handles parallel batch simulations using Rayon.
  - `lib.rs`: Exports public modules and basic tests.
- **gui/src/**:
  - `main.rs`: Entry point with [`MyEguiApp`](gui/src/main.rs:322) implementing `eframe::App`. Includes drawing systems (e.g., `draw_hexes`, `draw_neural_network`), config setup, and event/command handling.

## Key Technical Decisions
- **Hexagonal Grid**: Uses axial coordinates (`Position { x: i32, y: i32 }`) for efficient neighbor calculations. World size configurable via `SimulationConfig { rows, columns }`.
- **Entity-Component-System (ECS)**: Bevy ECS for scalable entity management. Snakes are composed entities (head + segments); resources like maps store world state to avoid per-entity storage.
- **Neural Evolution**: Custom NEAT implementation with innovation numbers to track structural mutations. Brains use trait [`Brain`](engine/src/core.rs:60) for polymorphism (RandomBrain for baseline, RandomNeuralBrain for evolved).
- **Evolution Mechanics**: Energy-based fitness (metabolism costs for actions/thinking); reproduction via splitting (when size >= threshold); mutations on DNA (segment types) and neural weights/connections; species clustering by gene similarity.
- **Parallelism**: Rayon for batch simulations (`simulate_batch`); Bevy's multi-threaded ECS for single sim performance.
- **Visualization**: Egui for immediate-mode UI; custom drawing for hex grid (circles for hexes, lines for neural nets) with color-coding (e.g., green for plants, red for meat).
- **Profiling/Logging**: Puffin for runtime profiling (integrates with egui); Tracing for debug logs (e.g., energy levels, splits).

## Design Patterns
- **ECS Pattern**: Entities (snakes, food) with components (Position, Snake, Energy); systems query and mutate in schedules; resources for global state (maps, configs).
- **Trait Polymorphism**: `Brain` trait allows extensible decision-making (random vs. neural); easy to add new brain types.
- **Builder/Factory**: Functions like `create_snake` for entity spawning; configs as resources for parameterization.
- **Observer/Event-Driven**: Channels (`Sender<EngineEvent>`, `Receiver<EngineCommand>`) for loose coupling between GUI and engine.
- **Immutable by Default**: Systems use queries for read-only access where possible; mutations explicit via `ResMut`.
- **Modular Schedules**: Bevy schedules separate concerns (core: logic; secondary: updates; GUI: rendering queries).

## Component Relationships
- **Agents (Snakes)**: Head entity with `Snake` (brain, direction, energy, DNA, metabolism, segments vec); segments as child entities with `SegmentType` (mutated via DNA). Relationships: Head queries segments for growth/split; species assigned based on neural gene similarity.
- **World Elements**: `Food` (plant/meat energy) in `FoodMap`; `Scent` entities diffuse in `ScentMap`; `Solid` for walls/obstacles in `SolidsMap`. Interactions: Vision cones query maps for sensory inputs (scent, food, solids in front/left/right).
- **Evolution**: `Species` resource tracks clusters; `InnovationTracker` ensures unique mutations. Reproduction: Split creates new snake with mutated brain/DNA; starvation/collisions despawn and add to food.
- **Simulation Flow**: `Simulation::step` runs schedules: sense (scents/vision) → decide (`think` via brain) → act (movement/eat/grow) → evolve (split/age/mutate/species).
- **GUI Integration**: Queries engine world for drawing (positions, scents, food, snakes); sends commands (e.g., CreateSnakes) and receives events (stats, done).

## Critical Implementation Paths
- **Single Simulation Loop** (`Simulation::run`): Initializes world with food/snakes; loops `step` until done (e.g., no snakes); chains systems: `think` → `increase_age` → `calculate_stats` → `diffuse_scents` → `movement` → `update_positions` → `split` → `eat_food` → `destroy_old_food`; secondary: `assign_solid_positions` → `assign_segment_positions` → `turn_counter` → `disperse_scents` → `despawn_food`.
- **Batch Simulation** (`simulate_batch`): Parallelizes multiple `Simulation::run` via Rayon; aggregates events/stats for analysis.
- **Neural Decision** (`NeuralNetwork::run`): Feeds 18 inputs (bias, chaos, scents x3, visions x9, levels x3) through nodes/connections with activations (sigmoid/tanh); outputs decision probabilities → argmax for action.
- **Mutation/Reproduction** (`split`, `Dna::mutate`): On split, halve energy/DNA, mutate (add/remove/perturb connections, segment types); calculate species diff via gene distance.
- **Sensory Processing** (`think`): Computes inputs from maps (scent levels, vision rays up to range); brain decides; costs energy based on metabolism/age efficiency.

This architecture ensures modularity, performance (via ECS/parallelism), and extensibility (traits, configs) for evolutionary experiments.