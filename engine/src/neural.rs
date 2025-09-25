use bevy_ecs::prelude::Resource;
use tinyrand::{Rand, RandRange};
use std::collections::HashMap;
use tracing::debug;

// Define a trait that all sensor inputs will implement.
#[derive(Debug, Clone)]
pub struct SensorInput {
    pub value: f32,
    pub index: usize,
}

type InnovationNumber = usize;

#[derive(Default, Resource, Clone)]
pub struct InnovationTracker {
    current_innovation: InnovationNumber,
    innovation_map: HashMap<(usize, usize), InnovationNumber>,
}

impl InnovationTracker {
    pub fn new() -> Self {
        InnovationTracker::default()
    }

    fn get_innovation_number(&mut self, in_node: usize, out_node: usize) -> usize {
        // Check if the innovation (i.e., connection) already exists
        let node_pair = (in_node, out_node);
        *self.innovation_map.entry(node_pair).or_insert_with(|| {
            // If not, assign a new innovation number
            let new_innovation = self.current_innovation;
            self.current_innovation += 1;
            new_innovation
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionGene {
    pub in_node: usize,
    pub out_node: usize,
    pub weight: f32,
    pub enabled: bool,
    pub innovation_number: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeType {
    Input,
    Hidden,
    Output,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Activation {
    Sigmoid,
    Relu,
    Tanh,
    None,
}

impl Activation {
    pub fn apply(&self, input: f32) -> f32 {
        match self {
            Activation::Sigmoid => 1.0 / (1.0 + (-input).exp()),
            Activation::Relu => input.max(0.0),
            Activation::Tanh => input.tanh(),
            Activation::None => panic!("Cannot apply activation function None"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeGene {
    pub node_type: NodeType,
    activation: Activation,
}

impl NodeGene {
    pub fn new(node_type: NodeType, activation: Activation) -> Self {
        NodeGene {
            node_type,
            activation,
        }
    }

    pub fn activate(&self, input: f32) -> f32 {
        self.activation.apply(input)
    }
}

// Your neural network with a generic vector for input values.
#[derive(Clone, Debug, PartialEq)]
pub struct NeuralNetwork {
    nodes: Vec<NodeGene>,
    pub connections: Vec<ConnectionGene>,
}

impl NeuralNetwork {
    pub fn new(input_activations: Vec<Activation>, output_activations: Vec<Activation>) -> Self {
        let mut network = NeuralNetwork {
            nodes: Vec::new(),
            connections: Vec::new(),
        };

        // Initialize input nodes with their respective activation functions
        for activation in input_activations {
            network.nodes.push(NodeGene {
                node_type: NodeType::Input,
                activation,
            });
        }

        // Initialize output nodes with their respective activation functions
        for activation in output_activations {
            network.nodes.push(NodeGene {
                node_type: NodeType::Output,
                activation,
            });
        }
        network
    }

    pub fn random_brain(
        total_inputs: usize,
        connection_active_probability: f32,
        innovation_tracker: &mut InnovationTracker,
        rng: &mut tinyrand::SplitMix,
    ) -> NeuralNetwork {
        // Define input activations: one for bias (using ReLU to keep it at 1) and one for the actual input.
        let input_activations = vec![Activation::Relu; total_inputs];

        // For outputs, we initially choose Sigmoid, as we want to simulate probabilities. Later, we'll apply softmax.
        let output_activations = vec![Activation::Sigmoid; 4];

        let mut network = NeuralNetwork::new(input_activations.clone(), output_activations.clone());
        for (i, _) in input_activations.iter().enumerate() {
            for (j, _) in output_activations.iter().enumerate() {
                let weight = ((rng.next_u32() as f64) / (u32::MAX as f64) - 0.5) as f32;
                let active = (rng.next_u32() as f64) / (u32::MAX as f64) < connection_active_probability as f64;
                network.add_connection(
                    i,
                    j + input_activations.len(),
                    weight,
                    active,
                    innovation_tracker.get_innovation_number(i, j),
                )
            }
        }
        network
    }

    pub fn add_connection(
        &mut self,
        in_node: usize,
        out_node: usize,
        weight: f32,
        enabled: bool,
        innovation_number: InnovationNumber,
    ) {
        let connection = ConnectionGene {
            in_node,
            out_node,
            weight,
            enabled,
            innovation_number,
        };
        assert!(
            in_node < self.nodes.len(),
            "The input node index is out of bounds"
        );
        assert!(
            out_node < self.nodes.len(),
            "The output node index is out of bounds"
        );
        self.connections.push(connection);
    }

    pub fn flip_random_connection(&mut self) {
        let mut rng = tinyrand::SplitMix::default();
        let index = rng.next_range(0..self.connections.len());
        debug!("Flipping connection {}", index);
        self.connections[index].enabled = !self.connections[index].enabled;
    }

    pub(crate) fn mutate_perturb_random_connection_weight(
        &mut self,
        mutation_strength: f32,
        perturb_disabled_connections: bool,
    ) {
        let mut rng = tinyrand::SplitMix::default();
        let mut index;
        let active_connections = self.get_active_connections();
        if perturb_disabled_connections || active_connections.is_empty() {
            index = rng.next_range(0..self.connections.len());
        } else {
            index = rng.next_range(0..self.get_active_connections().len());
            index = self
                .connections
                .iter()
                .position(|c| active_connections.get(index).unwrap() == &c)
                .unwrap();
        }
        self.connections[index].weight += ((rng.next_u32() as f32) / (u32::MAX as f32)) * (mutation_strength * 2.0) - mutation_strength;
        debug!(
            "Mutating connection {} to value {}",
            index, self.connections[index].weight
        );
    }

    pub(crate) fn mutate_reset_random_connection_weight(
        &mut self,
        mutation_strength: f32,
        perturb_disabled_connections: bool,
    ) {
        let mut rng = tinyrand::SplitMix::default();
        let mut index;
        let active_connections = self.get_active_connections();
        if perturb_disabled_connections || active_connections.is_empty() {
            index = rng.next_range(0..self.connections.len());
        } else {
            index = rng.next_range(0..self.get_active_connections().len());
            index = self
                .connections
                .iter()
                .position(|c| active_connections.get(index).unwrap() == &c)
                .unwrap();
        }
        self.connections[index].weight = ((rng.next_u32() as f32) / (u32::MAX as f32)) * (mutation_strength * 2.0) - mutation_strength;
        debug!(
            "Mutating connection {} to value {}",
            index, self.connections[index].weight
        );
    }

    pub fn get_active_connections(&self) -> Vec<&ConnectionGene> {
        self.connections
            .iter()
            .filter(|connection| connection.enabled)
            .collect()
    }

    pub fn get_nodes(&self) -> Vec<&NodeGene> {
        self.nodes.iter().collect()
    }

    pub fn run_cost(&self) -> f32 {
        let active_connections = self.get_active_connections();
        let think_cost = active_connections.len() as f32 * 0.15
            + active_connections
                .iter()
                .map(|c| c.weight.abs())
                .sum::<f32>()
                * 0.1;
        think_cost + 0.01
    }

    pub fn run(&self, inputs: Vec<SensorInput>) -> Vec<f32> {
        debug!("Running network with inputs: {:?}", inputs);
        debug!("Nodes len: {}", self.nodes.len());
        let mut node_values = vec![0.0; self.nodes.len()];

        // Set initial values for input nodes based on SensorInput
        for input in inputs {
            let index = input.index;
            if index < self.nodes.len() && matches!(self.nodes[index].node_type, NodeType::Input) {
                debug!("Setting input node {} to {}", index, input.value);
                node_values[index] = input.value;
            }
        }

        // Propagate values through the network
        for connection in &self.connections {
            if connection.enabled {
                let input_value = node_values[connection.in_node];
                node_values[connection.out_node] += input_value * connection.weight;
                debug!(
                    "Propagating value {} from node {} to node {}",
                    input_value, connection.in_node, connection.out_node
                )
            }
        }

        // Apply activation functions to all nodes (skipping input nodes)
        for (i, node) in self.nodes.iter().enumerate() {
            if matches!(node.node_type, NodeType::Hidden | NodeType::Output) {
                node_values[i] = node.activation.apply(node_values[i]);
                debug!(
                    "Applying activation function to node {} with value {}",
                    i, node_values[i]
                );
            }
        }

        // Extract the output values and return them
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                if matches!(node.node_type, NodeType::Output) {
                    Some(node_values[i])
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Activation, NeuralNetwork, SensorInput};
    #[test]
    fn test_neural_network_forward_pass() {
        let input_activations = vec![Activation::None; 2];
        let output_activations = vec![Activation::Sigmoid];
        let mut network = NeuralNetwork::new(input_activations, output_activations);
        network.add_connection(0, 2, 0.5, true, 0);
        network.add_connection(1, 2, 0.5, true, 1);
        let inputs = vec![
            SensorInput {
                value: 1.0,
                index: 0,
            },
            SensorInput {
                value: 1.0,
                index: 1,
            },
        ];
        let outputs = network.run(inputs);
        let expected = 1.0 / (1.0 + (1.0f32).exp().recip());
        assert!((outputs[0] - expected).abs() < 1e-6);
    }

    #[test]
    fn test_neural_network_empty_forward_pass() {
        let input_activations = vec![Activation::None; 2];
        let output_activations = vec![Activation::Sigmoid];
        let network = NeuralNetwork::new(input_activations, output_activations);
        let inputs = vec![
            SensorInput {
                value: 1.0,
                index: 0,
            },
            SensorInput {
                value: 1.0,
                index: 1,
            },
        ];
        let outputs = network.run(inputs);
        assert!((outputs[0] - 0.5).abs() < 1e-6);
    }

    //
    // struct FloatInput {
    //     value: f32,
    //     index: usize,
    // }
    //
    // struct ConstantInput {
    //     index: usize,
    // }
    //
    // impl SensorInput for FloatInput {
    //     fn as_float(&self) -> f32 {
    //         self.value
    //     }
    //
    //     fn index(&self) -> usize {
    //         self.index
    //     }
    // }
    //
    // impl SensorInput for ConstantInput {
    //     fn as_float(&self) -> f32 {
    //         1.0
    //     }
    //
    //     fn index(&self) -> usize {
    //         self.index
    //     }
    // }
    //
    // #[test]
    // fn test_random_brain_even_distribution() {
    //     let mut innovation_tracker = InnovationTracker::new(); // Assuming you have an InnovationTracker implementation
    //     let brain = NeuralNetwork::random_brain(&mut innovation_tracker);
    //     let mut distribution = [0; 4]; // This will hold the count of times each output node has the highest activation
    //
    //     for i in 0..100 {
    //         let input_value = i as f32 * 0.01; // Generate the input
    //         let outputs = brain.run(vec![
    //             FloatInput { value: input_value, index: 0 }, // Bias input
    //             FloatInput { value: 1.0, index: 1 }, // Varying input
    //         ]);
    //
    //         let (max_index, _) = outputs
    //             .iter()
    //             .enumerate()
    //             .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
    //             .unwrap();
    //         distribution[max_index] += 1;
    //     }
    //
    //     // Verify that each action is selected approximately 25% of the time.
    //     // The distribution can be perfectly even because the weights are deterministic.
    //     for &count in &distribution {
    //         assert_eq!(count, 25, "Each output should be selected exactly 25 times for an even distribution. Found: {:?}", distribution);
    //     }
    // }
    // #[test]
    // fn test_run_network() {
    //     let mut innovation_tracker = InnovationTracker::new();
    //     let mut network = NeuralNetwork::new(2, 2, &mut innovation_tracker);
    //
    //     // Let's enable all connections and set some weights
    //     for connection in &mut network.connections {
    //         connection.enabled = true;
    //         connection.weight = 1.0; // Setting all weights to 1 for simplicity
    //     }
    //
    //     // Set inputs
    //     network.inputs = vec![1.0, 2.0]; // For two inputs
    //
    //     // Run the network
    //     network.run();
    //
    //     // Since all weights are 1 and all connections are enabled,
    //     // each output should be the sum of the inputs.
    //     assert_eq!(network.outputs[0], 3.0);
    //     assert_eq!(network.outputs[1], 3.0);
    // }
    //
    // #[test]
    // fn test_fully_connected_neural_network_initialization() {
    //     // For example, if you have 3 inputs and 2 outputs, there should be 3 * 2 = 6 connections
    //     let input_size = 3;
    //     let output_size = 2;
    //     let mut innovation_tracker = InnovationTracker::new();
    //     let nn = NeuralNetwork::new(input_size, output_size, &mut innovation_tracker);
    //
    //     // Assuming 'connections' is a Vec<Connection> and Connection is a struct representing a connection
    //     let expected_connections_count = input_size * output_size;
    //     assert_eq!(nn.connections.len(), expected_connections_count, "There should be {} connections", expected_connections_count);
    //
    //     // Now let's ensure that they are all initially disabled
    //     for connection in &nn.connections {
    //         assert!(!connection.enabled, "All connections should initially be disabled");
    //     }
    // }
    //
    // #[test]
    // fn test_add_connection() {
    //     let mut innovation_tracker = InnovationTracker::new();
    //     let mut nn = NeuralNetwork::new(3, 2, &mut innovation_tracker);
    //
    //     // Test adding a connection with a given innovation number
    //     let innovation_number = innovation_tracker.get_innovation_number(0, 3);
    //     nn.add_connection(0, 3, 0.5, innovation_number);
    //     assert_eq!(nn.connections.len(), 3 * 2 + 1);
    //     assert_eq!(nn.connections.last().unwrap().innovation_number, innovation_number);
    // }
    //
    // #[test]
    // #[should_panic(expected = "The input node index is out of bounds")]
    // fn test_add_connection_input_out_of_bounds() {
    //     let mut innovation_tracker = InnovationTracker::new();
    //     let mut network = NeuralNetwork::new(5, 4, &mut innovation_tracker); // assuming 5 input nodes and 3 output nodes
    //     let innovation_number = 1;
    //     // Attempt to add a connection with an invalid input node index
    //     network.add_connection(10, 1, 0.5, innovation_number);
    // }
    //
    // #[test]
    // #[should_panic(expected = "The output node index is out of bounds")]
    // fn test_add_connection_output_out_of_bounds() {
    //     let mut innovation_tracker = InnovationTracker::new();
    //     let mut network = NeuralNetwork::new(5, 4, &mut innovation_tracker); // assuming 5 input nodes and 3 output nodes
    //     let innovation_number = 2;
    //     // Attempt to add a connection with an invalid output node index
    //     network.add_connection(1, 10, 0.5, innovation_number);
    // }
    //
    // #[test]
    // fn test_innovation_database_assigns_unique_numbers() {
    //     let mut db = InnovationTracker::new();
    //
    //     let innovation1 = db.get_innovation_number(1, 2);
    //     let innovation2 = db.get_innovation_number(3, 4);
    //
    //     assert_ne!(innovation1, innovation2, "Innovations should be unique");
    // }
    //
    // #[test]
    // fn test_innovation_database_reuses_numbers_for_same_connection() {
    //     let mut db = InnovationTracker::new();
    //
    //     let first_call = db.get_innovation_number(1, 2);
    //     let second_call = db.get_innovation_number(1, 2);
    //
    //     assert_eq!(first_call, second_call, "Should reuse innovation number for the same connection");
    // }
    //
    // #[test]
    // fn test_innovation_database_continues_incrementing_after_reuse() {
    //     let mut db = InnovationTracker::new();
    //
    //     let _ = db.get_innovation_number(1, 2);
    //     let _ = db.get_innovation_number(1, 2); // reuse the same to get the same number
    //     let innovation3 = db.get_innovation_number(2, 3);
    //
    //     // Assuming the innovation numbers start at 0 and increment by 1
    //     assert_eq!(innovation3, 1, "The third unique connection should have innovation number 1");
    // }
}
