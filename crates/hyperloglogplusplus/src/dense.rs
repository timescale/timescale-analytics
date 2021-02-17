use crate::hyperloglog_data::{
    BIAS_DATA_OFFSET, BIAS_DATA_VEC, RAW_ESTIMATE_DATA_OFFSET, RAW_ESTIMATE_DATA_VEC,
    THRESHOLD_DATA_OFFSET, THRESHOLD_DATA_VEC,
};

use crate::{registers::Registers, Extractable};

#[derive(Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Storage<'s> {
    pub registers: Registers<'s>,
    // TODO can be derived from block.len()
    index_shift: u8,
    pub precision: u8,
    hash_mask: u64,
}

impl<'s> Storage<'s> {
    pub fn new(precision: u8) -> Self {
        // TODO what is max precision
        assert!(
            precision >= 4 && precision <= 18,
            "invalid value for precision: {}; must be within [4, 18]",
            precision,
        );
        let non_index_bits = 64 - precision;
        Self {
            registers: Registers::new(precision),
            index_shift: non_index_bits,
            precision,
            hash_mask: (1 << non_index_bits) - 1,
        }
    }

    pub fn from_parts(registers: &'s [u8], precision: u8) -> Self {
        let non_index_bits = 64 - precision;
        Self {
            registers: Registers::from_raw(registers),
            index_shift: non_index_bits,
            precision,
            hash_mask: (1 << non_index_bits) - 1,
        }
    }

    pub fn add_hash(&mut self, hash: u64) {
        let (idx, count) = self.idx_count_from_hash(hash);
        self.registers.set_max(idx as usize, count as u8);
    }

    pub fn add_encoded(&mut self, encoded: crate::sparse::Encoded) {
        let (idx, count) = self.idx_count_from_encoded(encoded);
        self.registers.set_max(idx as usize, count);
    }

    fn idx_count_from_hash(&self, hash: u64) -> (usize, u8) {
        let idx = hash.extract(63, self.precision);
        // w in the paper
        let hash_bits = hash.extract_bits(63 - self.precision, 0);
        let count = hash_bits.q() - self.precision;
        (idx as usize, count)
    }

    fn idx_count_from_encoded(&self, encoded: crate::sparse::Encoded) -> (usize, u8) {
        let old_idx = encoded.idx();
        let idx = old_idx >> (25 - self.precision);
        let count = encoded.count(self.precision);
        (idx as usize, count)
    }

    pub fn estimate_count(&self) -> u64 {
        let num_zeros = self.registers.count_zeroed_registers();
        let sum: f64 = self
            .registers
            .iter()
            .map(|v| 2.0f64.powi(-(v as i32)))
            .sum();
        let m = (1 << self.precision) as f64;
        let a_m = self.a_m();
        let e = a_m * m.powi(2) / sum;
        let e_p = if e <= 5.0 * m {
            e - self.estimate_bias(e)
        } else {
            e
        };

        let h = if num_zeros != 0 {
            self.linear_counting(num_zeros as f64)
        } else {
            e_p
        };

        if h <= self.threshold() {
            return h as u64;
        } else {
            return e as u64;
        }
    }

    fn linear_counting(&self, v: f64) -> f64 {
        let m = (1 << self.precision) as f64;
        m * (m / v).ln()
    }

    fn threshold(&self) -> f64 {
        THRESHOLD_DATA_VEC[self.precision as usize - THRESHOLD_DATA_OFFSET] as f64
    }

    fn a_m(&self) -> f64 {
        let size = 1 << self.precision;
        let m = size as f64;
        match size {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m),
        }
    }

    fn estimate_bias(&self, estimate: f64) -> f64 {
        use Bounds::*;

        let raw_estimates =
            RAW_ESTIMATE_DATA_VEC[self.precision as usize - RAW_ESTIMATE_DATA_OFFSET];
        let bias_data = BIAS_DATA_VEC[self.precision as usize - BIAS_DATA_OFFSET];

        let start = raw_estimates.binary_search_by(|v| v.partial_cmp(&estimate).unwrap());
        let mut bounds = match start {
            Ok(i) => return bias_data[i],
            Err(0) => Right(0),
            Err(i) if i == raw_estimates.len() => Left(i - 1),
            Err(i) => (Both(i - 1, i)),
        };
        let mut neighbors = [0; 6];
        let mut distances = [0.0; 6];
        for i in 0..6 {
            let (idx, distance) = bounds.next_closest(estimate, raw_estimates);
            neighbors[i] = idx;
            distances[i] = distance;
        }
        for i in 0..6 {
            distances[i] = 1.0 / distances[i];
        }
        let total: f64 = distances.iter().sum();
        for i in 0..6 {
            distances[i] /= total;
        }

        let mut value = 0.0;
        for i in 0..6 {
            value += distances[i] * bias_data[i];
        }

        return value;

        enum Bounds {
            Left(usize),
            Right(usize),
            Both(usize, usize),
        }

        impl Bounds {
            // find the closet neighbor to `estimate` in `raw_estimates` and update self
            fn next_closest(&mut self, estimate: f64, raw_estimates: &[f64]) -> (usize, f64) {
                match self {
                    Left(i) => {
                        let idx = *i;
                        *i -= 1;
                        (idx, (raw_estimates[*i] - estimate).abs())
                    }
                    Right(i) => {
                        let idx = *i;
                        *i += 1;
                        (idx, (raw_estimates[*i] - estimate).abs())
                    }
                    Both(l, r) => {
                        let left_delta = (raw_estimates[*l] - estimate).abs();
                        let right_delta = (raw_estimates[*r] - estimate).abs();
                        if right_delta < left_delta {
                            let idx = *r;
                            if *r < raw_estimates.len() - 1 {
                                *r += 1;
                                return (idx, right_delta);
                            }
                            *self = Left(*l);
                            (idx, right_delta)
                        } else {
                            let idx = *l;
                            if *r > 0 {
                                *r -= 1;
                                return (idx, left_delta);
                            }
                            *self = Right(*r);
                            (idx, left_delta)
                        }
                    }
                }
            }
        }
    }

    pub fn merge_in<'o>(&mut self, other: &Storage<'o>) {
        assert!(
            self.precision == other.precision,
            "precision must be equal (left={}, right={})",
            self.precision,
            other.precision
        );

        assert!(
            self.registers.bytes().len() == other.registers.bytes().len(),
            "registers length must be equal (left={}, right={})",
            self.registers.bytes().len(),
            other.registers.bytes().len(),
        );

        // TODO this is probably inefficient
        for (i, r) in other.registers.iter().enumerate() {
            self.registers.set_max(i, r)
        }
    }

    pub fn num_bytes(&self) -> usize {
        self.registers.byte_len()
    }
}

#[cfg(test)]
mod tests {
    use fnv::FnvHasher;

    use crate::sparse::Encoded;

    use super::*;

    use std::{
        collections::HashSet,
        hash::{Hash, Hasher},
    };

    pub fn hash<V: Hash>(val: V) -> u64 {
        let mut hasher = FnvHasher::default();
        val.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    #[should_panic(expected = "invalid value for precision: 3; must be within [4, 18]")]
    fn new_panics_b3() {
        Storage::new(3);
    }

    #[test]
    fn new_works_b4() {
        Storage::new(4);
    }

    #[test]
    fn new_works_b18() {
        Storage::new(18);
    }

    #[test]
    #[should_panic(expected = "invalid value for precision: 19; must be within [4, 18]")]
    fn new_panics_b19() {
        Storage::new(19);
    }

    #[test]
    fn empty() {
        assert_eq!(Storage::new(8).estimate_count(), 0);
    }

    #[test]
    fn add_b4_n1k() {
        let mut hll = Storage::new(4);
        for i in 0..1000 {
            hll.add_hash(hash(i));
        }
        // FIXME examine in more detail
        assert_eq!(hll.estimate_count(), 96);
    }

    #[test]
    fn add_b8_n1k() {
        let mut hll = Storage::new(8);
        for i in 0..1000 {
            hll.add_hash(hash(i));
        }
        // FIXME examine in more detail
        assert_eq!(hll.estimate_count(), 469);
    }

    #[test]
    fn add_b12_n1k() {
        let mut hll = Storage::new(12);
        for i in 0..1000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 1146);
    }

    #[test]
    fn add_b16_n1k() {
        let mut hll = Storage::new(16);
        for i in 0..1000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 1007);
    }

    #[test]
    fn add_b8_n10k() {
        let mut hll = Storage::new(8);
        for i in 0..10000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 10536);
    }

    #[test]
    fn add_b12_n10k() {
        let mut hll = Storage::new(12);
        for i in 0..10000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 11513);
    }

    #[test]
    fn add_b16_n10k() {
        let mut hll = Storage::new(16);
        for i in 0..10000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 10850);
    }

    #[test]
    fn add_b16_n100k() {
        let mut hll = Storage::new(16);
        for i in 0..100000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 126_448);
    }

    #[test]
    fn add_b16_n1m() {
        let mut hll = Storage::new(16);
        for i in 0..1000000 {
            hll.add_hash(hash(i));
        }
        assert_eq!(hll.estimate_count(), 882644);
    }

    #[test]
    fn clone() {
        let mut hll1 = Storage::new(12);
        for i in 0..500 {
            hll1.add_hash(hash(i));
        }
        let c1a = hll1.estimate_count();

        let hll2 = hll1.clone();
        assert_eq!(hll2.estimate_count(), c1a);

        for i in 501..1000 {
            hll1.add_hash(hash(i));
        }
        let c1b = hll1.estimate_count();
        assert_ne!(c1b, c1a);
        assert_eq!(hll2.estimate_count(), c1a);
    }

    #[test]
    fn merge() {
        let mut hll1 = Storage::new(12);
        let mut hll2 = Storage::new(12);
        let mut hll = Storage::new(12);
        for i in 0..500 {
            hll.add_hash(hash(i));
            hll1.add_hash(hash(i));
        }
        for i in 501..1000 {
            hll.add_hash(hash(i));
            hll2.add_hash(hash(i));
        }
        assert_ne!(hll.estimate_count(), hll1.estimate_count());
        assert_ne!(hll.estimate_count(), hll2.estimate_count());

        hll1.merge_in(&hll2);
        assert_eq!(hll.estimate_count(), hll1.estimate_count());
    }

    #[test]
    #[should_panic(expected = "precision must be equal (left=5, right=12)")]
    fn merge_panics_p() {
        let mut hll1 = Storage::new(5);
        let hll2 = Storage::new(12);
        hll1.merge_in(&hll2);
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

        let mut hll = Storage::new(4);
        for entry in &panic_data {
            hll.add_hash(hash(entry));
        }

        hll.estimate_count();
    }

    #[quickcheck]
    fn quick_16(values: HashSet<u64>) -> quickcheck::TestResult {
        let mut hll = Storage::new(16);
        let expected = values.iter().collect::<HashSet<_>>().len() as f64;
        for value in values {
            hll.add_hash(value);
        }
        let estimated = hll.estimate_count() as f64;
        let error = 0.01 * expected;
        // quickcheck instantly finds hash collisions, so we can only check that
        // we underestimate the cardinality
        if estimated <= expected + error {
            return quickcheck::TestResult::passed();
        }
        println!("got {}, expected {} +- {}", estimated, expected, error);
        quickcheck::TestResult::failed()
    }

    #[cfg(feature="flaky_tests")]
    #[quickcheck]
    fn quick_8(values: Vec<u64>) -> quickcheck::TestResult {
        let mut hll = Storage::new(8);
        let expected = values.iter().collect::<HashSet<_>>().len() as f64;
        for value in values {
            hll.add_hash(value);
        }
        let estimated = hll.estimate_count() as f64;
        let error = 0.10 * expected;
        // quickcheck instantly finds hash collisions, so we can only check that
        // we underestimate the cardinality
        if estimated <= expected + error {
            return quickcheck::TestResult::passed();
        }
        println!("got {}, expected {} +- {}", estimated, expected, error);
        quickcheck::TestResult::failed()
    }

    #[quickcheck]
    fn quick_decode_16(value: u64) -> bool {
        let hll = Storage::new(8);
        let from_hash = hll.idx_count_from_hash(value);
        let from_encoded = hll.idx_count_from_encoded(Encoded::from_hash(value, hll.precision));
        if from_hash != from_encoded {
            println!(
                "{:#x}, expected {:?}, got {:?}",
                value, from_hash, from_encoded
            );
            return false;
        }

        return true;
    }
}
