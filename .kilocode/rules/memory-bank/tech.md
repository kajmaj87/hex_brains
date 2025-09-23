# Technologies and Setup: Hex Brains

## Technologies Used
- **Rust**: Core language for the entire project, chosen for its performance, memory safety, and concurrency support. Uses the 2021 edition.
- **Bevy ECS (0.12.0)**: Entity-Component-System framework for managing simulation entities (snakes, food, scents) and scheduling systems. Features multi-threaded execution for better performance.
- **egui and eframe (0.23.0)**: Immediate-mode GUI library for the interactive frontend. egui handles UI rendering, while eframe provides the application framework with cross-platform support.
- **Rayon (1.8.0)**: Parallel iteration library for batch simulations, enabling efficient parallel execution of multiple independent simulations.
- **Custom Neural Network Implementation**: NEAT-inspired neural evolution in `neural.rs`, including `NeuralNetwork`, `NodeGene`, `ConnectionGene`, and `InnovationTracker` for structural mutations.
- **Puffin (0.17.0)**: Profiling tool integrated for runtime performance analysis, with egui integration (`puffin_egui`) and HTTP server (`puffin_http`) for remote profiling.
- **Tracing (0.1.40)**: Structured logging library for debug output (e.g., energy levels, splits), with subscriber support in the GUI.
- **Rand (0.8.5)**: Random number generation for decisions, mutations, and initializations (e.g., random brains, positions).

## Dependencies
The project uses a Cargo workspace with two crates: `hex_brains_engine` and `hex_brains_gui`.

- **engine/Cargo.toml**:
  - `bevy_ecs = { version = "0.12.0", features = ["multi-threaded"] }`
  - `rayon = "1.8.0"`
  - `puffin = "0.17.0"`
  - `rand = "0.8.5"`
  - `tracing = "0.1.40"`

- **gui/Cargo.toml** (inherits engine dependencies):
  - `hex_brains_engine = { path = "../engine" }`
  - `bevy_ecs = { version = "0.12.0", features = ["multi-threaded"] }`
  - `eframe = { version = "0.23.0", features = ["puffin"] }`
  - `egui = "0.23.0"`
  - `puffin = "0.17.0"`
  - `puffin_http = "0.14.0"`
  - `puffin_egui = "0.23.0"`
  - `tracing = "0.1.40"`
  - `tracing-subscriber = "0.3.17"`

No external databases or web services; all computation is local.

## Development Setup
- **Prerequisites**: Rust toolchain (stable channel, 2021 edition). Install via `rustup`.
- **Project Structure**: Workspace root with `Cargo.toml` defining members `["engine", "gui"]`. Engine is a library crate; GUI is a binary crate.
- **Building**:
  - Standard build: `cargo build`
  - Optimized release: `cargo build --release`
  - Ultra-optimized: `cargo build --profile ultra-release` (inherits release with LTO=fat, codegen-units=1, panic=abort for maximum performance).
- **Running**:
  - GUI application: `cargo run --bin hex_brains_gui` (starts the interactive simulation viewer).
  - Engine tests: `cargo test` (basic tests in `lib.rs`; expand as needed).
- **Profiling**: Run with Puffin enabled (default in GUI); access via `http://localhost:29617` for remote profiling.
- **Logging**: Enable tracing with `RUST_LOG=debug cargo run` for detailed output.
- **IDE Support**: VS Code with rust-analyzer extension recommended for ECS and async code navigation.

## Technical Constraints
- **Performance Focus**: Designed for large-scale simulations (e.g., 100x100 hex grids, hundreds of agents). Multi-threaded ECS and Rayon parallelism mitigate bottlenecks, but high entity counts may require optimization.
- **No Async I/O**: Purely computational; no networking beyond Puffin HTTP (optional).
- **Headless Mode**: Engine supports non-GUI batch runs via `simulate_batch`, ideal for experiments without visualization.
- **Extensibility Limits**: Custom neural impl is NEAT-inspired but simplified (fixed inputs/outputs); adding new brain types requires trait impls. Hex grid is axial-coordinate based, fixed to 2D.
- **Testing**: Minimal unit tests present; integration tests for systems recommended but not implemented. No fuzzing or property-based testing.
- **Platform**: Cross-platform (Linux/macOS/Windows) via eframe, but profiling/UI best on desktop.

## Tool Usage Patterns
- **Profiling**: Puffin scopes (`puffin::profile_function!()`) in hot paths (e.g., `think`, `movement`); GUI integrates viewer for real-time metrics.
- **Logging**: Tracing spans for simulation steps (e.g., energy calculations, splits); subscriber in GUI for console output.
- **Randomness**: Seeded RNG via `rand::thread_rng()` for reproducibility in tests/mutations.
- **Build Optimization**: Ultra-release profile for production sims; standard release for development.
- **Documentation**: Inline comments for complex logic (e.g., neural run, hex neighbors); expand with docstrings for public APIs.

This setup prioritizes simulation efficiency and ease of experimentation while maintaining Rust's safety guarantees.