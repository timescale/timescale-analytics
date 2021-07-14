
pub mod delta {
    use crate::zigzag;

    pub fn i64_decoder() -> impl FnMut(i64) -> i64 {
        let mut prev = 0i64;
        move |delta| {
            let value = prev.wrapping_add(delta);
            prev = value;
            value
        }
    }

    pub fn u64_decoder() -> impl FnMut(u64) -> u64 {
        let mut prev = 0u64;
        move |delta| {
            let delta = zigzag::decode(delta) as u64;
            let value = prev.wrapping_add(delta);
            prev = value;
            value
        }
    }

    pub fn i64_encoder() -> impl FnMut(i64) -> i64 {
        let mut prev = 0i64;
        move |value: i64| {
            let delta = value.wrapping_sub(prev);
            prev = value;
            delta
        }
    }

    pub fn u64_encoder() -> impl FnMut(u64) -> u64 {
        let mut prev = 0u64;
        move |value: u64| {
            let delta = value.wrapping_sub(prev);
            prev = value;
            zigzag::encode(delta as i64)
        }
    }

    #[cfg(test)]
    mod test {
        use quickcheck_macros::quickcheck;

        use super::*;

        #[quickcheck]
        fn quick_test_roundtrip_u64(values: Vec<u64>) -> bool {
            let mut bytes = vec![];
            crate::prefix_varint::compress_u64s_to_vec(&mut bytes, values.iter().cloned().map(u64_encoder()));

            let output: Vec<u64> = crate::prefix_varint::u64_decompressor(&bytes).map(u64_decoder()).collect();
            assert_eq!(values, output);
            true
        }

        #[quickcheck]
        fn quick_test_roundtrip_i64(values: Vec<i64>) -> bool {
            let mut bytes = vec![];
            crate::prefix_varint::compress_i64s_to_vec(&mut bytes, values.iter().cloned().map(i64_encoder()));

            let output: Vec<i64> = crate::prefix_varint::i64_decompressor(&bytes).map(i64_decoder()).collect();
            assert_eq!(values, output);
            true
        }
    }
}

pub mod zigzag {
    #[inline(always)]
    pub fn encode(n: i64) -> u64 {
        if n < 0 {
            // let's avoid the edge case of i64::min_value()
            // !n is equal to `-n - 1`, so this is:
            // !n * 2 + 1 = 2(-n - 1) + 1 = -2n - 2 + 1 = -2n - 1
            !(n as u64) * 2 + 1
        } else {
            (n as u64) * 2
        }
    }

    #[inline(always)]
    pub fn decode(n: u64) -> i64 {
        if n % 2 == 0 {
            // positive number
            (n / 2) as i64
        } else {
            // negative number
            // !m * 2 + 1 = n
            // !m * 2 = n - 1
            // !m = (n - 1) / 2
            // m = !((n - 1) / 2)
            // since we have n is odd, we have floor(n / 2) = floor((n - 1) / 2)
            !(n / 2) as i64
        }
    }
}

pub mod prefix_varint {
    /// Similar to [LEB128](https://en.wikipedia.org/wiki/LEB128), but it moves
    /// all the tag bits to the LSBs of the first byte, which ends up looking
    /// like this (`x` is a value bit, the rest are tag bits):
    /// ```python,ignore,no_run
    /// xxxxxxx1  7 bits in 1 byte
    /// xxxxxx10 14 bits in 2 bytes
    /// xxxxx100 21 bits in 3 bytes
    /// xxxx1000 28 bits in 4 bytes
    /// xxx10000 35 bits in 5 bytes
    /// xx100000 42 bits in 6 bytes
    /// x1000000 49 bits in 7 bytes
    /// 10000000 56 bits in 8 bytes
    /// 00000000 64 bits in 9 bytes
    /// ```
    /// based on https://github.com/stoklund/varint

    pub fn size_vec<I: Iterator<Item=u64>>(bytes: &mut Vec<u8>, values: I) {
        let size: usize = values.map(|v| bytes_for_value(v) as usize).sum();
        bytes.reserve(size + 9);
    }

    #[inline]
    pub fn bytes_for_value(value: u64) -> u32 {
        let bits = value.leading_zeros();
        let mut bytes = 1 + bits.wrapping_sub(1) / 7;
        if bits > 56 {
            bytes = 9
        }
        bytes
    }
    pub struct I64Compressor<F: FnMut(i64) -> i64> {
        compressor: U64Compressor<fn(u64) -> u64>,
        encoder: F,
    }

    impl I64Compressor<fn(i64) -> i64> {
        pub fn new() -> Self {
            Self {
                compressor: U64Compressor::new(),
                encoder: |i| i,
            }
        }
    }

    impl<F: FnMut(i64) -> i64> I64Compressor<F> {
        pub fn with(encoder: F) -> Self {
            Self {
                compressor: U64Compressor::new(),
                encoder,
            }
        }

        pub fn push(&mut self, value: i64) {
            let encoded = crate::zigzag::encode((self.encoder)(value));
            self.compressor.push(encoded)
        }

        pub fn finish(self) -> Vec<u8> {
            self.compressor.finish()
        }
    }

    pub struct U64Compressor<F: FnMut(u64) -> u64> {
        bytes: Vec<u8>,
        encoder: F,
    }

    impl U64Compressor<fn(u64) -> u64> {
        pub fn new() -> Self {
            Self {
                bytes: vec![],
                encoder: |i| i,
            }
        }
    }

    impl<F: FnMut(u64) -> u64> U64Compressor<F> {
        pub fn with(encoder: F) -> Self {
            Self {
                bytes: vec![],
                encoder,
            }
        }

        pub fn push(&mut self, value: u64) {
            let encoded = (self.encoder)(value);
            write_to_vec(&mut self.bytes, encoded);
        }

        pub fn finish(self) -> Vec<u8> {
            self.bytes
        }

        pub fn is_empty(&self) -> bool {
            self.bytes.is_empty()
        }
    }

    pub fn compress_i64s_to_vec<I: Iterator<Item=i64>>(bytes: &mut Vec<u8>, values: I) {
        compress_u64s_to_vec(bytes, values.map(crate::zigzag::encode))
    }

    pub fn compress_u64s_to_vec<I: Iterator<Item=u64>>(bytes: &mut Vec<u8>, values: I) {
        values.for_each(|v| write_to_vec(bytes, v));
    }

    // based on https://github.com/stoklund/varint, (Apache licensed)
    // see also https://github.com/WebAssembly/design/issues/601
    #[inline]
    pub fn write_to_vec(out: &mut Vec<u8>, mut value: u64) {
        if value == 0 {
            out.push(0x1);
            return
        }
        let bits = 64 - value.leading_zeros();
        let mut bytes = 1 + bits.wrapping_sub(1) / 7;
        if bits > 56 {
            out.push(0);
            bytes = 8
        } else if value != 0 {
            value = (2 * value + 1) << (bytes - 1)
        }
        let value = value.to_le_bytes();
        for i in 0..(bytes as usize) {
            out.push(value[i]);
        }
    }

    type Value = u64;

    pub fn i64_decompressor(bytes: &[u8])
    -> impl Iterator<Item=i64> + '_ {
        u64_decompressor(bytes).map(crate::zigzag::decode)
    }

    pub fn u64_decompressor(mut bytes: &[u8])
    -> impl Iterator<Item=u64> + '_ {
        std::iter::from_fn(move || {
            if bytes.is_empty() {
                return None
            }

            let (value, len) = read_from_slice(bytes);
            bytes = &bytes[len..];
            return Some(value)
        })
    }

    #[inline]
    pub fn read_from_slice(bytes: &[u8]) -> (Value, usize) {
        use std::convert::TryInto;

        let value: [u8; 8] = if bytes.len() >= 8 {
            bytes[0..8].try_into().unwrap()
        } else {
            let mut value = [0; 8];
            value[..bytes.len()].copy_from_slice(bytes);
            value
        };
        let tag_byte = value[0];
        if tag_byte & 1 == 1 {
            let value = (tag_byte >> 1) as u64;
            return (value, 1)
        }
        let length = prefix_length(tag_byte) as usize;
        let value = if length < 9 {
            let unused = 64 - 8 * length;
            let value = u64::from_le_bytes(value.try_into().unwrap());
            (value << unused) >> (unused + length)
        } else {
            u64::from_le_bytes(bytes[1..9].try_into().unwrap())
        };

        return (value, length)
    }

    #[inline(always)]
    pub fn prefix_length(tag_byte: u8) -> u32 {
        1 + ((tag_byte as u32) | 0x100).trailing_zeros()
    }

    #[cfg(test)]
    mod test {
        use quickcheck_macros::quickcheck;

        use super::*;

        #[quickcheck]
        fn quick_test_roundtrip_u64(values: Vec<u64>) -> bool {
            let mut bytes = vec![];
            compress_u64s_to_vec(&mut bytes, values.iter().cloned());

            let output: Vec<u64> = u64_decompressor(&bytes).collect();
            assert_eq!(values, output);
            true
        }

        #[quickcheck]
        fn quick_test_roundtrip_i64(values: Vec<i64>) -> bool {
            let mut bytes = vec![];
            compress_i64s_to_vec(&mut bytes, values.iter().cloned());

            let output: Vec<i64> = i64_decompressor(&bytes).collect();
            assert_eq!(values, output);
            true
        }
    }
}
