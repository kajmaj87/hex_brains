# Current Context

## Work Focus
The Memory Bank for Hex Brains has been initialized with core documentation files. Current focus is verifying accuracy, refining contents based on user feedback, and planning expansions such as additional documentation and testing.

## Recent Changes
- Analyzed project structure using semantic search and code definition listing across engine and GUI crates.
- Created product.md detailing project purpose, problems solved, functionality, and UX goals.
- Created architecture.md documenting system design, component relationships, and key implementation paths.
- Created tech.md outlining technologies, dependencies, development setup, and constraints.
- Confirmed existing brief.md aligns with project as an evolutionary simulation engine using Bevy ECS for snake agents in a hex grid world.
- Fixed bug in DNA mutation jump adjustment to prevent underflow when removing genes.
- Fixed condition in grow system to prevent index out of bounds when current_gene equals genes length.
- Updated integration test to use proper mocking for RNG to avoid hangs.
- Fixed bug in DNA mutation current_gene adjustment to prevent underflow when removing genes at index 0 with current_gene 0.
- Refactored RNG usage to pass by Rand interface, using Wyrand as the concrete implementation. Replaced Brain trait with BrainType enum for better type safety and to avoid dyn compatibility issues.
- Modified GUI to automatically start simulation on program launch with 200x speed and 10 initial snakes.
- Modified speed limiting logic in simulation.rs to cap overflow frames at speed_limit (1 second worth) to prevent unlimited accumulation when simulation runs slower than the limit.
- Updated Bevy ECS dependency from 0.12.0 to 0.13.0 in both engine and GUI crates. No code changes required as the project uses only ECS features unaffected by the migration.
- Updated Bevy ECS dependency from 0.13.0 to 0.14.0 in both engine and GUI crates, renaming the "multi-threaded" feature to "multi_threaded". No code changes required as the project uses only ECS features unaffected by the migration.
- Updated Bevy ECS dependency from 0.14.0 to 0.15.0 in both engine and GUI crates. Fixed test assertions for `World::get_entity()` returning `Result` instead of `Option`. No other code changes required as the project uses only ECS features unaffected by the migration.
- Fixed Windows cross-compilation issue by adding winapi dependency with "winuser" and "windef" features to gui/Cargo.toml to enable necessary Windows API modules for eframe.

## Next Steps
- Verify memory bank contents with user for accuracy and completeness.
- Expand project documentation (e.g., update README.md with setup instructions and examples).
- Consider adding unit and integration tests for core systems (e.g., neural evolution, simulation steps).
- Explore additional memory bank files if needed (e.g., tasks.md for repetitive workflows).