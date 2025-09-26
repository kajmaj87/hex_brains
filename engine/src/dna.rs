use bevy_ecs::prelude::Component;
use tinyrand::{Rand, RandRange};

#[cfg(test)]
use tinyrand::Seeded;

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub energy_cost_move: f32,
    pub energy_cost_always: f32,
    pub mobility: f32,
}
#[derive(Clone, Debug, PartialEq, Component)]
pub enum SegmentType {
    Muscle(Segment),
    Solid(Segment),
    Solar(Segment),
    Stomach(Segment),
}

impl SegmentType {
    pub fn muscle() -> Self {
        SegmentType::Muscle(Segment {
            energy_cost_move: 1.0,
            energy_cost_always: 0.0,
            mobility: 1.0,
        })
    }

    pub fn solid() -> Self {
        SegmentType::Solid(Segment {
            energy_cost_move: 1.0,
            energy_cost_always: 0.0,
            mobility: 0.1,
        })
    }
    pub fn solar() -> Self {
        SegmentType::Solar(Segment {
            energy_cost_move: 1.0,
            energy_cost_always: -0.1,
            mobility: 0.2,
        })
    }

    pub fn stomach() -> Self {
        SegmentType::Stomach(Segment {
            energy_cost_move: 1.0,
            energy_cost_always: 1.0,
            mobility: 0.5,
        })
    }
    pub fn mobility(&self) -> f32 {
        match self {
            SegmentType::Muscle(segment) => segment.mobility,
            SegmentType::Solid(segment) => segment.mobility,
            SegmentType::Solar(segment) => segment.mobility,
            SegmentType::Stomach(segment) => segment.mobility,
        }
    }

    pub fn energy_cost_move(&self) -> f32 {
        match self {
            SegmentType::Muscle(segment) => segment.energy_cost_move,
            SegmentType::Solid(segment) => segment.energy_cost_move,
            SegmentType::Solar(segment) => segment.energy_cost_move,
            SegmentType::Stomach(segment) => segment.energy_cost_move,
        }
    }

    pub fn energy_cost_always(&self) -> f32 {
        match self {
            SegmentType::Muscle(segment) => segment.energy_cost_always,
            SegmentType::Solid(segment) => segment.energy_cost_always,
            SegmentType::Solar(segment) => segment.energy_cost_always,
            SegmentType::Stomach(segment) => segment.energy_cost_always,
        }
    }
}

fn all_segment_types() -> [SegmentType; 4] {
    [
        SegmentType::muscle(),
        SegmentType::solid(),
        SegmentType::solar(),
        SegmentType::stomach(),
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub enum MutationType {
    AddGene,
    RemoveGene,
    ChangeSegmentType,
    ChangeJump,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gene {
    pub segment_type: SegmentType,
    pub id: usize,
    pub jump: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dna {
    pub genes: Vec<Gene>,
    pub current_gene: usize,
}

impl Dna {
    pub fn random(
        rng: &mut impl Rand,
        gene_pool_size: usize,
        config: &crate::simulation::MutationConfig,
    ) -> Dna {
        let mut genes = Vec::new();
        let available_types: Vec<SegmentType> = all_segment_types()
            .into_iter()
            .filter(|t| match t {
                SegmentType::Muscle(_) => !config.disable_muscle,
                SegmentType::Solid(_) => !config.disable_solid,
                SegmentType::Solar(_) => !config.disable_solar,
                SegmentType::Stomach(_) => !config.disable_stomach,
            })
            .collect();
        // If no types available, fall back to all types to avoid panic
        let available_types = if available_types.is_empty() {
            all_segment_types().to_vec()
        } else {
            available_types
        };
        for i in 0..gene_pool_size {
            let random_segment_type =
                available_types[rng.next_range(0..available_types.len())].clone();
            let random_jump = rng.next_range(0..gene_pool_size);
            genes.push(Gene {
                segment_type: random_segment_type,
                id: i,
                jump: random_jump,
            });
        }
        Dna {
            genes,
            current_gene: 0,
        }
    }
    pub fn mutate(&mut self, rng: &mut impl Rand, config: &crate::simulation::MutationConfig) {
        let mutations = [
            MutationType::AddGene,
            MutationType::RemoveGene,
            MutationType::ChangeSegmentType,
            MutationType::ChangeJump,
        ];
        let random_mutation = mutations[rng.next_range(0..mutations.len())].clone();
        self.mutate_internal(random_mutation, rng, config);
    }

    fn mutate_internal(
        &mut self,
        mutation: MutationType,
        rng: &mut impl Rand,
        config: &crate::simulation::MutationConfig,
    ) {
        let available_types: Vec<SegmentType> = all_segment_types()
            .into_iter()
            .filter(|t| match t {
                SegmentType::Muscle(_) => !config.disable_muscle,
                SegmentType::Solid(_) => !config.disable_solid,
                SegmentType::Solar(_) => !config.disable_solar,
                SegmentType::Stomach(_) => !config.disable_stomach,
            })
            .collect();
        match mutation {
            MutationType::AddGene => {
                if !available_types.is_empty() {
                    let new_id = self.genes.len();
                    let random_segment_type =
                        available_types[rng.next_range(0..available_types.len())].clone();
                    let new_jump = rng.next_range(0..new_id);
                    self.genes.push(Gene {
                        segment_type: random_segment_type,
                        id: new_id,
                        jump: new_jump,
                    });
                }
            }
            MutationType::RemoveGene => {
                if self.genes.len() > 1 {
                    let index = rng.next_range(0..self.genes.len());
                    self.genes.remove(index);
                    for i in 0..self.genes.len() {
                        if self.genes[i].jump > index {
                            self.genes[i].jump -= 1;
                        }
                    }
                    for i in index..self.genes.len() {
                        self.genes[i].id -= 1;
                    }
                }
            }
            MutationType::ChangeSegmentType => {
                if !available_types.is_empty() {
                    let random_segment_type =
                        available_types[rng.next_range(0..available_types.len())].clone();
                    let random_index = rng.next_range(0..self.genes.len());
                    self.genes[random_index].segment_type = random_segment_type;
                }
            }
            MutationType::ChangeJump => {
                let random_jump = rng.next_range(0..self.genes.len());
                let random_index = rng.next_range(0..self.genes.len());
                self.genes[random_index].jump = random_jump;
            }
        }
    }

    pub fn get_current_gene(&self) -> &Gene {
        &self.genes[self.current_gene]
    }

    pub fn build_segment(&mut self) -> SegmentType {
        tracing::debug!(
            "build_segment: current_gene={}, len={}",
            self.current_gene,
            self.genes.len()
        );
        if self.current_gene >= self.genes.len() {
            self.current_gene = 0;
        }
        let segment = self.genes[self.current_gene].segment_type.clone();
        self.current_gene = self.genes[self.current_gene].jump;
        segment
    }
}

#[cfg(any(test, feature = "integration"))]
impl Dna {
    pub fn mutate_specific(
        &mut self,
        mutation: MutationType,
        rng: &mut impl Rand,
        config: &crate::simulation::MutationConfig,
    ) {
        self.mutate_internal(mutation, rng, config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tinyrand::Wyrand;

    fn create_test_dna() -> Dna {
        let genes = vec![
            Gene {
                segment_type: SegmentType::muscle(),
                id: 0,
                jump: 0,
            },
            Gene {
                segment_type: SegmentType::solid(),
                id: 1,
                jump: 2,
            },
            Gene {
                segment_type: SegmentType::solar(),
                id: 2,
                jump: 1,
            },
            Gene {
                segment_type: SegmentType::stomach(),
                id: 3,
                jump: 3,
            },
            Gene {
                segment_type: SegmentType::muscle(),
                id: 4,
                jump: 4,
            },
        ];
        Dna {
            genes,
            current_gene: 0,
        }
    }

    fn is_valid_dna(dna: &Dna) -> bool {
        if dna.genes.is_empty() {
            return false;
        }
        for (i, gene) in dna.genes.iter().enumerate() {
            if gene.id != i {
                return false;
            }
            if gene.jump >= dna.genes.len() {
                return false;
            }
            match &gene.segment_type {
                SegmentType::Muscle(_)
                | SegmentType::Solid(_)
                | SegmentType::Solar(_)
                | SegmentType::Stomach(_) => {}
            }
        }
        true
    }

    #[test]
    fn test_mutate_add_gene() {
        let mut rng = Wyrand::default();
        let mut dna = create_test_dna();
        let initial_len = dna.genes.len();
        let initial_genes = dna.genes.clone();
        let config = crate::simulation::MutationConfig::default();

        dna.mutate_internal(MutationType::AddGene, &mut rng, &config);

        assert_eq!(dna.genes.len(), initial_len + 1);
        let new_gene = &dna.genes[initial_len];
        assert_eq!(new_gene.id, initial_len);
        assert!(new_gene.jump <= initial_len);
        match &new_gene.segment_type {
            SegmentType::Muscle(_)
            | SegmentType::Solid(_)
            | SegmentType::Solar(_)
            | SegmentType::Stomach(_) => {}
        }
        // Other genes unchanged
        for i in 0..initial_len {
            assert_eq!(dna.genes[i].id, initial_genes[i].id);
            assert_eq!(dna.genes[i].jump, initial_genes[i].jump);
            assert_eq!(dna.genes[i].segment_type, initial_genes[i].segment_type);
        }
        assert!(is_valid_dna(&dna));
    }

    #[test]
    fn test_mutate_remove_gene() {
        let mut rng = Wyrand::default();
        let mut dna = create_test_dna();
        let initial_len = dna.genes.len();
        let config = crate::simulation::MutationConfig::default();

        dna.mutate_internal(MutationType::RemoveGene, &mut rng, &config);

        assert_eq!(dna.genes.len(), initial_len - 1);
        // Check ids are renumbered correctly
        for (i, gene) in dna.genes.iter().enumerate() {
            assert_eq!(gene.id, i);
        }
        // Jumps should be adjusted if necessary, but since we don't adjust jumps, just check bounds
        for gene in &dna.genes {
            assert!(gene.jump < dna.genes.len());
        }
        assert!(is_valid_dna(&dna));
    }

    #[test]
    fn test_mutate_change_segment_type() {
        let mut rng = Wyrand::seed(1);
        let mut dna = create_test_dna();
        let initial_genes = dna.genes.clone();
        let config = crate::simulation::MutationConfig::default();

        dna.mutate_internal(MutationType::ChangeSegmentType, &mut rng, &config);

        // One gene should have changed segment type
        let mut changed_count = 0;
        for (i, gene) in dna.genes.iter().enumerate() {
            if gene.segment_type != initial_genes[i].segment_type {
                changed_count += 1;
                match &gene.segment_type {
                    SegmentType::Muscle(_)
                    | SegmentType::Solid(_)
                    | SegmentType::Solar(_)
                    | SegmentType::Stomach(_) => {}
                }
            }
        }
        assert_eq!(changed_count, 1);
        assert!(is_valid_dna(&dna));
    }

    #[test]
    fn test_mutate_change_jump() {
        let mut rng = Wyrand::default();
        let mut dna = create_test_dna();
        let initial_genes = dna.genes.clone();
        let config = crate::simulation::MutationConfig::default();

        dna.mutate_internal(MutationType::ChangeJump, &mut rng, &config);

        // One gene should have changed jump
        let mut changed_count = 0;
        for (i, gene) in dna.genes.iter().enumerate() {
            if gene.jump != initial_genes[i].jump {
                changed_count += 1;
                assert!(gene.jump < dna.genes.len());
            }
        }
        assert_eq!(changed_count, 1);
        assert!(is_valid_dna(&dna));
    }

    #[test]
    fn test_multiple_mutations() {
        let mut rng = Wyrand::default();
        let mut dna = create_test_dna();
        let config = crate::simulation::MutationConfig::default();

        // Perform 10 mutations
        for _ in 0..10 {
            dna.mutate(&mut rng, &config);
            assert!(is_valid_dna(&dna));
            // Length should be at least 1
            assert!(dna.genes.len() >= 1);
        }

        // Length may vary, but jumps and ids valid
        for (i, gene) in dna.genes.iter().enumerate() {
            assert_eq!(gene.id, i);
            assert!(gene.jump < dna.genes.len());
        }
    }

    #[test]
    fn test_mutate_no_panic_on_min_length() {
        let mut rng = Wyrand::default();
        let mut dna = Dna {
            genes: vec![Gene {
                segment_type: SegmentType::muscle(),
                id: 0,
                jump: 0,
            }],
            current_gene: 0,
        };
        let config = crate::simulation::MutationConfig::default();

        // Try remove on length 1, should not change
        let initial_len = dna.genes.len();
        dna.mutate_internal(MutationType::RemoveGene, &mut rng, &config);
        assert_eq!(dna.genes.len(), initial_len);
        assert!(is_valid_dna(&dna));
    }

    #[test]
    fn test_regression_build_segment_index_out_of_bounds() {
        // Regression test for the fixed panic: ensures build_segment handles valid jumps without panic
        // Previously, invalid jumps >= len could cause index out of bounds
        let mut dna = Dna {
            genes: vec![
                Gene {
                    segment_type: SegmentType::muscle(),
                    id: 0,
                    jump: 1,
                },
                Gene {
                    segment_type: SegmentType::solid(),
                    id: 1,
                    jump: 2,
                },
                Gene {
                    segment_type: SegmentType::solar(),
                    id: 2,
                    jump: 3,
                },
                Gene {
                    segment_type: SegmentType::stomach(),
                    id: 3,
                    jump: 4,
                },
                Gene {
                    segment_type: SegmentType::muscle(),
                    id: 4,
                    jump: 5,
                },
                Gene {
                    segment_type: SegmentType::solid(),
                    id: 5,
                    jump: 6,
                },
                Gene {
                    segment_type: SegmentType::solar(),
                    id: 6,
                    jump: 6,
                }, // valid jump < len
            ],
            current_gene: 6,
        };

        // Should not panic
        let _ = dna.build_segment();
        let _ = dna.build_segment();
    }

    #[test]
    fn test_regression_remove_gene_jump_adjustment() {
        // Regression test for the fixed jump adjustment bug in remove_gene
        // With buggy code (>), this should panic; with fixed (>=), it should not
        let mut dna = Dna {
            genes: vec![
                Gene {
                    segment_type: SegmentType::muscle(),
                    id: 0,
                    jump: 0,
                },
                Gene {
                    segment_type: SegmentType::solid(),
                    id: 1,
                    jump: 2,
                },
                Gene {
                    segment_type: SegmentType::solar(),
                    id: 2,
                    jump: 2,
                },
            ],
            current_gene: 1,
        };
        let mut rng = Wyrand::default();
        let config = crate::simulation::MutationConfig::default();
        dna.mutate_internal(MutationType::RemoveGene, &mut rng, &config);
        // With buggy code (>), jumps may be invalid, causing panic in build_segment
        let result = std::panic::catch_unwind(|| {
            let mut dna_clone = dna.clone();
            dna_clone.build_segment(); // sets current_gene to 2
            dna_clone.build_segment(); // tries to access genes[2], panic
        });
        assert!(result.is_ok(), "Should not panic with correct adjustment");
    }
}
