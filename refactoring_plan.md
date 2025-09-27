# Refactoring Plan for Hex Brains Project

## Task List Summary

 1. [x] Extract sensory helper functions from `think` in `engine/src/core.rs`.
     - Completed: Created `collect_scent_inputs` returning [f32; 3] for left/front/right scents, and `collect_vision_inputs` returning [f32; 9] for plant/meat/solid visions. Used &Direction in helper signatures to handle non-Copy Direction enum.
 2. [x] Break `simulation::run` method into smaller methods in `engine/src/simulation.rs`.
     - Completed: Extracted `handle_commands`, `simulation_loop`, and `send_events` methods. `handle_commands` processes incoming commands per simulation step, `simulation_loop` performs stepping and collects FrameDrawn events, `send_events` sends all collected events including SimulationFinished. Logic preserved with improved separation of concerns.
 3. [x] Extract mutation application functions from `split` in `engine/src/core.rs`.
     - Completed: Created functions apply_connection_flip, apply_weight_perturbation, apply_weight_reset, apply_dna_mutation. Used &mut Wyrand for RNG to match neural methods. In split, replaced inline mutation calls with function calls, passing config values as needed. No unexpected choices; functions are simple wrappers for clarity and modularity.
 4. [x] Extract shared helper for available segment types in `engine/src/dna.rs`.
      - Completed: Added get_available_segment_types helper function to centralize config-based filtering of segment types, reducing duplication in random and mutate_internal methods. No unexpected choices; function simply encapsulates the filter logic.
 5. [x] Extract shared helper for random connection index in `engine/src/neural.rs`.
      - Completed: Added select_random_connection_index helper function to standardize random index selection from connection slices, used in mutation methods for all connections case. No unexpected choices; helper returns Option for safety.
 6. [x] Split long test function in `engine/src/core.rs`.
    - Completed: Split `test_energy_conservation_invariant` into focused tests: `test_energy_on_movement`, `test_energy_on_thinking`, `test_energy_on_eating`, `test_energy_on_processing_food`, `test_energy_on_aging`, `test_energy_on_starvation`, `test_energy_on_splitting`, `test_energy_on_mutation`. Each test sets up minimal world state and asserts specific energy invariants. No unexpected choices; tests isolated concerns effectively.
 7. [x] Break `update` method into sub-methods in `gui/src/main.rs`.
     - Completed: Extracted `handle_events`, `render_windows`, `render_toolbar`, `render_central_panel`, `handle_keyboard_shortcuts` as private methods in `impl MyEguiApp`. Moved keyboard shortcuts handling outside central panel to `handle_keyboard_shortcuts`. Updated `update` to call these methods in order. No unexpected choices; methods are simple extractions for better organization.
 8. [x] Extract UI helper functions for sliders and checkboxes in `gui/src/main.rs`.
     - Completed: Added `add_drag_value` and `add_checkbox` helper functions, replaced inline UI code in settings windows with calls to these helpers for standardization and reduced duplication. No unexpected choices; helpers use the same tooltip for label and control.
 9. [x] Split `MyEguiApp` struct into sub-structs in `gui/src/main.rs`.
     - Completed: Created three sub-structs: `UiState` (window visibility and UI state), `PerformanceStats` (frame tracking and performance metrics), and `ConfigState` (configuration and data management). Updated `MyEguiApp` to use these sub-structs and systematically updated all field accesses throughout the file. Improved code organization by grouping related fields with clear separation of concerns.
10. [x] Extract drawing functions to a new module in `gui/src/`.
     - Completed: Created `drawing.rs` with `draw_hexes`, `draw_neural_network`, `get_node_position`, `transform_to_circle`, `with_alpha`, and `u32_to_color` functions. Moved all drawing logic from `main.rs` to the new module and updated all function calls and tests to use the drawing module. No unexpected choices; extraction was straightforward and preserved all functionality.
11. [x] Introduce `CommandDispatcher` for engine commands in `gui/src/main.rs`.
     - Completed: Introduced CommandDispatcher struct with sender field and methods for all EngineCommand variants. Replaced direct sender.send() calls with dispatcher method calls throughout MyEguiApp. No unexpected choices; abstraction improves decoupling between UI and engine.
12. [x] Define constants for magic numbers in `gui/src/main.rs`.
    - Completed: Defined constants for window size, speed limit, history limit, smoothing window, performance update interval, and default snakes to add. Replaced literals with constants throughout the file. Colors were not made const due to non-const constructors. No unexpected choices; constants improve tunability without changing visuals.
13. [x] Simplify complex conditionals in GUI rendering.
      - Added early return in Statistics window when no species exist.
      - Extracted render_vision_ranges helper function to flatten nested ui.horizontal chains in Mutation Settings for plant, meat, and obstacle vision ranges.
      - Used guard clauses for vision range rendering only when enabled.
14. [x] Create `sensory.rs` utility module in `engine/src/`.
      - Completed: Moved sensory helper functions from core.rs to new sensory.rs module, added pub mod sensory to lib.rs, and updated calls in think function. Made turn_left and turn_right pub for use in sensory.rs. No unexpected choices; extraction was straightforward and preserved all functionality.
15. [x] Create `mutation.rs` utility module in `engine/src/`.
     - Completed: Moved mutation application functions (apply_connection_flip, apply_weight_perturbation, apply_weight_reset, apply_dna_mutation) from core.rs to new mutation.rs module. Exported in lib.rs. No unexpected choices; extraction was straightforward and preserved functionality.
16. [x] Create `ui_helpers.rs` module in `gui/src/`.
     - Completed: Moved `add_drag_value` and `add_checkbox` helper functions from `main.rs` to new `ui_helpers.rs` module. Added module declaration and imports in `main.rs`. No unexpected choices; extraction was straightforward and preserved functionality.
17. [x] Modularize GUI into submodules.
     - Completed: Created windows.rs with render_*_window functions for each GUI window, and components.rs with render_vision_ranges. Moved code from main.rs render_windows method. No unexpected choices; extraction was straightforward and preserved all functionality.
18. [x] Optimize ECS queries for reduced duplication.
     - Created SensoryCache resource to cache sensory inputs computed once per frame, reducing duplication in think system and enabling potential reuse by other systems.
19. [x] Implement config builders and validation.
     - Completed: Added MutationConfigBuilder and SimulationConfigBuilder structs with fluent methods for all fields and build() methods performing validation. Validation includes range checks for probabilities (0-1), positive values for costs/ranges, and non-zero dimensions. No unexpected choices; builders use defaults from Default impls.
20. [x] Replace `BrainType` enum with `Brain` trait.
     - Completed: Defined `Brain` trait with `decide` and `get_neural_network` methods, implemented for `RandomBrain` and `RandomNeuralBrain`. Changed `Snake.brain` to `Box<dyn Brain>`, updated all usages to use `Box::new` for brain creation. Used `&mut dyn Rand` in trait to make it object-safe. No unexpected choices; trait allows polymorphism for future brain types.
21. [x] Add proper error handling for neural operations.
     - Completed: Introduced NeuralError enum with variants for InvalidActivation, InvalidNodeIndex, NoConnections. Changed Activation::apply, NodeGene::activate, NeuralNetwork::run, add_connection, flip_random_connection, mutate_perturb_random_connection_weight, mutate_reset_random_connection_weight to return Result. Updated mutation.rs functions to propagate errors. Handled Results in core.rs with unwrap_or defaults for run, unwrap for mutations and add_connection. No unexpected choices; errors are propagated where possible, with unwrap in non-Result contexts.
22. [x] Split MyEguiApp struct and implementation into app.rs.
     - Completed: Created app.rs with MyEguiApp struct, UiState, PerformanceStats, ConfigState, CommandDispatcher, and all impl methods. Moved constants to app.rs. Updated main.rs to import app module and instantiate MyEguiApp. No unexpected choices; extraction preserved all functionality and UI interactions.
23. [ ] Extract UI state structs into ui_state.rs.
24. [ ] Create config.rs for configuration management.
25. [ ] Move main function and entry point to minimal main.rs.

## Detailed Task Descriptions

### 1. Extract sensory helper functions from `think` in `engine/src/core.rs`
**Rationale**: The `think` function is 130+ lines with duplicated sensory input collection for directions. Extracting helpers improves readability, reduces duplication, and enhances testability by isolating sensory logic.

**Specific Code Changes**:
- Create helper functions: `fn collect_scent_inputs(world: &World, position: Position) -> [f32; 3]`, `fn collect_vision_inputs(world: &World, position: Position, direction: Direction) -> [f32; 9]`, etc., for each sensory type.
- In `think`, replace inline loops with calls to these helpers, collecting inputs into arrays before passing to brain.
- Ensure helpers are pure functions querying the world.

**Verification Steps**:
- Run `cargo test` to ensure all tests pass, particularly those involving sensory inputs.
- Execute `./verify.sh` to check for compilation and linting issues.
- Manually verify simulation behavior: run GUI and observe agent movements match pre-refactor state.

### 2. Break `simulation::run` method into smaller methods in `engine/src/simulation.rs`
**Rationale**: The `run` method is 180+ lines mixing command processing, simulation loop, and event handling. Breaking it improves separation of concerns and makes the code easier to maintain and debug.

**Specific Code Changes**:
- Extract `fn handle_commands(&mut self) -> Vec<EngineEvent>`, `fn simulation_loop(&mut self) -> Vec<EngineEvent>`, `fn send_events(&self, events: Vec<EngineEvent>)`.
- In `run`, call these methods in sequence, collecting and forwarding events.
- Ensure each method handles its specific responsibility without side effects beyond intended outputs.

**Verification Steps**:
- Run `cargo test` for engine tests.
- Execute `./verify.sh`.
- Test batch simulations and single runs in GUI to confirm event handling and loop execution unchanged.

### 3. Extract mutation application functions from `split` in `engine/src/core.rs`
**Rationale**: The `split` function has inline mutation logic. Extracting dedicated functions for each mutation type (e.g., connection flip, weight perturbation) modularizes the code and reduces the function's length.

**Specific Code Changes**:
- Create functions: `fn apply_connection_flip(dna: &mut Dna, rng: &mut impl Rng)`, `fn apply_weight_perturbation(neural: &mut NeuralNetwork, rng: &mut impl Rng)`, etc.
- In `split`, after creating new snake, call these functions based on mutation types.
- Pass RNG and mutable references to avoid cloning.

**Verification Steps**:
- Run `cargo test`, focusing on mutation-related tests.
- Execute `./verify.sh`.
- Observe evolution in GUI: ensure mutation rates and types produce expected diversity.

### 4. Extract shared helper for available segment types in `engine/src/dna.rs`
**Rationale**: `random` and `mutate_internal` duplicate filtering for available types. A shared helper reduces duplication and centralizes logic.

**Specific Code Changes**:
- Add `fn get_available_segment_types(current: &[SegmentType]) -> Vec<SegmentType> { SegmentType::all().into_iter().filter(|t| !current.contains(t)).collect() }`.
- Replace duplicated code in `random` and `mutate_internal` with calls to this helper.

**Verification Steps**:
- Run `cargo test` for DNA tests.
- Execute `./verify.sh`.
- Test DNA generation and mutation in simulation to ensure no invalid segments.

### 5. Extract shared helper for random connection index in `engine/src/neural.rs`
**Rationale**: Mutation methods duplicate index selection logic. Extracting a helper standardizes and simplifies the code.

**Specific Code Changes**:
- Add `fn select_random_connection_index(connections: &[ConnectionGene], rng: &mut impl Rng) -> Option<usize> { if connections.is_empty() { None } else { Some(rng.gen_range(0..connections.len())) } }`.
- Use this in `mutate_perturb_random_connection_weight` and `mutate_reset_random_connection_weight`.

**Verification Steps**:
- Run `cargo test` for neural tests.
- Execute `./verify.sh`.
- Verify neural evolution: check connection mutations in GUI neural network view.

### 6. Split long test function in `engine/src/core.rs`
**Rationale**: `test_energy_conservation_invariant` is 300+ lines. Splitting into focused tests improves isolation and readability.

**Specific Code Changes**:
- Split into `test_energy_on_movement`, `test_energy_on_thinking`, `test_energy_on_splitting`, etc.
- Each test sets up minimal world state and asserts specific invariants.

**Verification Steps**:
- Run `cargo test` to ensure all new tests pass.
- Execute `./verify.sh`.
- No functional change, so GUI simulation remains stable.

### 7. Break `update` method into sub-methods in `gui/src/main.rs`
**Rationale**: `update` is 900 lines handling multiple concerns. Breaking into methods improves maintainability and debugging.

**Specific Code Changes**:
- Extract `fn handle_events(&mut self, ctx: &egui::Context)`, `fn render_windows(&mut self, ctx: &egui::Context)`, `fn render_toolbar(&mut self, ui: &mut Ui)`, `fn render_central_panel(&mut self, ctx: &egui::Context, ui: &mut Ui)`, `fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context)`.
- In `update`, call these in order.

**Verification Steps**:
- Run `cargo run --bin hex_brains_gui` and test UI interactions.
- Execute `./verify.sh`.
- Ensure all windows, toolbar, and shortcuts work as before.

### 8. Extract UI helper functions for sliders and checkboxes in `gui/src/main.rs`
**Rationale**: Repetitive UI patterns for sliders and checkboxes. Helpers reduce duplication and standardize styling.

**Specific Code Changes**:
- Add `fn add_drag_value<T: emath::Numeric>(ui: &mut Ui, label: &str, value: &mut T, speed: f64, tooltip: &str)`, `fn add_checkbox(ui: &mut Ui, label: &str, value: &mut bool, tooltip: &str)`.
- Replace inline code in settings windows with these calls.

**Verification Steps**:
- Run GUI and check settings windows for correct rendering.
- Execute `./verify.sh`.
- Test slider and checkbox interactions.

### 9. Split `MyEguiApp` struct into sub-structs in `gui/src/main.rs`
**Rationale**: `MyEguiApp` has 30+ fields with mixed concerns. Sub-structs improve organization.

**Specific Code Changes**:
- Create `struct UiState { show_simulation_settings: bool, ... }`, `struct PerformanceStats { frames: usize, ... }`, `struct ConfigState { simulation_config: SimulationConfig, ... }`.
- Embed these in `MyEguiApp` and update field accesses.

**Verification Steps**:
- Compile and run GUI.
- Execute `./verify.sh`.
- Verify all state persists and updates correctly.

### 10. Extract drawing functions to a new module in `gui/src/`
**Rationale**: Drawing logic is mixed in main. A module separates rendering concerns.

**Specific Code Changes**:
- Create `drawing.rs` with `pub fn draw_hexes(...)`, `pub fn draw_neural_network(...)`.
- Move functions from `main.rs` and update calls.

**Verification Steps**:
- Run GUI and check hex grid and neural drawings.
- Execute `./verify.sh`.

### 11. Introduce `CommandDispatcher` for engine commands in `gui/src/main.rs`
**Rationale**: Direct command sending couples UI to engine. A dispatcher abstracts this.

**Specific Code Changes**:
- Create `struct CommandDispatcher { sender: Sender<EngineCommand> }` with methods like `send_add_snakes(&self, count: usize)`.
- Replace direct sends with dispatcher calls.

**Verification Steps**:
- Test command sending in GUI (e.g., add snakes).
- Execute `./verify.sh`.

### 12. Define constants for magic numbers in `gui/src/main.rs`
**Rationale**: Hardcoded values make tuning hard. Constants improve maintainability.

**Specific Code Changes**:
- Define `const INPUT_X_POS: f32 = 0.25;`, `const SMOOTHING_WINDOW: usize = 100;`, etc.
- Replace literals with constants.

**Verification Steps**:
- Run GUI and verify visuals unchanged.
- Execute `./verify.sh`.

### 13. Simplify complex conditionals in GUI rendering
**Rationale**: Nested logic in rendering is hard to follow. Simplifying improves readability.

**Specific Code Changes**:
- Use early returns and guard clauses in window rendering.
- Flatten nested `if` and `ui.horizontal` chains.

**Verification Steps**:
- Run GUI and check window layouts.
- Execute `./verify.sh`.

### 14. Create `sensory.rs` utility module in `engine/src/`
**Rationale**: Sensory helpers are scattered. A module centralizes them for reuse.

**Specific Code Changes**:
- Move extracted sensory functions from task 1 to `sensory.rs`.
- Add `pub mod sensory;` to `lib.rs`.

**Verification Steps**:
- Run tests and simulation.
- Execute `./verify.sh`.

### 15. Create `mutation.rs` utility module in `engine/src/`
**Rationale**: Mutation logic is duplicated. A module shares it.

**Specific Code Changes**:
- Move mutation functions from task 3 to `mutation.rs`.
- Export in `lib.rs`.

**Verification Steps**:
- Run tests and evolution checks.
- Execute `./verify.sh`.

### 16. Create `ui_helpers.rs` module in `gui/src/`
**Rationale**: UI helpers are in main. A module organizes them.

**Specific Code Changes**:
- Move helpers from task 8 to `ui_helpers.rs`.
- Include in `main.rs`.

**Verification Steps**:
- Run GUI.
- Execute `./verify.sh`.

### 17. Modularize GUI into submodules
**Rationale**: `main.rs` is monolithic. Submodules improve structure.

**Specific Code Changes**:
- Create `windows.rs`, `components.rs`, etc., moving code from main.
- Use `mod` declarations.

**Verification Steps**:
- Run GUI.
- Execute `./verify.sh`.

### 18. Optimize ECS queries for reduced duplication
**Rationale**: Overlapping queries cause boilerplate. Optimization reduces it.

**Specific Code Changes**:
- Create shared query builders or cache in resources.
- Update systems to use optimized queries.

**Verification Steps**:
- Run performance tests.
- Execute `./verify.sh`.

### 19. Implement config builders and validation
**Rationale**: Configs have many fields. Builders ensure validity.

**Specific Code Changes**:
- Add builder methods and validation to `SimulationConfig` and `MutationConfig`.

**Verification Steps**:
- Test config loading.
- Execute `./verify.sh`.

### 20. Replace `BrainType` enum with `Brain` trait
**Rationale**: Enum limits extensibility. Trait allows polymorphism.

**Specific Code Changes**:
- Define `trait Brain { fn decide(...) }`, implement for variants.

**Verification Steps**:
- Run simulation with different brains.
- Execute `./verify.sh`.

### 21. Add proper error handling for neural operations
**Rationale**: Panics on errors. Proper handling improves robustness.

**Specific Code Changes**:
- Use `Result` in neural functions, propagate errors.

**Verification Steps**:
- Test edge cases.
- Execute `./verify.sh`.

### 22. Split MyEguiApp struct and implementation into app.rs
**Rationale**: main.rs is 1873 lines with mixed concerns. Moving the core app struct and its methods to a separate file improves modularity and reduces main.rs size.

**Specific Code Changes**:
- Create `app.rs` with `pub struct MyEguiApp { ... }` and its `impl` block, including `new`, `update`, and other methods.
- In `main.rs`, keep only the `main` function and minimal imports, using `mod app;` and instantiating `MyEguiApp`.

**Verification Steps**:
- Run `cargo run --bin hex_brains_gui` to ensure the app starts and functions.
- Execute `./verify.sh`.
- Check that all UI interactions work as before.

### 23. Extract UI state structs into ui_state.rs
**Rationale**: UI-related structs (like UiState, PerformanceStats) are embedded in main. Separating them organizes state management.

**Specific Code Changes**:
- Create `ui_state.rs` with `pub struct UiState { ... }`, `pub struct PerformanceStats { ... }`, etc.
- Move these from `main.rs` or `app.rs`, updating imports.

**Verification Steps**:
- Compile and run GUI.
- Execute `./verify.sh`.
- Verify state persistence and updates.

### 24. Create config.rs for configuration management
**Rationale**: Config loading and handling is mixed in main. A dedicated file centralizes config logic.

**Specific Code Changes**:
- Create `config.rs` with functions for loading/saving configs, validation, and builders.
- Move config-related code from `main.rs`.

**Verification Steps**:
- Test config changes in GUI.
- Execute `./verify.sh`.

### 25. Move main function and entry point to minimal main.rs
**Rationale**: main.rs contains too much logic. Keeping only the entry point improves clarity.

**Specific Code Changes**:
- In `main.rs`, keep only `fn main() { ... }` with eframe setup, importing from other modules.
- Ensure all other code is moved to appropriate files (app.rs, ui_state.rs, etc.).

**Verification Steps**:
- Run the GUI application.
- Execute `./verify.sh`.
- Confirm no functionality loss.