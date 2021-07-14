#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

use std::{
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
};

pub mod dense;
mod hyperloglog_data;
pub mod registers;
pub mod sparse;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct HyperLogLog<'s, T: ?Sized, B> {
    storage: HyperLogLogStorage<'s>,
    pub buildhasher: B,
    _pd: PhantomData<T>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum HyperLogLogStorage<'s> {
    Sparse(sparse::Storage<'s>),
    Dense(dense::Storage<'s>),
}

impl<'s, T, B> HyperLogLog<'s, T, B> {
    pub fn new(precision: u8, buildhasher: B) -> Self {
        Self {
            storage: HyperLogLogStorage::Sparse(sparse::Storage::new(precision)),
            buildhasher,
            _pd: PhantomData,
        }
    }

    pub fn from_sparse_parts(
        bytes: &'s[u8],
        num_compressed: u64,
        precision: u8,
        buildhasher: B,
    ) -> Self {
        Self {
            storage: HyperLogLogStorage::Sparse(sparse::Storage::from_parts(
                bytes, num_compressed, precision
            )),
            buildhasher,
            _pd: PhantomData,
        }
    }

    pub fn from_dense_parts(
        bytes: &'s[u8],
        precision: u8,
        buildhasher: B,
    ) -> Self {
        Self {
            storage: HyperLogLogStorage::Dense(dense::Storage::from_parts(
                bytes, precision
            )),
            buildhasher,
            _pd: PhantomData,
        }
    }


    pub fn estimate_count(&mut self) -> u64 {
        use HyperLogLogStorage::*;

        match &mut self.storage {
            Sparse(s) => s.estimate_count(),
            Dense(s) => s.estimate_count(),
        }
    }

    pub fn immutable_estimate_count(&self) -> u64 {
        use HyperLogLogStorage::*;

        match &self.storage {
            Sparse(s) => s.immutable_estimate_count(),
            Dense(s) => s.estimate_count(),
        }
    }

    pub fn is_sparse(&self) -> bool {
        use HyperLogLogStorage::*;

        matches!(&self.storage, Sparse(..))
    }

    pub fn num_bytes(&self) -> usize {
        use HyperLogLogStorage::*;

        match &self.storage {
            Sparse(s) => s.num_bytes(),
            Dense(s) => s.num_bytes(),
        }
    }

    pub fn to_parts(&mut self) -> &HyperLogLogStorage<'s> {
        self.merge_all();
        &self.storage
    }

    fn merge_all(&mut self) {
        match &mut self.storage {
            HyperLogLogStorage::Sparse(s) => s.merge_buffers(),
            HyperLogLogStorage::Dense(_) => {},
        }
    }
}

impl<'s, T, B> HyperLogLog<'s, T, B>
where
    T: Hash + ?Sized,
    B: BuildHasher,
{
    pub fn add(&mut self, value: &T) {
        use HyperLogLogStorage::*;

        let mut hasher = self.buildhasher.build_hasher();
        value.hash(&mut hasher);
        let hash = hasher.finish();
        match &mut self.storage {
            Sparse(s) => {
                let overflowing = s.add_hash(hash);
                if overflowing {
                    let dense = s.to_dense();
                    self.storage = Dense(dense);
                }
            }
            Dense(s) => s.add_hash(hash),
        }
    }

    pub fn merge_in<'o>(&mut self, other: &HyperLogLog<'o, T, B>) {
        use HyperLogLogStorage::*;
        match (&mut self.storage, &other.storage) {
            (Sparse(s), Sparse(o)) => {
                let overflowing = s.merge_in(o);
                if overflowing {
                    let dense = s.to_dense();
                    self.storage = Dense(dense);
                }
            },
            (Sparse(s), Dense(o)) => {
                let mut dense = s.to_dense();
                dense.merge_in(o);
                self.storage = Dense(dense);
            },
            (Dense(s), Sparse(o)) => {
                s.merge_in(&o.immutable_to_dense())
            },
            (Dense(s), Dense(o)) => {
                s.merge_in(o)
            },
        }
    }
}

pub(crate) trait Extractable:
    Sized + Copy + std::ops::Shl<u8, Output = Self> + std::ops::Shr<u8, Output = Self>
{
    const NUM_BITS: u8;
    fn extract_bits(&self, high: u8, low: u8) -> Self {
        self.extract(high, high - low + 1)
    }
    fn extract(&self, high: u8, len: u8) -> Self {
        (*self << (Self::NUM_BITS - 1 - high)) >> (Self::NUM_BITS - len)
    }
    fn q(&self) -> u8;
}

impl Extractable for u64 {
    const NUM_BITS: u8 = 64;
    fn q(&self) -> u8 {
        self.leading_zeros() as u8 + 1
    }
}

impl Extractable for u32 {
    const NUM_BITS: u8 = 32;
    fn q(&self) -> u8 {
        self.leading_zeros() as u8 + 1
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use fnv::FnvBuildHasher;
    use quickcheck::TestResult;

    use super::*;

    #[test]
    fn test_asc_4_10k() {
        let mut hll = HyperLogLog::new(4, FnvBuildHasher::default());
        for i in 0..10_000 {
            hll.add(&i);
        }
        assert_eq!(hll.estimate_count(), 11113);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 13);
        assert!(hll.num_bytes() <= (1 << 4) * 6 / 8 + 1);
    }

    #[test]
    fn test_asc_4_100k() {
        let mut hll = HyperLogLog::new(4, FnvBuildHasher::default());
        for i in 0..100_000 {
            hll.add(&i);
        }
        assert_eq!(hll.estimate_count(), 108_048);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 13);
        assert!(hll.num_bytes() <= (1 << 4) * 6 / 8 + 1);
    }

    #[test]
    fn test_asc_4_500k() {
        let mut hll = HyperLogLog::new(4, FnvBuildHasher::default());
        for i in 0..500_000 {
            hll.add(&i);
        }

        assert_eq!(hll.estimate_count(), 425_701);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 13);
        assert!(hll.num_bytes() <= (1 << 4) * 6 / 8 + 1);
    }

    #[test]
    fn test_asc_8_10k() {
        let mut hll = HyperLogLog::new(8, FnvBuildHasher::default());
        for i in 0..10_000 {
            hll.add(&i);
        }
        assert_eq!(hll.estimate_count(), 10_536);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 193);
        assert!(hll.num_bytes() <= (1 << 8) * 6 / 8 + 1);
    }

    #[test]
    fn test_asc_8_100k() {
        let mut hll = HyperLogLog::new(8, FnvBuildHasher::default());
        for i in 0..100_000 {
            hll.add(&i);
        }
        assert_eq!(hll.estimate_count(), 121_578);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 193);
        assert!(hll.num_bytes() <= (1 << 8) * 6 / 8 + 1);
    }

    #[test]
    fn test_asc_8_500k() {
        let mut hll = HyperLogLog::new(8, FnvBuildHasher::default());
        for i in 0..500_000 {
            hll.add(&i);
        }

        assert_eq!(hll.estimate_count(), 517_382);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 193);
        assert!(hll.num_bytes() <= (1 << 8) * 6 / 8 + 1);
    }

    #[test]
    fn test_asc_16_10k() {
        let mut hll = HyperLogLog::new(16, FnvBuildHasher::default());
        for i in 0..10_000 {
            hll.add(&i);
        }
        assert_eq!(hll.estimate_count(), 10_001);
        assert!(hll.is_sparse());
        assert_eq!(hll.num_bytes(), 23_181);
        assert!(hll.num_bytes() <= (1 << 16) * 6 / 8 + 1)
    }

    #[test]
    fn test_asc_16_100k() {
        let mut hll = HyperLogLog::new(16, FnvBuildHasher::default());
        for i in 0..100_000 {
            hll.add(&i);
        }
        assert_eq!(hll.estimate_count(), 126_448);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 49_153);
        assert!(hll.num_bytes() <= (1 << 16) * 6 / 8 + 1)
    }

    #[test]
    fn test_asc_16_500k() {
        let mut hll = HyperLogLog::new(16, FnvBuildHasher::default());
        for i in 0..500_000 {
            hll.add(&i);
        }

        assert_eq!(hll.estimate_count(), 510_445);
        assert!(!hll.is_sparse());
        assert_eq!(hll.num_bytes(), 49_153);
        assert!(hll.num_bytes() <= (1 << 16) * 6 / 8 + 1)
    }

    #[quickcheck]
    fn quick_hll_16(values: HashSet<u64>) -> TestResult {
        let mut hll = HyperLogLog::new(16, FnvBuildHasher::default());
        let expected = values.len() as f64;
        for value in values {
            hll.add(&value);
        }
        let estimated = hll.estimate_count() as f64;
        let error = 0.0005 * expected;
        if expected - error <= estimated && estimated <= expected + error {
            return TestResult::passed();
        }
        if expected - 10.0 <= estimated && estimated <= expected + 10.0 {
            return TestResult::passed();
        }
        println!("got {}, expected {} +- {}", estimated, expected, error);
        TestResult::failed()
    }

    #[quickcheck]
    fn quick_merge_hll_16(values_a: Vec<u64>, values_b: Vec<u64>) {
        let mut hll_a = HyperLogLog::new(16, FnvBuildHasher::default());
        let mut baseline = HyperLogLog::new(16, FnvBuildHasher::default());
        for value in values_a {
            hll_a.add(&value);
            baseline.add(&value)
        }

        let mut hll_b = HyperLogLog::new(16, FnvBuildHasher::default());
        for value in values_b {
            hll_b.add(&value);
            baseline.add(&value)
        }

        hll_a.merge_all();
        hll_b.merge_in(&hll_a);
        assert_eq!(hll_b.estimate_count(), baseline.estimate_count())
    }

    // FIXME needs hash collision check
    #[cfg(feature="flaky_tests")]
    #[quickcheck]
    fn quick_merge_hll_8(values_a: Vec<u64>, values_b: Vec<u64>) {
        let mut hll_a = HyperLogLog::new(8, FnvBuildHasher::default());
        let mut baseline = HyperLogLog::new(8, FnvBuildHasher::default());
        for value in &values_a {
            hll_a.add(value);
            baseline.add(value)
        }

        let mut hll_b = HyperLogLog::new(8, FnvBuildHasher::default());
        for value in &values_b {
            hll_b.add(value);
            baseline.add(value)
        }

        hll_a.merge_all();
        hll_b.merge_in(&hll_a);
        let estimate = hll_b.estimate_count();
        let baseline = baseline.estimate_count();
        // FIXME
        // if there's a hash collision between the elements unique to a and b
        // the counts could be off slightly, check if there is in fact such a
        // collision
        if estimate > baseline + 5
            || estimate < baseline.saturating_sub(6) {
            panic!("{} != {}", estimate, baseline)
        }
    }

    #[quickcheck]
    fn quick_merge_hll_4(values_a: Vec<u64>, values_b: Vec<u64>) {
        let mut hll_a = HyperLogLog::new(4, FnvBuildHasher::default());
        let mut baseline = HyperLogLog::new(4, FnvBuildHasher::default());
        for value in values_a {
            hll_a.add(&value);
            baseline.add(&value)
        }

        let mut hll_b = HyperLogLog::new(4, FnvBuildHasher::default());
        for value in values_b {
            hll_b.add(&value);
            baseline.add(&value)
        }

        hll_a.merge_all();
        hll_b.merge_in(&hll_a);
        assert_eq!(hll_b.estimate_count(), baseline.estimate_count())
    }
}
