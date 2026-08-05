//! Ogg's CRC-32.
//!
//! Ogg uses a CRC-32 with polynomial `0x04C11DB7`, initial value 0, **no**
//! input/output reflection, and no final XOR — which is different from the
//! common zlib/PNG CRC-32 (that one reflects and uses poly `0xEDB88320`). Every
//! page stores this checksum computed over the entire page *with the checksum
//! field itself zeroed*.
//!
//! Reference: RFC 3533 §6.

/// Precomputed CRC table (256 entries), built at first use.
///
/// Using a `OnceLock` keeps the table zero-cost until the first page is
/// checksummed and avoids a large const array in the source.
use std::sync::OnceLock;

static CRC_TABLE: OnceLock<[u32; 256]> = OnceLock::new();

fn table() -> &'static [u32; 256] {
    CRC_TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        let mut i = 0usize;
        while i < 256 {
            // Non-reflected: process the byte in the high 8 bits.
            let mut crc = (i as u32) << 24;
            let mut bit = 0;
            while bit < 8 {
                crc = if crc & 0x8000_0000 != 0 {
                    (crc << 1) ^ 0x04C1_1DB7
                } else {
                    crc << 1
                };
                bit += 1;
            }
            table[i] = crc;
            i += 1;
        }
        table
    })
}

/// Incremental Ogg CRC-32.
///
/// Feed the CRC in pieces — useful for a page, where the stored CRC field must
/// be treated as zero without copying or mutating the input: feed the bytes
/// before the field, then four zero bytes, then the rest.
#[derive(Debug, Clone)]
pub struct Hasher {
    crc: u32,
}

impl Hasher {
    /// Start a new hasher (initial value 0, per RFC 3533).
    pub fn new() -> Self {
        Hasher { crc: 0 }
    }

    /// Feed more bytes.
    pub fn update(&mut self, data: &[u8]) {
        let table = table();
        let mut crc = self.crc;
        for &byte in data {
            let idx = ((crc >> 24) as u8 ^ byte) as usize;
            crc = (crc << 8) ^ table[idx];
        }
        self.crc = crc;
    }

    /// Finish and return the checksum.
    pub fn finish(&self) -> u32 {
        self.crc
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One-shot checksum, for tests only.
    fn checksum(data: &[u8]) -> u32 {
        let mut h = Hasher::new();
        h.update(data);
        h.finish()
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(checksum(&[]), 0);
    }

    #[test]
    fn known_vector() {
        // CRC of the ASCII string "123456789" under Ogg's exact parameters
        // (poly 0x04C11DB7, init 0, no reflection, no final xor) is 0x89A1897F.
        // Verified against a reference implementation of RFC 3533's CRC.
        assert_eq!(checksum(b"123456789"), 0x89A1_897F);
    }

    #[test]
    fn order_matters() {
        assert_ne!(checksum(b"AB"), checksum(b"BA"));
    }
}
