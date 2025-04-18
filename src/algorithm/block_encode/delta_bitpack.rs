// compress docid with delta encoding and bitpacking
// compress tf with bitpacking

use std::num::NonZeroU32;

use bitpacking::{BitPacker, BitPacker4x};

use super::{BlockDecodeTrait, BlockEncodeTrait};

const BLOCK_SIZE: usize = 128;

pub struct DeltaBitpackEncode {
    output: Vec<u8>,
}

impl DeltaBitpackEncode {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }
}

impl BlockEncodeTrait for DeltaBitpackEncode {
    fn encode(
        &mut self,
        offset: Option<NonZeroU32>,
        docids: &mut [u32],
        freqs: &mut [u32],
    ) -> &[u8] {
        assert!(docids.len() == freqs.len());
        assert!(docids.len() == BLOCK_SIZE);

        let bit_packer = BitPacker4x::new();

        self.output.clear();
        freqs.iter_mut().for_each(|v| *v -= 1);
        let offset = offset.map(|x| x.get());

        let docid_bits = bit_packer.num_bits_strictly_sorted(offset, docids);
        let freq_bits = bit_packer.num_bits(freqs);
        let docid_size = compress_size(docid_bits, docids.len());
        let freq_size = compress_size(freq_bits, freqs.len());
        self.output.extend_from_slice(&[docid_bits, freq_bits]);
        self.output.resize(docid_size + freq_size + 2, 0);

        let mut output = &mut self.output[2..];
        bit_packer.compress_strictly_sorted(offset, docids, output, docid_bits);
        output = &mut output[docid_size..];
        bit_packer.compress(freqs, output, freq_bits);
        &self.output
    }
}

#[derive(Debug)]
pub struct DeltaBitpackDecode {
    inner: Box<DeltaBitpackDecodeInner>,
}

impl DeltaBitpackDecode {
    pub fn new() -> Self {
        Self {
            inner: Box::new(DeltaBitpackDecodeInner::new()),
        }
    }
}

impl BlockDecodeTrait for DeltaBitpackDecode {
    fn decode(&mut self, data: &[u8], offset: Option<NonZeroU32>) {
        self.inner.decode(data, offset);
    }

    fn next(&mut self) -> bool {
        self.inner.next()
    }

    fn seek(&mut self, target: u32) -> bool {
        self.inner.seek(target)
    }

    fn docid(&self) -> u32 {
        self.inner.docid()
    }

    fn freq(&self) -> u32 {
        self.inner.freq()
    }
}

#[derive(Debug)]
pub struct DeltaBitpackDecodeInner {
    docids: [u32; BLOCK_SIZE],
    freqs: [u32; BLOCK_SIZE],
    offset: usize,
}

impl DeltaBitpackDecodeInner {
    pub fn new() -> Self {
        Self {
            docids: [0; BLOCK_SIZE],
            freqs: [0; BLOCK_SIZE],
            offset: 0,
        }
    }
}

impl BlockDecodeTrait for DeltaBitpackDecodeInner {
    fn decode(&mut self, mut data: &[u8], offset: Option<NonZeroU32>) {
        let offset = offset.map(|x| x.get());

        let bit_packer = BitPacker4x::new();

        let docid_bits = data[0];
        let freq_bits = data[1];
        data = &data[2..];
        let docid_size =
            bit_packer.decompress_strictly_sorted(offset, data, &mut self.docids, docid_bits);
        data = &data[docid_size..];
        bit_packer.decompress(data, &mut self.freqs, freq_bits);

        self.freqs.iter_mut().for_each(|v| *v += 1);
        self.offset = 0;
    }

    fn next(&mut self) -> bool {
        self.offset += 1;

        if self.offset == BLOCK_SIZE {
            return false;
        }
        true
    }

    fn seek(&mut self, target: u32) -> bool {
        self.offset += self.docids[self.offset..].partition_point(|&v| v < target);
        self.offset < BLOCK_SIZE
    }

    fn docid(&self) -> u32 {
        self.docids[self.offset]
    }

    fn freq(&self) -> u32 {
        self.freqs[self.offset]
    }
}

// fn num_bits_strictly_sorted(offset: Option<NonZeroU32>, values: &[u32]) -> u8 {
//     let mut prev = offset.map(|x| x.get()).unwrap_or(u32::MAX);
//     let mut max = 0;
//     for &v in values {
//         let delta = v.wrapping_sub(prev) - 1;
//         prev = v;
//         max = max.max(delta);
//     }
//     32 - max.leading_zeros() as u8
// }

// fn num_bits(values: &[u32]) -> u8 {
//     let max = values.iter().copied().max().unwrap_or(0);
//     32 - max.leading_zeros() as u8
// }

fn compress_size(num_bits: u8, len: usize) -> usize {
    (num_bits as usize * len).div_ceil(8)
}

// fn compress_strictly_sorted(
//     offset: Option<NonZeroU32>,
//     uncompressed: &[u32],
//     mut compressed: &mut [u8],
//     bit_width: u8,
// ) {
//     let mut prev = offset.map(|x| x.get()).unwrap_or(u32::MAX);
//     let mut mini_buffer: u32 = 0u32;
//     let mut cursor = 0; //< number of bits written in the mini_buffer.
//     for &v in uncompressed {
//         let delta = v.wrapping_sub(prev) - 1;
//         prev = v;
//         let remaining = 32 - cursor;
//         match bit_width.cmp(&remaining) {
//             Ordering::Less => {
//                 // Plenty of room remaining in our mini buffer.
//                 mini_buffer |= delta << cursor;
//                 cursor += bit_width;
//             }
//             Ordering::Equal => {
//                 mini_buffer |= delta << cursor;
//                 // We have completed our minibuffer exactly.
//                 // Let's write it to `compressed`.
//                 compressed[..4].copy_from_slice(&mini_buffer.to_le_bytes());
//                 compressed = &mut compressed[4..];
//                 mini_buffer = 0u32;
//                 cursor = 0;
//             }
//             Ordering::Greater => {
//                 mini_buffer |= delta << cursor;
//                 // We have completed our minibuffer.
//                 // Let's write it to `compressed` and set the fresh mini_buffer
//                 // with the remaining bits.
//                 compressed[..4].copy_from_slice(&mini_buffer.to_le_bytes());
//                 compressed = &mut compressed[4..];
//                 cursor = bit_width - remaining;
//                 mini_buffer = delta >> remaining;
//             }
//         }
//     }
//     let bit = cursor.div_ceil(8) as usize;
//     compressed[..bit].copy_from_slice(&mini_buffer.to_le_bytes()[..bit]);
// }

// fn compress(uncompressed: &[u32], mut compressed: &mut [u8], bit_width: u8) {
//     let mut mini_buffer: u32 = 0u32;
//     let mut cursor = 0; //< number of bits written in the mini_buffer.
//     for &v in uncompressed {
//         let remaining = 32 - cursor;
//         match bit_width.cmp(&remaining) {
//             Ordering::Less => {
//                 // Plenty of room remaining in our mini buffer.
//                 mini_buffer |= v << cursor;
//                 cursor += bit_width;
//             }
//             Ordering::Equal => {
//                 mini_buffer |= v << cursor;
//                 // We have completed our minibuffer exactly.
//                 // Let's write it to `compressed`.
//                 compressed[..4].copy_from_slice(&mini_buffer.to_le_bytes());
//                 compressed = &mut compressed[4..];
//                 mini_buffer = 0u32;
//                 cursor = 0;
//             }
//             Ordering::Greater => {
//                 mini_buffer |= v << cursor;
//                 // We have completed our minibuffer.
//                 // Let's write it to `compressed` and set the fresh mini_buffer
//                 // with the remaining bits.
//                 compressed[..4].copy_from_slice(&mini_buffer.to_le_bytes());
//                 compressed = &mut compressed[4..];
//                 cursor = bit_width - remaining;
//                 mini_buffer = v >> remaining;
//             }
//         }
//     }
//     let bit = cursor.div_ceil(8) as usize;
//     compressed[..bit].copy_from_slice(&mini_buffer.to_le_bytes()[..bit]);
// }

// fn decompress_strictly_sorted(
//     offset: Option<NonZeroU32>,
//     compressed: &[u8],
//     uncompressed: &mut [u32],
//     bit_width: u8,
// ) {
//     assert!(bit_width <= 32);
//     let mut prev = offset.map(|x| x.get()).unwrap_or(u32::MAX);

//     if bit_width == 0 {
//         for v in uncompressed.iter_mut() {
//             prev = prev.wrapping_add(1);
//             *v = prev;
//         }
//         return;
//     }

//     let mut mini_buffer: u64 = 0;
//     let mut cursor = 0; //< number of bits read in the mini_buffer.
//     let mut idx = 0;
//     for &byte in compressed {
//         mini_buffer |= (byte as u64) << cursor;
//         cursor += 8;
//         while cursor >= bit_width {
//             let delta = mini_buffer & ((1 << bit_width) - 1);
//             mini_buffer >>= bit_width;
//             cursor -= bit_width;
//             let v = prev.wrapping_add(delta as u32).wrapping_add(1);
//             prev = v;
//             uncompressed[idx] = v;
//             idx += 1;
//             if idx == uncompressed.len() {
//                 return;
//             }
//         }
//     }
// }

// fn decompress(compressed: &[u8], uncompressed: &mut [u32], bit_width: u8) {
//     assert!(bit_width <= 32);
//     let mut mini_buffer: u64 = 0;
//     let mut cursor = 0; //< number of bits read in the mini_buffer.
//     let mut idx = 0;
//     for &byte in compressed {
//         mini_buffer |= (byte as u64) << cursor;
//         cursor += 8;
//         while cursor >= bit_width {
//             let v = mini_buffer & ((1 << bit_width) - 1);
//             mini_buffer >>= bit_width;
//             cursor -= bit_width;
//             uncompressed[idx] = v as u32;
//             idx += 1;
//             if idx == uncompressed.len() {
//                 return;
//             }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next() {
        let mut encoder = DeltaBitpackEncode::new();
        let mut decoder = DeltaBitpackDecode::new();

        let mut docids = rand::seq::index::sample(&mut rand::rng(), 10000, BLOCK_SIZE)
            .into_iter()
            .map(|x| x as u32)
            .collect::<Vec<_>>();
        docids.sort_unstable();
        let freqs = (0..BLOCK_SIZE)
            .map(|_| rand::random::<u32>() % 1000 + 1)
            .collect::<Vec<_>>();
        let offset = NonZeroU32::new(0);

        println!("docids: {:?}", docids);
        println!("freqs: {:?}", freqs);

        let data = encoder.encode(offset, &mut docids.clone(), &mut freqs.clone());
        decoder.decode(data, offset);

        for i in 0..docids.len() {
            assert_eq!(docids[i], decoder.docid());
            assert_eq!(freqs[i], decoder.freq());
            if i + 1 < docids.len() {
                assert!(decoder.next());
            } else {
                assert!(!decoder.next());
            }
        }
    }

    #[test]
    fn test_seek() {
        let mut encoder = DeltaBitpackEncode::new();
        let mut decoder = DeltaBitpackDecode::new();

        let mut docids = rand::seq::index::sample(&mut rand::rng(), 10000, BLOCK_SIZE)
            .into_iter()
            .map(|x| x as u32)
            .collect::<Vec<_>>();
        docids.sort_unstable();
        let freqs = (0..BLOCK_SIZE)
            .map(|_| rand::random::<u32>() % 1000 + 1)
            .collect::<Vec<_>>();
        let offset = NonZeroU32::new(0);

        println!("docids: {:?}", docids);
        println!("freqs: {:?}", freqs);

        let data = encoder.encode(offset, &mut docids.clone(), &mut freqs.clone());
        decoder.decode(data, offset);

        for i in 0..docids.len() {
            assert_eq!(docids[i], decoder.docid());
            assert_eq!(freqs[i], decoder.freq());
            if i + 1 < docids.len() {
                assert!(decoder.seek(docids[i] + 1));
            } else {
                assert!(!decoder.seek(docids[i] + 1));
            }
        }
    }

    #[test]
    fn test_seek_long() {
        let mut encoder = DeltaBitpackEncode::new();
        let mut decoder = DeltaBitpackDecode::new();

        let mut docids = rand::seq::index::sample(&mut rand::rng(), 10000, BLOCK_SIZE)
            .into_iter()
            .map(|x| x as u32)
            .collect::<Vec<_>>();
        docids.sort_unstable();
        let freqs = (0..BLOCK_SIZE)
            .map(|_| rand::random::<u32>() % 1000 + 1)
            .collect::<Vec<_>>();
        let offset = NonZeroU32::new(0);

        println!("docids: {:?}", docids);
        println!("freqs: {:?}", freqs);

        let data = encoder.encode(offset, &mut docids.clone(), &mut freqs.clone());
        decoder.decode(data, offset);

        assert_eq!(docids[0], decoder.docid());
        assert_eq!(freqs[0], decoder.freq());

        assert!(decoder.seek(docids.last().unwrap().clone()));
        assert_eq!(docids.last().unwrap().clone(), decoder.docid());
    }

    #[test]
    fn test_zero_bit_width() {
        let mut encoder = DeltaBitpackEncode::new();
        let mut decoder = DeltaBitpackDecode::new();

        let docids = (10..).take(BLOCK_SIZE).collect::<Vec<_>>();
        let freqs = vec![1; BLOCK_SIZE];
        let offset = NonZeroU32::new(9);

        println!("docids: {:?}", docids);
        println!("freqs: {:?}", freqs);

        let data = encoder.encode(offset, &mut docids.clone(), &mut freqs.clone());
        decoder.decode(data, offset);

        for i in 0..docids.len() {
            assert_eq!(docids[i], decoder.docid());
            assert_eq!(freqs[i], decoder.freq());
            if i + 1 < docids.len() {
                assert!(decoder.seek(docids[i] + 1));
            } else {
                assert!(!decoder.seek(docids[i] + 1));
            }
        }
    }
}
