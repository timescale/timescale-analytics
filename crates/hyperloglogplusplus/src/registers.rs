use std::{borrow::Cow, convert::TryInto, debug_assert};

/// array of 6bit registers, of power-of-2 size
// 24 is the LCM of 6 and 8, so we can divide our registers into
// blocks of 24 bits and only deal with whole registers as follows:
//
//    b b b b b b|b b
//    b b b b|b b b b
//    b b|b b b b b b
//
// (3 bytes makes 4 whole registers)
// We can turn this into a 32bit block like so
//
//    b b b b b b|b b
//    b b b b|b b b b
//    b b|b b b b b b
//    0 0 0 0 0 0 0 0
//
// and treat the block like a regular integer, using shifts to get the
// values in and out
#[derive(Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Registers<'s>(Cow<'s, [u8]>);

impl<'s> Registers<'s> {
    /// allocate a new Registers of size `2^exponent`
    pub fn new(exponent: u8) -> Self {
        assert!(exponent >= 4 && exponent <= 64);
        let num_registers: i128 = 1 << exponent;
        let num_bits = num_registers * 6;
        // store an additional byte at the end so we can always use 16-bit reads
        // exhaustive search of the [4, 64] parameter space shows that this
        // formula works correctly
        let num_bytes = (num_bits / 8) + 1;
        let mut bytes = vec![0u8; num_bytes as usize];

        // set the extra byte to 0xff so we don't count it as 0
        bytes.last_mut().map(|l| *l = 0xff);

        Self(bytes.into())
    }

    pub fn from_raw(bytes: &'s [u8]) -> Self {
        Self(bytes.into())
    }

    #[cfg(test)]
    pub fn at(&self, idx: usize) -> u8 {
        // TODO switch chunks_exact_mut() to as_chunks_mut() once stable?
        let block_num = idx / 4;
        let idx_in_block = idx % 4;
        let block = self.0.chunks_exact(3).skip(block_num).next().unwrap();
        let block = u32::from_be_bytes([block[0], block[1], block[2], 0x0]);
        let value = block >> (8 + 6 * (3 - idx_in_block));
        (value & 0x3f) as u8
    }

    pub fn set_max(&mut self, idx: usize, value: u8) {
        debug_assert!(value < (1 << 6));

        let block_num = idx / 4;
        let idx_in_block = idx % 4;
        // TODO switch chunks_exact_mut() to as_chunks_mut() once stable?
        let (a, b, c) = match self.0.to_mut().chunks_exact_mut(3).skip(block_num).next() {
            Some([a, b, c, ..]) => (a, b, c),
            _ => panic!(
                "index {} out of bounds of {} registers",
                idx,
                (self.0.len() - 1) / 3 * 4,
            ),
        };

        let block = u32::from_be_bytes([*a, *b, *c, 0x0]);

        let shift = 8 // extra 0 byte at the end
            + 6 * (3 - idx_in_block); // idx 0 is at the largest offset, so it needs the greatest shift
        let mask = 0x3f << shift;
        let value = (value as u32) << shift;

        let old_value = block & mask;
        if old_value < value {
            let block = (block & !mask) | value;

            let [new_a, new_b, new_c, _] = u32::to_be_bytes(block);

            *a = new_a;
            *b = new_b;
            *c = new_c;
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &*self.0
    }

    pub fn count_zeroed_registers(&self) -> u64 {
        self.iter().filter(|&b| b == 0).count() as u64
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        use std::iter::once;

        // our length should be divisible by 3, plus an extra byte we add
        debug_assert_eq!(self.0.len() % 3, 1);

        self.0.chunks_exact(3).flat_map(|bytes| {
            const LOW_REG_MASK: u32 = (1 << 6) - 1;
            let [a, b, c]: [u8; 3] = (&*bytes).try_into().unwrap();
            let block = u32::from_be_bytes([a, b, c, 0x0]);
            // TODO replace with
            // ```
            // std::array::IntoIter::new([
            //     ((block >> 26) & LOW_REG_MASK) as u8,
            //     ((block >> 20) & LOW_REG_MASK) as u8,
            //     ((block >> 14) & LOW_REG_MASK) as u8,
            //     ((block >> 8) & LOW_REG_MASK) as u8,
            // ])
            // ```
            // once std::array::IntoIter becomes stable
            once(((block >> 26) & LOW_REG_MASK) as u8)
                .chain(once(((block >> 20) & LOW_REG_MASK) as u8))
                .chain(once(((block >> 14) & LOW_REG_MASK) as u8))
                .chain(once(((block >> 8) & LOW_REG_MASK) as u8))
        })
    }

    pub fn byte_len(&self) -> usize {
        self.0.len()
    }

    pub fn merge<'a, 'b>(a: &Registers<'a>, b: &Registers<'b>) -> Self {
        if a.0.len() != b.0.len() {
            panic!("different register size in merge: {} != {}", a.0.len(), b.0.len())
        }

        let registers: Vec<u8> = (&*a.0).into();
        let mut merged = Registers(registers.into());
        for (i, v) in b.iter().enumerate() {
            merged.set_max(i, v);
        }

        merged
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_last_index_not_clobbered() {
        for i in 4..14 {
            let mut regs = Registers::new(i);

            let read = regs.at((i - 1) as _);
            assert!(read == 0, "{}: {} = {}", i, read, 0);

            regs.set_max((i - 1) as _, 0xf);
            let read = regs.at((i - 1) as _);
            assert!(read == 0xf, "{}: {} = {}", i, read, 0xf);

            if i > 1 {
                let read = regs.at((i - 2) as _);
                assert!(read == 0, "{}: {} = {}", i, read, 0);

                regs.set_max((i - 2) as _, 0x3f);
                let read = regs.at((i - 2) as _);
                assert!(read == 0x3f, "{}: {} = {}", i, read, 0x3f);

                let read = regs.at((i - 1) as _);
                assert!(read == 0xf, "{}: {} = {}", i, read, 0xf);
            }
        }
    }

    #[test]
    fn test_last_index_not_clobbers() {
        for i in 4..14 {
            let mut regs = Registers::new(i);

            let read = regs.at((i - 2) as _);
            assert!(read == 0, "{}: {} = {}", i, read, 0);

            regs.set_max((i - 2) as _, 0x3c);
            let read = regs.at((i - 2) as _);
            assert!(read == 0x3c, "{}: {} = {}", i, read, 0x3c);

            let read = regs.at((i - 1) as _);
            assert!(read == 0, "{}: {} = {}", i, read, 0);

            let read = regs.at((i - 1) as _);
            assert!(read == 0, "{}: {} = {}", i, read, 0);

            regs.set_max((i - 1) as _, 0x3f);
            let read = regs.at((i - 1) as _);
            assert!(read == 0x3f, "{}: {} = {}", i, read, 0x3f);

            if i > 1 {
                let read = regs.at((i - 2) as _);
                assert!(read == 0x3c, "{}: {} = {}", i, read, 0x3c);
            }
        }
    }

    #[test]
    fn test_count_empty() {
        assert_eq!(Registers::new(4).count_zeroed_registers(), 16);
    }

    #[test]
    fn test_count_4() {
        let registers = Registers::new(4);
        assert_eq!(registers.count_zeroed_registers(), 16);
    }

    #[test]
    fn test_count_5() {
        let registers = Registers::new(5);
        assert_eq!(registers.count_zeroed_registers(), 32);
    }

    #[test]
    fn test_count_6() {
        let registers = Registers::new(6);
        assert_eq!(registers.count_zeroed_registers(), 64);
    }

    #[test]
    fn test_count_7() {
        let registers = Registers::new(7);
        assert_eq!(registers.count_zeroed_registers(), 128);
    }

    #[test]
    fn test_iter_4_0_1() {
        let mut registers = Registers::new(4);
        registers.set_max(0, 1);
        let values: Vec<_> = registers.iter().collect();
        let mut expected = [0; 16];
        expected[0] = 1;
        assert_eq!(values, expected);
    }

    #[quickcheck]
    fn quick_test(exp: u8, ops: Vec<(usize, u8)>) -> quickcheck::TestResult {
        use quickcheck::TestResult;
        use std::cmp::max;
        if exp < 4 || exp > 16 {
            return TestResult::discard();
        }

        let size = 1 << exp;
        let mut reference = vec![0; size];
        let mut registers = Registers::new(exp);
        for (idx, val) in ops {
            let fixed_idx = idx % size;
            let val = val & 0x3f;
            reference[fixed_idx] = max(val, reference[fixed_idx]);
            registers.set_max(fixed_idx, val);
        }
        let mut expected_count = 0;
        for (idx, val) in reference.iter().enumerate() {
            if registers.at(idx) != *val {
                return TestResult::failed();
            }
            if *val == 0 {
                expected_count += 1;
            }
        }

        let expeceted_len = reference.len();
        let mut actual_len = 0;
        for (i, (a, b)) in reference.iter().zip(registers.iter()).enumerate() {
            if *a != b {
                println!("value mismatch @ {}, expected {}, got {}", i, a, b,);
                return TestResult::failed();
            }
            actual_len += 1
        }
        if expeceted_len != actual_len {
            println!(
                "iter len mismatch, expected {}, got {}",
                expeceted_len, actual_len,
            );
            return TestResult::failed();
        }

        let actual_count = registers.count_zeroed_registers();
        if actual_count != expected_count {
            println!(
                "count mismatch, expected {}, got {}",
                expected_count, actual_count,
            );
            return TestResult::failed();
        }
        TestResult::passed()
    }

    #[quickcheck]
    fn quick_merge(exp: u8, ops_a: Vec<(usize, u8)>, ops_b: Vec<(usize, u8)>)
    -> quickcheck::TestResult {
        use quickcheck::TestResult;

        if exp < 4 || exp > 16 {
            return TestResult::discard();
        }

        let size = 1 << exp;
        let mut reference = Registers::new(exp);
        let mut a = Registers::new(exp);
        for (idx, val) in ops_a {
            let fixed_idx = idx % size;
            let val = val & 0x3f;
            a.set_max(fixed_idx, val);
            reference.set_max(fixed_idx, val);
        }

        let mut b = Registers::new(exp);
        for (idx, val) in ops_b {
            let fixed_idx = idx % size;
            let val = val & 0x3f;
            b.set_max(fixed_idx, val);
            reference.set_max(fixed_idx, val);
        }


        let merged = Registers::merge(&a, &b);
        assert_eq!(&*merged.0, &*reference.0);
        TestResult::passed()
    }
}
