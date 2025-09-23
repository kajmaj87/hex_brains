# Product Overview: Hex Brains

## Why This Project Exists
Hex Brains is an educational and research tool for simulating artificial life and evolutionary algorithms. It models the emergence of intelligent behavior in simple agents (snake-like creatures) within a constrained environment, allowing users to observe how neural networks and genetic mutations lead to adaptive strategies over generations. The project bridges concepts from biology, AI, and complex systems, making abstract ideas tangible through interactive visualization.

## Problems It Solves
- **Lack of Accessible Evolutionary Simulation Tools**: Many existing simulators are either too simplistic (e.g., basic genetic algorithms) or overly complex (e.g., full agent-based modeling frameworks). Hex Brains provides a focused, performant platform for experimenting with neural evolution without requiring deep expertise in simulation design.
- **Visualization of Emergent Behavior**: Understanding how simple rules (e.g., energy management, sensory inputs) lead to complex intelligence is challenging without visual feedback. The tool addresses this by rendering the hexagonal world, agent movements, and neural decisions in real-time.
- **Efficiency in Experimentation**: Researchers and educators need quick iteration on parameters like mutation rates or grid sizes. Hex Brains supports parallel batch simulations to test hypotheses efficiently, reducing computation time for large-scale runs.
- **Educational Gaps in AI Evolution**: It demonstrates key principles like natural selection, speciation, and neuroplasticity in an engaging way, suitable for teaching or self-study.

## How It Should Work
1. **Setup and Configuration**: Users define the world (hex grid size, food distribution, walls) and agent parameters (starting population, mutation configs, brain types: random or neural) via the GUI.
2. **Simulation Execution**:
   - Agents perceive the environment (scents, food, obstacles via vision cones).
   - Brains decide actions (move forward/left/right, wait) based on sensory inputs.
   - Evolution occurs through reproduction (splitting/growing), mutations (DNA for segments, neural weights), and selection (starvation, collisions).
   - Systems handle physics (movement on hex grid), metabolism (energy costs for actions/thinking), and stats tracking (population, species diversity).
3. **Visualization and Control**: Real-time GUI shows the grid with agents, food, scents; neural network diagrams for selected species; controls for pausing, stepping, or running batch experiments.
4. **Output and Analysis**: Collect stats (energy totals, generations, fitness) for export; batch runs aggregate results across multiple simulations for statistical insights.

The core loop runs in discrete steps: sense → decide → act → evolve, powered by an ECS for efficient entity management.

## User Experience Goals
- **Intuitiveness**: Simple GUI with sliders for configs, buttons for common actions (e.g., "Add Snakes", "Run Batch"), and tooltips explaining mechanics.
- **Engagement**: Visually appealing hex grid with color-coded elements (e.g., green for plants, red for meat); animated neural nets showing active connections during decisions.
- **Flexibility**: Easy switching between single sim (for observation) and batch mode (for experiments); zoom/pan on grid; inspect individual agents' brains/DNA.
- **Performance**: Smooth real-time rendering even for large grids/populations; profiling tools (via Puffin) for debugging slowdowns.
- **Accessibility**: Minimal setup (cargo run); clear README with examples; extensible for custom brains or worlds.

This product aims to inspire curiosity about AI evolution while providing a robust platform for serious research.