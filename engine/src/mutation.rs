use crate::dna::Dna;
use crate::neural::NeuralNetwork;
use crate::simulation::MutationConfig;
use tinyrand::Wyrand;

pub fn apply_connection_flip(neural: &mut NeuralNetwork, rng: &mut Wyrand) {
    neural.flip_random_connection(rng);
}

pub fn apply_weight_perturbation(
    neural: &mut NeuralNetwork,
    rng: &mut Wyrand,
    range: f32,
    perturb_disabled: bool,
) {
    neural.mutate_perturb_random_connection_weight(range, perturb_disabled, rng);
}

pub fn apply_weight_reset(
    neural: &mut NeuralNetwork,
    rng: &mut Wyrand,
    range: f32,
    perturb_disabled: bool,
) {
    neural.mutate_reset_random_connection_weight(range, perturb_disabled, rng);
}

pub fn apply_dna_mutation(dna: &mut Dna, rng: &mut Wyrand, mutation_config: &MutationConfig) {
    dna.mutate(rng, mutation_config);
}
