use bevy_ecs::prelude::Component;
use rand::prelude::SliceRandom;
use rand::Rng;

#[derive(Clone, Debug)]
pub struct Segment {
    pub energy_cost_move: f32,
    pub energy_cost_always: f32,
}
#[derive(Clone, Debug, Component)]
pub enum SegmentType {
    Muscle(Segment),
    Solid(Segment),
    Split(Segment)
}

fn all_segment_types() -> [SegmentType; 3] {
    [SegmentType::muscle(), SegmentType::solid(), SegmentType::split()]
}

impl SegmentType {
    pub fn muscle() -> Self {
        SegmentType::Muscle(Segment {
            energy_cost_move: 1.0,
            energy_cost_always: 0.0,
        })
    }

    pub fn solid() -> Self {
        SegmentType::Solid(Segment {
            energy_cost_move: 1.0,
            energy_cost_always: 0.0,
        })
    }
    pub fn split() -> Self {
        SegmentType::Split(Segment {
            energy_cost_move: 1.0,
            energy_cost_always: 0.0,
        })
    }
}

pub enum MutationType {
    AddGene,
    RemoveGene,
    ChangeSegmentType,
    ChangeJump
}

#[derive(Clone)]
pub struct Gene {
    pub segment_type: SegmentType,
    pub id: usize,
    pub jump: usize,
}

#[derive(Clone)]
pub struct Dna {
    pub genes: Vec<Gene>,
    pub current_gene: usize,
}

impl Dna {
    pub(crate) fn random(gene_pool_size: usize) -> Dna {
        let mut rng = rand::thread_rng();
        let mut genes = Vec::new();
        for i in 0..gene_pool_size {
            let segment_types = all_segment_types();
            let random_segment_type = segment_types.choose(&mut rng).unwrap().clone();
            let random_jump = rng.gen_range(0..gene_pool_size);
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
    pub fn mutate(&mut self) {
        let rng = &mut rand::thread_rng();
        let mutations = [MutationType::AddGene, MutationType::RemoveGene, MutationType::ChangeSegmentType, MutationType::ChangeJump];
        let random_mutation = mutations.choose(rng).unwrap();
        let segment_types = all_segment_types();
        match random_mutation {
            MutationType::AddGene => {}
            MutationType::RemoveGene => {}
            MutationType::ChangeSegmentType => {
                let random_segment_type = segment_types.choose(rng).unwrap().clone();
                let random_index = rng.gen_range(0..self.genes.len());
                self.genes[random_index].segment_type = random_segment_type;
            }
            MutationType::ChangeJump => {
                let random_jump = rng.gen_range(0..self.genes.len());
                let random_index = rng.gen_range(0..self.genes.len());
                self.genes[random_index].jump = random_jump;
            }
        }
    }

    pub fn get_current_gene(&self) -> &Gene {
        &self.genes[self.current_gene]
    }

    pub fn build_segment(&mut self) -> SegmentType {
        let segment = self.genes[self.current_gene].segment_type.clone();
        self.current_gene = self.genes[self.current_gene].jump;
        segment
    }
}