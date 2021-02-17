// based on https://github.com/crepererum/pdatastructs.rs/blob/e4f49e6462187700b9a12e8301df9a72a0c6e58c/src/hyperloglog.rs

//! HyperLogLog implementation.
use std::borrow::Cow;
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::marker::PhantomData;

use serde::{Serialize, Deserialize};

use crate::hyperloglog_data::{
    BIAS_DATA_OFFSET, BIAS_DATA_VEC, RAW_ESTIMATE_DATA_OFFSET, RAW_ESTIMATE_DATA_VEC,
    THRESHOLD_DATA_OFFSET, THRESHOLD_DATA_VEC,
};

mod hyperloglog_data;

/// A HyperLogLog is a data structure to count unique elements on a data stream.
///
/// # Examples
/// ```
/// use hyperloglog::HyperLogLogger;
///
/// // set up filter
/// let address_bits = 4;  // so we store 2^4 = 16 registers in total
/// let mut hll = HyperLogLogger::new(address_bits);
///
/// // add some data
/// hll.add(&"my super long string");
/// hll.add(&"another super long string");
/// hll.add(&"my super long string");  // again
///
/// // later
/// assert_eq!(hll.count(), 2);
/// ```
///
/// # Applications
/// - an approximative `COUNT(DISTINCT x)` in SQL
/// - count distinct elements in a data stream
///
/// # How It Works
/// The HyperLogLog consists of `2^b` 8bit counters. Each counter is initialized to 0.
///
/// During insertion, a hash `h(x)` is calculated. The first `b` bits of the hash function are used
/// to address a register, the other bits are used to create a number `p` which essentially counts
/// the number of leading 0-bits (or in other words: the leftmost 1-bit). The addressed register is
/// then updated to the maximum of its current value and `p`.
///
/// The calculation of the count is based on `1 / Sum_0^{2^b} (2^-register_i)` with a bunch of
/// factors a corrections applied (see paper or source code).
///
/// # Implementation
/// - The registers always allocate 8 bits and are not compressed.
/// - No sparse representation is used at any point.
/// - A 64 bit hash function is used (like in HyperLogLog++ paper) instead of the 32 bit hash
///   function (like in the original HyperLogLog paper).
/// - Bias correction is applied and the data is currently just taken from the HyperLogLog++ paper
///   appendix.
///
/// # See Also
/// - `std::collections::HashSet`: can be used to get the exact count but requires you to store
///   each and every element
/// - `pdatastructs::bloomfilter::BloomFilter` and `pdatastructs::cuckoofilter::CuckooFilter`: when
///   you also want to check if a single element is in the observed set
///
/// # References
/// - ["HyperLogLog: the analysis of a near-optimal cardinality estimation algorithm", Philippe
///   Flajolet, Éric Fusy, Olivier Gandouet, Frédéric Meunier, 2007](http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.142.9475)
/// - ["HyperLogLog in Practice: Algorithmic Engineering of a State of The Art Cardinality Estimation Algorithm", Stefan
///   Heule, Marc Nunkesser, Alexander Hall, 2013](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/40671.pdf)
/// - ["Appendix to HyperLogLog in Practice: Algorithmic Engineering of a State of the Art
///   Cardinality Estimation Algorithm", Stefan Heule, Marc Nunkesser, Alexander Hall, 2016](https://goo.gl/iU8Ig)
/// - [Wikipedia: HyperLogLog](https://en.wikipedia.org/wiki/HyperLogLog)
#[derive(Clone, Serialize, Deserialize)]
pub struct HyperLogLogger<T: ?Sized, B = BuildHasherDefault<DefaultHasher>> {
    registers: Box<[u8]>,
    b: usize,
    buildhasher: B,
    #[serde(skip)]
    phantom: PhantomData<T>,
}

impl<T> HyperLogLogger<T>
where
    T: Hash + ?Sized,
{
    /// Creates a new, empty HyperLogLog.
    ///
    /// - `b` number of bits used for register selection, number of registers within the
    ///   HyperLogLog will be `2^b`. `b` must be in `[4, 16]`
    ///
    /// Panics when `b` is out of bounds.
    pub fn new(b: usize) -> Self {
        let bh = BuildHasherDefault::<DefaultHasher>::default();
        Self::with_hash(b, bh)
    }
}

impl<T, B> HyperLogLogger<T, B>
where
    T: Hash + ?Sized,
    B: BuildHasher + Clone,
{
    /// Same as `new` but with a specific `BuildHasher`.
    pub fn with_hash(b: usize, buildhasher: B) -> Self {
        assert!(
            (b >= 4) & (b <= 18),
            "b ({}) must be larger or equal than 4 and smaller or equal than 18",
            b
        );

        let m = (1 as usize) << b;
        let registers = vec![0; m].into_boxed_slice();
        Self {
            registers,
            b,
            buildhasher,
            phantom: PhantomData,
        }
    }

    pub fn as_hyperloglog(&self) -> HyperLogLog<'_, T, B> {
        HyperLogLog {
            registers: &self.registers,
            b: self.b,
            buildhasher: Cow::Borrowed(&self.buildhasher),
            phantom: PhantomData,
        }
    }

    /// Get number of bits used for register selection.
    pub fn b(&self) -> usize {
        self.as_hyperloglog().b
    }

    /// Get number of registers.
    pub fn m(&self) -> usize {
        self.as_hyperloglog().m()
    }

    /// Get `BuildHasher`.
    pub fn buildhasher(&self) -> &B {
        &self.buildhasher
    }

    /// Get relative error for this HyperLogLog configuration.
    pub fn relative_error(&self) -> f64 {
        self.as_hyperloglog().relative_error()
    }

    /// Adds an element to the HyperLogLog.
    pub fn add(&mut self, obj: &T) {
        let mut hasher = self.buildhasher.build_hasher();
        obj.hash(&mut hasher);
        let h: u64 = hasher.finish();

        // split h into:
        //  - w = 64 - b upper bits
        //  - j = b lower bits
        let w = h >> self.b;
        let j = h - (w << self.b); // no 1 as in the paper since register indices are 0-based

        // p = leftmost bit (1-based count)
        let p = w.leading_zeros() + 1 - (self.b as u32);

        let m_old = self.registers[j as usize];
        self.registers[j as usize] = cmp::max(m_old, p as u8);
    }

    /// Guess the number of unique elements seen by the HyperLogLog.
    pub fn count(&self) -> i64 {
        self.as_hyperloglog().count()
    }

    /// Merge w/ another HyperLogLog.
    ///
    /// This HyperLogLog will then have the same state as if all elements seen by `other` where
    /// directly added to `self`.
    ///
    /// Panics when `b` or `buildhasher` parameter of `self` and `other` do not match.
    pub fn merge_in(&mut self, other: &Self)
    where B: Eq {
        assert_eq!(
            self.b, other.b,
            "b must be equal (left={}, right={})",
            self.b, other.b
        );
        assert!(
            self.buildhasher == other.buildhasher,
            "buildhasher must be equal"
        );

        let other_registers = &other.registers[..self.registers.len()];
        for i in 0..self.registers.len() {
            self.registers[i] = cmp::max(self.registers[i], other_registers[i])
        }
    }

    /// Empties the HyperLogLog.
    pub fn clear(&mut self) {
        self.registers.iter_mut().for_each(|r| *r = 0);
    }

    /// Checks whether the HyperLogLog has never seen an element.
    pub fn is_empty(&self) -> bool {
        self.as_hyperloglog().is_empty()
    }
}

impl<T> fmt::Debug for HyperLogLogger<T>
where
    T: Hash + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HyperLogLog {{ b: {} }}", self.b)
    }
}

impl<T> Extend<T> for HyperLogLogger<T>
where
    T: Hash,
{
    fn extend<S: IntoIterator<Item = T>>(&mut self, iter: S) {
        for elem in iter {
            self.add(&elem);
        }
    }
}

impl<'a, T> Extend<&'a T> for HyperLogLogger<T>
where
    T: 'a + Hash + ?Sized,
{
    fn extend<S: IntoIterator<Item = &'a T>>(&mut self, iter: S) {
        for elem in iter {
            self.add(elem);
        }
    }
}
pub struct HyperLogLog<'buffer, T: ?Sized, B = BuildHasherDefault<DefaultHasher>>
where B: Clone {
    pub registers: &'buffer [u8],
    pub b: usize,
    pub buildhasher: Cow<'buffer, B>,
    pub phantom: PhantomData<T>,
}

impl<'buffer, T, B> Clone for HyperLogLog<'buffer, T, B>
where
    T: ?Sized,
    B: Clone, {

    fn clone(&self) -> Self {
        Self {
            registers: self.registers,
            b: self.b,
            buildhasher: self.buildhasher.clone(),
            phantom: self.phantom,
        }
    }
}

impl<'buffer, T, B> HyperLogLog<'buffer, T, B>
where
    T: ?Sized,
    B: Clone {
    /// Get number of registers.
    pub fn m(&self) -> usize {
        self.registers.len()
    }

    /// Get relative error for this HyperLogLog configuration.
    pub fn relative_error(&self) -> f64 {
        (3f64 * 2f64.ln() - 1f64).sqrt() / (self.m() as f64).sqrt()
    }


    fn am(&self) -> f64 {
        let m = self.registers.len();

        if m >= 128 {
            0.7213 / (1. + 1.079 / (m as f64))
        } else if m >= 64 {
            0.709
        } else if m >= 32 {
            0.697
        } else {
            0.673
        }
    }

    fn neighbor_search_startpoints(lookup_array: &[f64], e: f64) -> (Option<usize>, Option<usize>) {
        // binary search first nearest neighbor
        match lookup_array.binary_search_by(|v| v.partial_cmp(&e).unwrap()) {
            Ok(i) => (Some(i), Some(i)),
            Err(i) => {
                if i == 0 {
                    // no left index
                    (None, Some(0))
                } else if i == lookup_array.len() {
                    // no right index
                    (Some(i - 1), None)
                } else {
                    (Some(i - 1), Some(i))
                }
            }
        }
    }

    fn estimate_bias(&self, e: f64) -> f64 {
        let lookup_array = RAW_ESTIMATE_DATA_VEC[self.b - RAW_ESTIMATE_DATA_OFFSET];
        let (mut idx_left, mut idx_right) = Self::neighbor_search_startpoints(lookup_array, e);

        // collect k nearest neighbors
        let k = 6;
        assert!(lookup_array.len() >= k);
        let mut neighbors = vec![];
        for _ in 0..k {
            let (right_instead_left, idx) = match (idx_left, idx_right) {
                (Some(i_left), Some(i_right)) => {
                    // 2 candidates, find better one
                    let delta_left = (lookup_array[i_left] - e).abs();
                    let delta_right = (lookup_array[i_right] - e).abs();
                    if delta_right < delta_left {
                        (true, i_right)
                    } else {
                        (false, i_left)
                    }
                }
                (Some(i_left), None) => {
                    // just left one is there, use it
                    (false, i_left)
                }
                (None, Some(i_right)) => {
                    // just right one is there, use it
                    (true, i_right)
                }
                _ => panic!("neighborhood search failed, this is bug!"),
            };
            neighbors.push(idx);
            if right_instead_left {
                idx_right = if idx < lookup_array.len() - 1 {
                    Some(idx + 1)
                } else {
                    None
                };
            } else {
                idx_left = if idx > 0 { Some(idx - 1) } else { None };
            }
        }

        // calculate mean of neighbors
        let bias_data = BIAS_DATA_VEC[self.b - BIAS_DATA_OFFSET];
        neighbors.iter().map(|&i| bias_data[i]).sum::<f64>() / (k as f64)
    }

    fn linear_counting(&self, v: usize) -> f64 {
        let m = self.registers.len() as f64;

        m * (m / (v as f64)).ln()
    }

    fn threshold(&self) -> usize {
        THRESHOLD_DATA_VEC[self.b - THRESHOLD_DATA_OFFSET]
    }

    /// Guess the number of unique elements seen by the HyperLogLog.
    pub fn count(&self) -> i64 {
        let m = self.registers.len() as f64;

        let z = 1f64
            / self
                .registers
                .iter()
                .map(|&x| 2f64.powi(-(i32::from(x))))
                .sum::<f64>();

        let e = self.am() * m * m * z;

        let e_star = if e <= (5. * m) {
            e - self.estimate_bias(e)
        } else {
            e
        };

        let v = bytecount::count(&self.registers, 0);
        let h = if v != 0 {
            self.linear_counting(v)
        } else {
            e_star
        };

        if h <= (self.threshold() as f64) {
            h as i64
        } else {
            e_star as i64
        }
    }

    pub fn is_empty(&self) -> bool {
        self.registers.iter().all(|&x| x == 0)
    }
}


impl<'buffer, T, B> HyperLogLog<'buffer, T, B>
where
    T: Hash + ?Sized,
    B: BuildHasher + Clone, {

    /// Get `BuildHasher`.
    pub fn buildhasher(&self) -> &B {
        &self.buildhasher
    }

    pub fn merge(a: &Self, b: &Self) -> HyperLogLogger<T, B>
    where B: Clone + Eq {
        assert_eq!(
            a.b, b.b,
            "b must be equal (left={}, right={})",
            a.b, b.b
        );
        assert!(
            a.buildhasher == b.buildhasher,
            "buildhasher must be equal"
        );

        let a_registers = &a.registers[..];
        let b_registers = &b.registers[..a_registers.len()];

        HyperLogLogger {
            registers: a_registers
                .iter()
                .zip(b_registers.iter())
                .map(|x| cmp::max(x.0, x.1))
                .cloned()
                .collect(),
            b: a.b,
            buildhasher: match &a.buildhasher {
                Cow::Borrowed(hasher) => B::clone(hasher),
                Cow::Owned(hasher) => hasher.clone(),
            },
            phantom: PhantomData,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{HyperLogLogger, HyperLogLog};
    use crate::hyperloglog_data::{RAW_ESTIMATE_DATA_OFFSET, RAW_ESTIMATE_DATA_VEC};

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{BuildHasher, Hasher};

        /// BuildHasher that takes a seed.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct BuildHasherSeeded {
        seed: usize,
    }

    impl BuildHasherSeeded {
        /// Create new BuildHasherSeeded with given seed.
        pub fn new(seed: usize) -> Self {
            Self { seed }
        }
    }

    impl BuildHasher for BuildHasherSeeded {
        type Hasher = DefaultHasher;

        fn build_hasher(&self) -> DefaultHasher {
            let mut h = DefaultHasher::default();
            h.write_usize(self.seed);
            h
        }
    }

    #[test]
    #[should_panic(expected = "b (3) must be larger or equal than 4 and smaller or equal than 18")]
    fn new_panics_b3() {
        HyperLogLogger::<u64>::new(3);
    }

    #[test]
    fn new_works_b4() {
        HyperLogLogger::<u64>::new(4);
    }

    #[test]
    fn new_works_b18() {
        HyperLogLogger::<u64>::new(18);
    }

    #[test]
    #[should_panic(expected = "b (19) must be larger or equal than 4 and smaller or equal than 18")]
    fn new_panics_b19() {
        HyperLogLogger::<u64>::new(19);
    }

    #[test]
    fn getter() {
        let hll = HyperLogLogger::<u64>::new(8);
        assert_eq!(hll.b(), 8);
        assert_eq!(hll.m(), 1 << 8);
        hll.buildhasher();
    }

    #[test]
    fn relative_error() {
        let hll = HyperLogLogger::<u64>::new(4);
        assert!((hll.relative_error() - 0.2597).abs() < 0.001);
    }

    #[test]
    fn empty() {
        let hll = HyperLogLogger::<u64>::new(8);
        assert_eq!(hll.count(), 0);
        assert!(hll.is_empty());
    }

    #[test]
    fn add_b4_n1k() {
        let mut hll = HyperLogLogger::new(4);
        for i in 0..1000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 571);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b4_n1k_fnv() {
        let mut hll: HyperLogLogger::<_, fnv::FnvBuildHasher> = HyperLogLogger::with_hash(4, Default::default());
        for i in 0..1000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 102);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b8_n1k() {
        let mut hll = HyperLogLogger::new(8);
        for i in 0..1000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 964);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b8_n1k_fnv() {
        let mut hll: HyperLogLogger::<_, fnv::FnvBuildHasher> = HyperLogLogger::with_hash(8, Default::default());
        for i in 0..1000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 1585);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b12_n1k() {
        let mut hll = HyperLogLogger::new(12);
        for i in 0..1000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 984);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b16_n1k() {
        let mut hll = HyperLogLogger::new(16);
        for i in 0..1000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 998);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b8_n10k() {
        let mut hll = HyperLogLogger::new(8);
        for i in 0..10000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 10196);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b12_n10k() {
        let mut hll = HyperLogLogger::new(12);
        for i in 0..10000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 10050);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b16_n10k() {
        let mut hll = HyperLogLogger::new(16);
        for i in 0..10000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 10055);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b16_n100k() {
        let mut hll = HyperLogLogger::new(16);
        for i in 0..100000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 100656);
        assert!(!hll.is_empty());
    }

    #[test]
    fn add_b16_n1m() {
        let mut hll = HyperLogLogger::new(16);
        for i in 0..1000000 {
            hll.add(&i);
        }
        assert_eq!(hll.count(), 1000226);
        assert!(!hll.is_empty());
    }

    #[test]
    fn clear() {
        let mut hll = HyperLogLogger::new(8);
        for i in 0..1000 {
            hll.add(&i);
        }
        hll.clear();
        assert_eq!(hll.count(), 0);
        assert!(hll.is_empty());
    }

    #[test]
    fn clone() {
        let mut hll1 = HyperLogLogger::new(12);
        for i in 0..500 {
            hll1.add(&i);
        }
        let c1a = hll1.count();

        let hll2 = hll1.clone();
        assert_eq!(hll2.count(), c1a);

        for i in 501..1000 {
            hll1.add(&i);
        }
        let c1b = hll1.count();
        assert_ne!(c1b, c1a);
        assert_eq!(hll2.count(), c1a);
    }

    #[test]
    fn merge() {
        let mut hll1 = HyperLogLogger::new(12);
        let mut hll2 = HyperLogLogger::new(12);
        let mut hll = HyperLogLogger::new(12);
        for i in 0..500 {
            hll.add(&i);
            hll1.add(&i);
        }
        for i in 501..1000 {
            hll.add(&i);
            hll2.add(&i);
        }
        assert_ne!(hll.count(), hll1.count());
        assert_ne!(hll.count(), hll2.count());

        let hll3 = HyperLogLog::merge(&hll1.as_hyperloglog(), &hll2.as_hyperloglog());
        assert_eq!(hll.count(), hll3.count());

        hll1.merge_in(&hll2);
        assert_eq!(hll.count(), hll1.count());
    }

    #[test]
    #[should_panic(expected = "b must be equal (left=5, right=12)")]
    fn merge_panics_p() {
        let mut hll1 = HyperLogLogger::<u64>::new(5);
        let hll2 = HyperLogLogger::<u64>::new(12);
        hll1.merge_in(&hll2);
    }

    #[test]
    #[should_panic(expected = "buildhasher must be equal")]
    fn merge_panics_buildhasher() {
        let mut hll1 =
            HyperLogLogger::<u64, BuildHasherSeeded>::with_hash(12, BuildHasherSeeded::new(0));
        let hll2 = HyperLogLogger::<u64, BuildHasherSeeded>::with_hash(12, BuildHasherSeeded::new(1));
        hll1.merge_in(&hll2);
    }

    #[test]
    fn debug() {
        let hll = HyperLogLogger::<u64>::new(12);
        assert_eq!(format!("{:?}", hll), "HyperLogLog { b: 12 }");
    }

    #[test]
    fn extend() {
        let mut hll = HyperLogLogger::new(4);
        hll.extend(0..1000);
        assert_eq!(hll.count(), 571);
        assert!(!hll.is_empty());
    }

    #[test]
    fn extend_reference() {
        let mut hll: HyperLogLogger<i32> = HyperLogLogger::new(4);
        {
            let v: Vec<i32> = (0..1000).collect();
            hll.extend(&v); // Can `extend` by reference.
        }
        // `hll` is still usable after `v` is dropped:
        assert_eq!(hll.count(), 571);
        assert!(!hll.is_empty());
    }

    #[test]
    fn slice() {
        let mut hll: HyperLogLogger<[u8]> = HyperLogLogger::new(4);
        {
            let v = vec![0];
            hll.add(&v[..]);
        }
        // `hll` is still usable after `v` is dropped:
        assert_eq!(hll.count(), 1);
        assert!(!hll.is_empty());
    }

    #[test]
    fn issue_74() {
        let panic_data = vec![
            "ofr-1-1517560282779878449",
            "ofr-1-1517589543534331019",
            "ofr-1-1517590532450550786",
            "ofr-1-1517644560121333465",
            "ofr-1-1517746611185649116",
            "ofr-1-1518051376300950677",
            "ofr-1-1518484387459892414",
            "ofr-1-1518488008830355319",
            "ofr-1-1518488407814571264",
            "ofr-1-1518561818180978525",
            "ofr-1-1518678274740717330",
            "ofr-1-1519461045930165638",
            "ofr-1-1519470647696557288",
            "ofr-1-1519567114956309703",
            "ofr-1-1519653616441755584",
            "ofr-1-1519655049912256356",
            "ofr-1-1520105514088138521",
            "ofr-1-1520294225822221822",
            "ofr-1-1520319017418884884",
            "ofr-1-1520505982893295286",
            "ofr-1-1520553027150677707",
            "ofr-1-1520925550686111649",
            "ofr-1-1520927095122167663",
            "ofr-1-1521290010424640726",
            "ofr-1-1521458659554886917",
            "ofr-1-1521943577454052994",
            "ofr-1-1521971260753839540",
            "ofr-1-1522000670785668758",
            "ofr-1-1522043914876749176",
            "ofr-1-1522206531944580201",
            "ofr-1-1522234960069920034",
            "ofr-1-1522333169901504119",
            "ofr-1-1522363887846294936",
            "ofr-1-1522484446749918495",
            "ofr-1-1522600458059122179",
            "ofr-1-1522687450205783676",
            "ofr-1-1522765602785461678",
            "ofr-1-1522815395559769187",
            "ofr-1-1522839112893465736",
            "ofr-1-1523001178903151627",
            "ofr-1-1523018056414397988",
            "ofr-1-1523096555609261412",
            "ofr-1-1523103371222189143",
            "ofr-1-1523256333918667890",
            "ofr-1-1523270427746895732",
            "ofr-1-1523411745695466681",
            "ofr-1-1523630566301631536",
            "ofr-1-1523839014553388093",
            "ofr-1-1523894230803940925",
            "ofr-1-1523931915564221543",
            "ofr-1-1524104734332815100",
            "ofr-1-1524113364834715372",
            "ofr-1-1524209603273164167",
            "ofr-1-1524276802153219312",
            "ofr-1-1524554894791804305",
            "ofr-1-1524621894100584193",
        ];

        let mut hll = HyperLogLogger::new(4);
        for entry in &panic_data {
            hll.add(entry);
        }

        hll.count();
    }

    #[test]
    fn neighbor_search_startpoints() {
        let b = 4;
        let lookup_array = RAW_ESTIMATE_DATA_VEC[b - RAW_ESTIMATE_DATA_OFFSET];

        assert_eq!(
            HyperLogLog::<u32>::neighbor_search_startpoints(lookup_array, 0.),
            (None, Some(0))
        );
        assert_eq!(
            HyperLogLog::<u32>::neighbor_search_startpoints(lookup_array, 100.),
            (Some(78), None)
        );

        assert_eq!(
            HyperLogLog::<u32>::neighbor_search_startpoints(lookup_array, 20.),
            (Some(14), Some(15))
        );

        assert_eq!(
            HyperLogLog::<u32>::neighbor_search_startpoints(lookup_array, 11.),
            (Some(0), Some(0))
        );
        assert_eq!(
            HyperLogLog::<u32>::neighbor_search_startpoints(lookup_array, 77.2394),
            (Some(78), Some(78))
        );
        assert_eq!(
            HyperLogLog::<u32>::neighbor_search_startpoints(lookup_array, 13.2882),
            (Some(4), Some(4))
        );
    }
}
