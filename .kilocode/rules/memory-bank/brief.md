Hex Brains is an evolutionary simulation engine built in Rust, designed to model the emergence of intelligent behavior in snake-like agents within a hexagonal grid world.

Main Objectives:
- Simulate natural selection and evolution through agent reproduction, mutation, and survival challenges.
- Explore how neural network-based brains enable adaptive decision-making in response to environmental stimuli.

Key Features:
- Hexagonal grid-based world with food sources, scents, and optional walls.
- Agents (snakes) equipped with sensory inputs (vision, scents) and neural or random brains for actions like movement and waiting.
- Evolutionary mechanics including reproduction, DNA/neural mutations, growth, starvation, and species clustering.
- Real-time GUI for visualization, configuration, and batch simulation control.
- Parallel batch simulations for efficient experimentation.

Used Technologies:
- Rust for performance-critical simulation logic.
- Bevy ECS framework for entity management and system scheduling.
- egui for the interactive graphical user interface.
- Rayon for parallel processing in batch runs.
- Custom neural network implementation supporting innovation tracking for evolving topologies.

Significance:
Hex Brains serves as an educational and research tool for artificial life simulations, demonstrating principles of evolutionary algorithms, neural evolution, and emergent complexity in a visually engaging manner. It allows users to experiment with parameters affecting evolution, providing insights into biological and AI-inspired adaptive systems.