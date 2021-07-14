
use std::borrow::Cow;

use encodings::{delta, prefix_varint};

use super::Encoded;

pub fn decompression_iter<'a, 'b>(Compressed(bytes): &'a Compressed<'b>) -> impl Iterator<Item=Encoded> + 'a {
    prefix_varint::u64_decompressor(bytes)
        .map(delta::u64_decoder())
        .map(|v| Encoded(v as u32))
}

#[derive(Default)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Compressed<'c>(Cow<'c, [u8]>);

impl<'c> Compressed<'c> {

    pub fn from_raw(bytes: &'c [u8]) -> Self {
        Self(bytes.into())
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn num_bytes(&self) -> usize {
        self.0.len()
    }

    #[allow(dead_code)]
    pub fn cap(&self) -> usize {
        self.0.len()
    }
}


pub struct Compressor<F: FnMut(u64) -> u64> {
    compressor: prefix_varint::U64Compressor<F>,
    buffer: Option<Encoded>,
    num_compressed: u64,
}

// TODO add capacity
pub fn compressor() -> Compressor<impl FnMut(u64) -> u64> {
    Compressor {
        compressor: prefix_varint::U64Compressor::with(delta::u64_encoder()),
        buffer: None,
        num_compressed: 0,
    }
}

impl<F: FnMut(u64) -> u64> Compressor<F> {
    pub fn is_empty(&self) -> bool {
        self.buffer.is_none() && self.compressor.is_empty()
    }

    pub fn last_mut(&mut self) -> Option<&mut Encoded> {
        self.buffer.as_mut()
    }

    pub fn push(&mut self, value: Encoded) {
        if let Some(val) = self.buffer.take() {
            self.compress_value(val)
        }

        self.buffer = Some(value)
    }

    pub fn into_compressed(mut self) -> (Compressed<'static>, u64) {
        if let Some(val) = self.buffer.take() {
            self.compress_value(val)
        }

        (Compressed(self.compressor.finish().into()), self.num_compressed)
    }

    fn compress_value(&mut self, Encoded(value): Encoded) {
        self.num_compressed += 1;
        self.compressor.push(value.into());
    }
}

impl<F: FnMut(u64) -> u64> Extend<Encoded> for Compressor<F> {
    fn extend<T: IntoIterator<Item = Encoded>>(&mut self, iter: T) {
        for e in iter {
            self.push(e)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[quickcheck]
    fn quick_test_roundtrip(values: Vec<u32>) -> bool {
        let mut compressor = compressor();
        for val in &values {
            compressor.push(Encoded(*val));
        }
        let (blocks, count) = compressor.into_compressed();

        let decompressed = decompression_iter(&blocks);

        let expected_len = values.len();
        let mut actual_len = 0;
        for (i, (a, b)) in values.iter().zip(decompressed).enumerate() {
            if *a != b.0 {
                println!("value mismatch @ {}, expected {}, got {}", i, a, b.0,);
                return false;
            }
            actual_len += 1
        }

        if expected_len != actual_len {
            println!(
                "iter len mismatch, expected {}, got {}",
                expected_len, actual_len,
            );
            return false;
        }
        if expected_len as u64 != count {
            println!(
                "compression count mismatch, expected {}, got {}",
                expected_len, count,
            );
            return false;
        }
        true
    }
}
