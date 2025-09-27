use bevy_ecs::prelude::Resource;
use std::collections::HashMap;
use tinyrand::{Rand, RandRange};

// Define a trait that all sensor inputs will implement.
#[derive(Debug, Clone)]
pub struct SensorInput {
    pub value: f32,
    pub index: usize,
}

#[derive(Debug)]
pub enum NeuralError {
    InvalidActivation,
    InvalidNodeIndex,
    NoConnections,
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
    pub fn apply(&self, input: f32) -> Result<f32, NeuralError> {
        match self {
            Activation::Sigmoid => Ok(1.0 / (1.0 + (-input).exp())),
            Activation::Relu => Ok(input.max(0.0)),
            Activation::Tanh => Ok(input.tanh()),
            Activation::None => Err(NeuralError::InvalidActivation),
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

    pub fn activate(&self, input: f32) -> Result<f32, NeuralError> {
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
        rng: &mut impl Rand,
    ) -> NeuralNetwork {
        // Define input activations: one for bias (using ReLU to keep it at 1) and one for the actual input.
        let input_activations = vec![Activation::Relu; total_inputs];

        // For outputs, we initially choose Sigmoid, as we want to simulate probabilities. Later, we'll apply softmax.
        let output_activations = vec![Activation::Sigmoid; 4];

        let mut network = NeuralNetwork::new(input_activations.clone(), output_activations.clone());
        for (i, _) in input_activations.iter().enumerate() {
            for (j, _) in output_activations.iter().enumerate() {
                let weight = ((rng.next_u32() as f64) / (u32::MAX as f64) - 0.5) as f32;
                let active = (rng.next_u32() as f64) / (u32::MAX as f64)
                    < connection_active_probability as f64;
                network
                    .add_connection(
                        i,
                        j + input_activations.len(),
                        weight,
                        active,
                        innovation_tracker.get_innovation_number(i, j),
                    )
                    .unwrap();
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
    ) -> Result<(), NeuralError> {
        if in_node >= self.nodes.len() || out_node >= self.nodes.len() {
            return Err(NeuralError::InvalidNodeIndex);
        }
        let connection = ConnectionGene {
            in_node,
            out_node,
            weight,
            enabled,
            innovation_number,
        };
        self.connections.push(connection);
        Ok(())
    }

    pub fn flip_random_connection(
        &mut self,
        rng: &mut tinyrand::Wyrand,
    ) -> Result<(), NeuralError> {
        if self.connections.is_empty() {
            return Err(NeuralError::NoConnections);
        }
        let index = rng.next_range(0..self.connections.len());
        self.connections[index].enabled = !self.connections[index].enabled;
        Ok(())
    }

    pub(crate) fn mutate_perturb_random_connection_weight(
        &mut self,
        mutation_strength: f32,
        perturb_disabled_connections: bool,
        rng: &mut tinyrand::Wyrand,
    ) -> Result<(), NeuralError> {
        let mut index;
        let active_connections = self.get_active_connections();
        if perturb_disabled_connections || active_connections.is_empty() {
            index = NeuralNetwork::select_random_connection_index(&self.connections, rng)?;
        } else {
            index = rng.next_range(0..self.get_active_connections().len());
            index = self
                .connections
                .iter()
                .position(|c| active_connections.get(index).unwrap() == &c)
                .unwrap();
        }
        let rand_val = (rng.next_u32() as f32) / (u32::MAX as f32);
        let f = 1.0 + mutation_strength * rand_val;
        let sign = if (rng.next_u32() as f32) / (u32::MAX as f32) < 0.5 {
            1.0
        } else {
            -1.0
        };
        self.connections[index].weight *= f.powf(sign);
        Ok(())
    }

    pub(crate) fn mutate_reset_random_connection_weight(
        &mut self,
        mutation_strength: f32,
        perturb_disabled_connections: bool,
        rng: &mut tinyrand::Wyrand,
    ) -> Result<(), NeuralError> {
        let mut index;
        let active_connections = self.get_active_connections();
        if perturb_disabled_connections || active_connections.is_empty() {
            index = NeuralNetwork::select_random_connection_index(&self.connections, rng)?;
        } else {
            index = rng.next_range(0..self.get_active_connections().len());
            index = self
                .connections
                .iter()
                .position(|c| active_connections.get(index).unwrap() == &c)
                .unwrap();
        }
        self.connections[index].weight = ((rng.next_u32() as f32) / (u32::MAX as f32))
            * (mutation_strength * 2.0)
            - mutation_strength;
        Ok(())
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
        let len_cost = active_connections.len() as f32 * 0.15;
        let weight_cost = active_connections
            .iter()
            .map(|c| c.weight.abs())
            .sum::<f32>()
            * 0.1;
        let think_cost = len_cost + weight_cost;
        think_cost + 0.01
    }

    pub fn run(&self, inputs: Vec<SensorInput>) -> Result<Vec<f32>, NeuralError> {
        let mut node_values = vec![0.0; self.nodes.len()];

        // Set initial values for input nodes based on SensorInput
        for input in inputs {
            let index = input.index;
            if index < self.nodes.len() && matches!(self.nodes[index].node_type, NodeType::Input) {
                node_values[index] = input.value;
            }
        }

        // Propagate values through the network
        for connection in &self.connections {
            if connection.enabled {
                let input_value = node_values[connection.in_node];
                node_values[connection.out_node] += input_value * connection.weight;
            }
        }

        // Apply activation functions to all nodes (skipping input nodes)
        for (i, node) in self.nodes.iter().enumerate() {
            if matches!(node.node_type, NodeType::Hidden | NodeType::Output) {
                node_values[i] = node.activation.apply(node_values[i])?;
            }
        }

        // Extract the output values and return them
        let outputs = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                if matches!(node.node_type, NodeType::Output) {
                    Some(node_values[i])
                } else {
                    None
                }
            })
            .collect();
        Ok(outputs)
    }

    fn select_random_connection_index(
        connections: &[ConnectionGene],
        rng: &mut impl Rand,
    ) -> Result<usize, NeuralError> {
        if connections.is_empty() {
            Err(NeuralError::NoConnections)
        } else {
            Ok(rng.next_range(0..connections.len()))
        }
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
        network.add_connection(0, 2, 0.5, true, 0).unwrap();
        network.add_connection(1, 2, 0.5, true, 1).unwrap();
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
        let outputs = network.run(inputs).unwrap();
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
        let outputs = network.run(inputs).unwrap();
        assert!((outputs[0] - 0.5).abs() < 1e-6);
    }
}
