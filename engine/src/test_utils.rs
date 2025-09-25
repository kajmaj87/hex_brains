/// Test utility for mocking RNG with predictable sequential values.
/// Initialize with a vec of values, returns them via type-specific methods in order.
/// Panics if more values are requested than provided.
pub struct MockRng<T> {
    values: Vec<T>,
    index: usize,
}

impl<T> MockRng<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self { values, index: 0 }
    }

    pub fn gen(&mut self) -> T
    where
        T: Clone,
    {
        if self.index >= self.values.len() {
            panic!("MockRng: no more values");
        }
        let val = self.values[self.index].clone();
        self.index += 1;
        val
    }

    pub fn mock_gen_range(&mut self, range: std::ops::Range<T>) -> T
    where
        T: Clone + PartialOrd + std::fmt::Debug,
    {
        let val = self.gen();
        if val < range.start || val >= range.end {
            panic!("MockRng value {:?} not in range {:?}..{:?}", val, range.start, range.end);
        }
        val
    }
}

#[cfg(test)]
use tinyrand::{Rand, RandRange};
#[cfg(test)]
use tinyrand_alloc::mock::Mock;






#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_rng_gen_f64() {
        let mut rng: MockRng<f64> = MockRng::new(vec![0.5, 0.25, 0.75]);
        assert_eq!(rng.gen(), 0.5);
        assert_eq!(rng.gen(), 0.25);
        assert_eq!(rng.gen(), 0.75);
    }

    #[test]
    #[should_panic(expected = "MockRng: no more values")]
    fn test_mock_rng_panic_on_exceed() {
        let mut rng: MockRng<f64> = MockRng::new(vec![0.5]);
        let _ = rng.gen();
        let _ = rng.gen(); // should panic
    }

    #[test]
    fn test_mock_rng_gen_bool() {
        let mut rng: MockRng<bool> = MockRng::new(vec![true, false, true]);
        assert_eq!(rng.gen(), true);
        assert_eq!(rng.gen(), false);
        assert_eq!(rng.gen(), true);
    }

    #[test]
    fn test_mock_rng_gen_range_usize() {
        let mut rng: MockRng<usize> = MockRng::new(vec![2]);
        let val = rng.mock_gen_range(0..3);
        assert_eq!(val, 2);
    }


    // Experiments with tinyrand_alloc::mock::Mock
    #[test]
    fn test_tinyrand_mock_fixed() {
        let mut mock = Mock::default().with_next_u128(|_| 42);
        assert_eq!(mock.next_u64(), 42);
        assert_eq!(mock.next_u128(), 42);
    }

    #[test]
    fn test_tinyrand_mock_counter() {
        use tinyrand_alloc::mock::counter;
        let mut mock = Mock::default().with_next_u128(counter(5..8));
        assert_eq!(mock.next_u64(), 5);
        assert_eq!(mock.next_u64(), 6);
        assert_eq!(mock.next_u64(), 7);
        assert_eq!(mock.next_u64(), 5); // wraps
    }

    #[test]
    fn test_tinyrand_mock_next_range() {
        let mut mock = Mock::default().with_next_lim_u128(|_, _| 17);
        assert_eq!(mock.next_range(10..100u16), 27); // 10 + 17
    }

    #[test]
    fn test_splitmix_default_deterministic() {
        let mut rng1 = tinyrand::SplitMix::default();
        let mut rng2 = tinyrand::SplitMix::default();
        // Both should produce identical sequences
        assert_eq!(rng1.next_u32(), rng2.next_u32());
        assert_eq!(rng1.next_u32(), rng2.next_u32());
        assert_eq!(rng1.next_range(0u32..10u32), rng2.next_range(0u32..10u32));
        assert_eq!(rng1.next_range(0u32..4u32), rng2.next_range(0u32..4u32)); // For decisions
    }

    #[test]
    fn test_random_brain_decisions_deterministic() {
        use crate::core::{RandomBrain};
        let brain = RandomBrain;
        let inputs = vec![1.0; 18]; // Dummy inputs
        // Multiple calls should give same sequence
        let mut rng1 = tinyrand::SplitMix::default();
        let decision1 = brain.decide(inputs.clone(), &mut rng1);
        let mut rng2 = tinyrand::SplitMix::default();
        let decision2 = brain.decide(inputs.clone(), &mut rng2);
        // Since new rng each time, same sequence
        assert_eq!(decision1, decision2);
    }

    #[test]
    fn test_direction_random_deterministic() {
        use crate::core::Direction;
        let mut rng1 = tinyrand::SplitMix::default();
        let dir1 = Direction::random(&mut rng1);
        let mut rng2 = tinyrand::SplitMix::default();
        let dir2 = Direction::random(&mut rng2);
        assert_eq!(dir1, dir2);
    }

    // Simulate rand behavior for comparison (manual implementation)
    #[test]
    fn test_simulate_rand_gen_range_vs_tinyrand() {
        // Simulate rand's gen_range (uniform in 0..n)
        // For tinyrand next_range(0..n) is uniform in 0..n
        // But since deterministic, sequences are fixed
        let mut rng = tinyrand::SplitMix::default();
        let values: Vec<u32> = (0..10).map(|_| rng.next_range(0u32..4u32)).collect();
        // All calls to next_range should give same sequence
        let mut rng2 = tinyrand::SplitMix::default();
        let values2: Vec<u32> = (0..10).map(|_| rng2.next_range(0u32..4u32)).collect();
        assert_eq!(values, values2);
        // In rand, it would be different each run, but here same
    }
}
