use std::{
    cmp::{
        min,
        Ordering::{Equal, Greater, Less},
    },
    collections::HashSet,
};

use crate::{dense, Extractable};

use self::varint::*;

mod varint;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Storage<'s> {
    to_merge: HashSet<Encoded>,
    pub compressed: Compressed<'s>,
    pub num_compressed: u64,
    pub precision: u8,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Encoded(u32);

const NUM_HIGH_BITS: u8 = 25;

pub type Overflowing = bool;

impl<'s> Storage<'s> {
    pub fn new(precision: u8) -> Self {
        // TODO what is max precision
        assert!(
            precision >= 4 && precision <= 18,
            "invalid value for precision: {}; must be within [4, 18]",
            precision,
        );
        Self {
            to_merge: Default::default(),
            compressed: Default::default(),
            num_compressed: 0,
            precision,
        }
    }

    pub fn from_parts(bytes: &'s[u8], num_compressed: u64, precision: u8) -> Self {
        // TODO what is max precision
        assert!(
            precision >= 4 && precision <= 18,
            "invalid value for precision: {}; must be within [4, 18]",
            precision,
        );
        Self {
            to_merge: Default::default(),
            compressed: Compressed::from_raw(bytes),
            num_compressed,
            precision,
        }
    }

    pub fn add_hash(&mut self, hash: u64) -> Overflowing {
        let encoded = Encoded::from_hash(hash, self.precision);
        self.add_encoded(encoded)
    }

    fn add_encoded(&mut self, encoded: Encoded) -> Overflowing {
        self.to_merge.insert(encoded);
        let max_sparse_bitsize = (1u64 << self.precision) * 6;
        // TODO what threshold?
        if self.to_merge.len() as u64 * 32 > max_sparse_bitsize / 4 {
            self.merge_buffers();
            return self.compressed.num_bytes() as u64 * 8 > max_sparse_bitsize;
        }
        false
    }

    pub fn estimate_count(&mut self) -> u64 {
        self.merge_buffers();
        self.immutable_estimate_count()
    }

    pub fn immutable_estimate_count(&self) -> u64 {
        if !self.to_merge.is_empty() {
            panic!("tried to estimate count with unmerged state")
        }
        let m_p = 1 << NUM_HIGH_BITS;
        let v = (m_p - self.num_compressed) as f64;
        let m_p = m_p as f64;
        (m_p * (m_p / v).ln()) as u64
    }

    pub fn merge_buffers(&mut self) {
        if self.to_merge.is_empty() {
            return;
        }
        let mut temp: Vec<_> = self.to_merge.drain().collect();
        temp.sort_unstable();
        temp.dedup_by_key(|e| e.idx());

        // TODO set original cap to self.compressed.cap()
        let mut new_compressed = compressor();
        let mut a = decompression_iter(&self.compressed).peekable();
        let mut b = temp.into_iter().fuse().peekable();

        let mut merge_in = |to_merge_in| {
            if new_compressed.is_empty() {
                new_compressed.push(to_merge_in);
                return;
            }

            let prev = new_compressed.last_mut().unwrap();
            if prev.idx() != to_merge_in.idx() {
                new_compressed.push(to_merge_in);
                return;
            }

            if prev.count(NUM_HIGH_BITS) < to_merge_in.count(NUM_HIGH_BITS) {
                *prev = to_merge_in;
            }
        };
        while let (Some(val_a), Some(val_b)) = (a.peek(), b.peek()) {
            let (idx_a, idx_b) = (val_a.idx(), val_b.idx());
            let to_merge_in = match idx_a.cmp(&idx_b) {
                Less => a.next().unwrap(),
                Greater => b.next().unwrap(),
                Equal => {
                    let (a, b) = (a.next().unwrap(), b.next().unwrap());
                    min(a, b)
                }
            };
            merge_in(to_merge_in);
        }
        a.for_each(&mut merge_in);
        b.for_each(merge_in);

        let (compressed, count) = new_compressed.into_compressed();
        self.compressed = compressed;
        self.num_compressed = count;
    }

    fn iter(&self) -> impl Iterator<Item = Encoded> + '_ {
        decompression_iter(&self.compressed)
    }

    pub fn to_dense(&mut self) -> dense::Storage<'static> {
        self.merge_buffers();

        self.immutable_to_dense()
    }

    pub fn immutable_to_dense(&self) -> dense::Storage<'static> {
        if !self.to_merge.is_empty() {
            panic!("tried to generate dense storage with unmerged state")
        }
        let mut dense = dense::Storage::new(self.precision);
        for encoded in self.iter() {
            dense.add_encoded(encoded)
        }
        dense
    }

    pub fn num_bytes(&self) -> usize {
        self.compressed.num_bytes()
    }

    pub fn merge_in<'o>(&mut self, other: &Storage<'o>) -> Overflowing {
        assert!(
            self.precision == other.precision,
            "precision must be equal (left={}, right={})",
            self.precision,
            other.precision
        );

        assert!(other.to_merge.is_empty());

        let mut overflowing = false;
        for encoded in other.iter() {
            overflowing = self.add_encoded(encoded)
        }
        overflowing
    }
}

impl Encoded {
    pub(crate) fn from_hash(hash: u64, precision: u8) -> Self {
        // Encoded form
        //
        //    | idx | count | tag |
        //    |  25 |   6*  |  1  |
        //
        // *`count` is only present when `tag` is `1`
        let idx = hash.extract(63, NUM_HIGH_BITS) as u32;
        let diff = hash.extract_bits(63 - precision, 64 - NUM_HIGH_BITS);
        if diff == 0 {
            // TODO is this right?
            let count = hash.extract_bits(63 - NUM_HIGH_BITS, 0).q() as u32 - NUM_HIGH_BITS as u32;
            Encoded((idx << 7) | (count << 1) | 1)
        } else {
            Encoded(idx << 1)
        }
    }

    pub fn idx(&self) -> u32 {
        if self.stores_count() {
            self.0 >> 7
        } else {
            self.0 >> 1
        }
    }

    pub fn count(&self, p: u8) -> u8 {
        if self.stores_count() {
            let extra_bits = NUM_HIGH_BITS - p;
            self.extract_count() + extra_bits
        } else {
            let new_hash = (self.idx() as u64) << (64 - NUM_HIGH_BITS);
            let hash_bits = new_hash.extract_bits(63 - p, 0);
            hash_bits.q() - p
        }
    }

    #[inline]
    fn stores_count(&self) -> bool {
        self.0 & 1 == 1
    }

    #[inline]
    fn extract_count(&self) -> u8 {
        self.0.extract_bits(6, 1) as u8
    }
}

impl PartialOrd for Encoded {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// The canonical ordering is by ascending index, then descending count.
// This allows us to deduplicate by index after sorting.
impl Ord for Encoded {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let idx_cmp = self.idx().cmp(&other.idx());
        if let Equal = idx_cmp {
            return match (self.stores_count(), other.stores_count()) {
                (false, false) => Equal,
                (true, false) => Less,
                (false, true) => Greater,
                (true, true) => self.extract_count().cmp(&other.extract_count()).reverse(),
            };
        }
        return idx_cmp;
    }
}

#[cfg(test)]
mod tests {
    use fnv::FnvHasher;
    use quickcheck::TestResult;

    use super::*;

    use std::hash::{Hash, Hasher};

    const NUM_HASH_BITS: u8 = 64 - NUM_HIGH_BITS;

    pub fn hash(val: i32) -> u64 {
        let mut hasher = FnvHasher::default();
        val.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_asc_10k() {
        let mut hll = Storage::new(16);
        for i in 0..10_000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 10_001)
    }

    #[test]
    fn test_asc_100k() {
        let mut hll = Storage::new(16);
        for i in 0..100_000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 100_149);
        assert_eq!(hll.compressed.num_bytes(), 184_315);
    }

    #[test]
    fn test_asc_500k() {
        let mut hll = Storage::new(16);
        for i in 0..500_000 {
            hll.add_hash(hash(i));
        }

        assert_eq!(hll.estimate_count(), 471_229);
        assert_eq!(hll.compressed.num_bytes(), 690_301);
    }

    #[quickcheck]
    fn quick_sparse(values: Vec<u64>) -> TestResult {
        if values.len() >= (1 << NUM_HASH_BITS) {
            return TestResult::discard();
        }
        let mut hll = Storage::new(16);
        let expected = values.iter().collect::<HashSet<_>>().len() as f64;
        for value in values {
            hll.add_hash(value);
        }
        let estimated = hll.estimate_count() as f64;
        let error = 0.001 * expected;
        if expected - error <= estimated && estimated <= expected + error {
            return TestResult::passed();
        }
        if estimated <= expected + 10.0 && estimated >= expected - 10.0 {
            return TestResult::passed();
        }
        println!("got {}, expected {} +- {}", estimated, expected, error);
        TestResult::failed()
    }

    #[quickcheck]
    fn quick_sparse_as_set(values: Vec<u64>) -> TestResult {
        if values.len() >= (1 << NUM_HASH_BITS) {
            return TestResult::discard();
        }
        let mut hll = Storage::new(16);
        for value in &values {
            hll.add_hash(*value);
        }
        hll.merge_buffers();
        let mut expected: Vec<_> = values
            .into_iter()
            .map(|h| Encoded::from_hash(h, 16))
            .collect();
        expected.sort_unstable();
        // println!("pre_sort {:?}", temp);
        expected.dedup_by_key(|e| e.idx());

        let expected_len = expected.len();
        let mut actual_len = 0;
        for (i, (a, b)) in expected.iter().zip(hll.iter()).enumerate() {
            if *a != b {
                println!("value mismatch @ {}, expected {}, got {}", i, a.0, b.0,);
                return TestResult::failed();
            }
            actual_len += 1
        }
        if expected_len != actual_len {
            println!(
                "iter len mismatch, expected {}, got {}",
                expected_len, actual_len,
            );
            return TestResult::failed();
        }
        return TestResult::passed();
    }

    #[quickcheck]
    fn quick_sparse_merge_invariant(values: Vec<u64>) -> TestResult {
        if values.len() >= (1 << NUM_HASH_BITS) {
            return TestResult::discard();
        }
        let mut hlla = Storage::new(16);
        let mut hllb = Storage::new(16);
        for value in &values {
            hlla.add_hash(*value);
            hllb.add_hash(*value);
            hllb.merge_buffers()
        }
        hlla.merge_buffers();

        for (i, (a, b)) in hlla.iter().zip(hllb.iter()).enumerate() {
            if a != b {
                println!("value mismatch @ {}, expected {}, got {}", i, a.0, b.0,);
                return TestResult::failed();
            }
        }

        let expected_len = hlla.iter().count();
        let actual_len = hllb.iter().count();
        if expected_len != actual_len {
            println!(
                "iter len mismatch, expected {}, got {}",
                expected_len, actual_len,
            );
            return TestResult::failed();
        }
        return TestResult::passed();
    }

    // fn encoded_order() {

    // }

    #[test]
    fn sparse_merge_01() {
        let mut hlla = Storage::new(16);
        let mut hllb = Storage::new(16);
        let values = [0, 1];
        for value in &values {
            hlla.add_hash(*value);
        }
        hlla.merge_buffers();

        for value in &values {
            hllb.add_hash(*value);
            hllb.merge_buffers()
        }
        let a: Vec<_> = hlla.iter().collect();
        let b: Vec<_> = hllb.iter().collect();
        assert_eq!(a, b)
    }
}
