# Hex Brains

[![Rust](https://img.shields.io/badge/Rust-1.0-blue?logo=rust)](https://www.rust-lang.org/)
[![Bevy ECS](https://img.shields.io/badge/Bevy_ECS-0.12-orange?logo=bevy)](https://bevyengine.org/)

Hex Brains is an evolutionary simulation engine built in Rust, designed to model the emergence of intelligent behavior in snake-like agents within a hexagonal grid world. It combines artificial life principles with neural evolution, allowing users to experiment with natural selection, mutation, and adaptive strategies in an engaging, visual environment.

This project serves as both an educational tool for understanding AI evolution and a research platform for exploring emergent complexity. Watch as simple agents evolve brains to navigate, forage, and survive in a dynamic world filled with food, scents, and challenges.

## Features

- **Hexagonal Grid World**: Efficient 2D simulation on axial-coordinate hex grids with configurable size, food distribution (plants/meat), scents, and optional walls/obstacles.
- **Evolvable Agents**: Snake-like creatures with modular bodies (DNA-defined segments: muscle, solid, solar, stomach). Agents perceive via vision cones and scents, decide actions using neural or random brains, and manage energy through metabolism.
- **Neural Evolution**: Custom NEAT-inspired neural networks that evolve topologies, weights, and connections over generations. Supports innovation tracking for structural mutations and species clustering based on genetic similarity.
- **Evolution Mechanics**: Reproduction via splitting/growth when energy thresholds are met; mutations on DNA and neural structures; natural selection through starvation, collisions, and energy costs for actions/thinking.
- **Real-Time Visualization**: Interactive GUI built with egui/eframe, rendering the grid, agent movements, scent diffusion, and neural network diagrams. Includes controls for pausing, stepping, adding agents, and inspecting individuals.
- **Performance Tools**: Integrated Puffin profiling for runtime analysis and Tracing for detailed logging. Multi-threaded Bevy ECS ensures scalability for large worlds and populations.
- **Extensibility**: Trait-based brain polymorphism (easy to add custom decision-makers); configurable parameters for mutations, world setup, and simulation rules.

## Quick Start

### Prerequisites

- Rust toolchain (stable channel, edition 2021): Install via [rustup](https://rustup.rs/).
- Git for cloning the repository.

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/hex-brains.git
   cd hex-brains
   ```

2. Build the project:
   ```
   cargo build
   ```

   For an optimized release build:
   ```
   cargo build --release
   ```

### Running the Project

- **Launch the GUI** (interactive simulation viewer):
  ```
  cargo run --bin hex_brains_gui
  ```
  This starts the egui application. You'll see a hexagonal grid world with initial agents and food. Use the UI to configure parameters (e.g., world size, mutation rates, starting population), add snakes, and run/pause the simulation.

- **Profiling and Logging**:
  - Enable profiling: Access via `http://localhost:29617` while running the GUI.
  - Debug logs: `RUST_LOG=debug cargo run --bin hex_brains_gui`.

## Usage and What to Expect

### Single Simulation (GUI Mode)
- **Setup**: Adjust sliders for grid dimensions (e.g., 50x50 hexes), food spawn rates, agent count (start with 10-50), and evolution configs (mutation probability ~0.01-0.05).
- **Running**: Click "Start" to begin. Agents will move, eat, grow, and reproduce. Observe emergent behaviors like foraging patterns or pack hunting as generations progress.
- **Visualization**:
  - Grid shows agents as colored snakes (head + segments), green/red dots for food, fading scents.
  - Select an agent to view its neural network diagram (nodes/connections light up during decisions).
  - Stats panel displays population, average energy/fitness, species diversity, and generation count.
- **Expectations**: Early generations show random survival; over 100-500 steps, neural agents adapt (e.g., seeking food efficiently). Simulations may end when all agents die—restart or tweak params for longer runs. Performance: Smooth at 60 FPS for medium grids; larger worlds benefit from release builds.

### Customization
- Extend brains: Implement the `Brain` trait in `engine/src/core.rs` for new decision logic (e.g., reinforcement learning).
- World mods: Add entities/systems via Bevy ECS in `simulation.rs`.
- Experiments: Vary inputs like vision range (default 3 hexes) or energy costs to study adaptation.

No screenshots are included yet, but the GUI provides an intuitive, real-time view—run it to explore!

## Architecture Overview

Hex Brains uses a modular Rust workspace:
- **engine**: Headless core with Bevy ECS for entities (snakes, food), systems (think, move, evolve), and resources (maps for food/scents).
- **gui**: eframe wrapper integrating the engine for rendering and controls via channels.

Key loop: Sense environment → Brain decides → Act (move/eat/grow) → Evolve (mutate/split). Multi-threaded ECS for efficient single runs.

For deeper details, see [architecture.md](.kilocode/rules/memory-bank/architecture.md) in the memory bank.

## Contributing

Contributions are welcome! Please:
1. Fork the repo and create a feature branch.
2. Ensure code passes `cargo check`, `cargo test`, `cargo fmt`, and `cargo clippy`.
3. Add tests for new features.
4. Update documentation (e.g., memory bank files).
5. Submit a PR with a clear description.

Focus areas: More brain types, export formats, or cross-platform optimizations.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. (Add a LICENSE file if not present.)

---

*Built with ❤️ for artificial life enthusiasts. Explore, evolve, and enjoy!*
