//! # ogg-core
//!
//! A small, dependency-free reader for the Ogg container format (RFC 3533),
//! written in pure Rust. It is the engine behind `@kryxjs/ogg`.
//!
//! ## What it does
//!
//! - Parses Ogg pages (capture pattern, version, header flags, granule
//!   position, serial number, sequence number, CRC).
//! - Validates each page's CRC-32.
//! - Reassembles logical packets from the segment table, including packets that
//!   span multiple pages.
//! - Separates multiple interleaved logical streams by serial number.
//! - Writes Ogg: [`OggWriter`] turns packets (+ granule positions) back into
//!   valid pages with correct CRCs, splitting large packets across pages and
//!   marking BOS/EOS automatically.
//!
//! ## What it deliberately does NOT do
//!
//! It knows nothing about codecs. It hands out **raw packet bytes** plus the
//! serial and (uninterpreted) granule position. What a packet *means* — an
//! `OpusHead`, a Vorbis identification header, an audio frame — is the codec's
//! job, one layer up. This keeps the container generic: the same reader serves
//! Opus, Vorbis, FLAC, Theora, without knowing any of them.
//!
//! ## Design
//!
//! Streaming-first: [`PageReader`] is a plain `Iterator` over pages, with no
//! async runtime. Higher layers (the napi bridge, the TS SDK) wrap it in a JS
//! async iterator. Eager helpers like [`read_packets`] are built on the same
//! engine — one parser, not two.
//!
//! M1 reads from a complete `&[u8]`. An incremental byte source will be added
//! later, feeding the same reassembly logic.

mod crc;
mod error;
mod page;
mod reader;
mod writer;

pub use error::{OggError, OggResult};
pub use page::{OggPage, FLAG_BOS, FLAG_CONTINUATION, FLAG_EOS};
pub use reader::{read_packets, Packet, PageReader};
pub use writer::OggWriter;

/// The crate version, from Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_present() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn end_to_end_smoke() {
        // A tiny hand-built one-page stream should round-trip through the
        // public API to a single packet.
        use crc::Hasher;
        let mut p = Vec::new();
        p.extend_from_slice(b"OggS");
        p.push(0);
        p.push(FLAG_BOS);
        p.extend_from_slice(&0u64.to_le_bytes());
        p.extend_from_slice(&42u32.to_le_bytes());
        p.extend_from_slice(&0u32.to_le_bytes());
        p.extend_from_slice(&0u32.to_le_bytes());
        p.push(1);
        p.push(4);
        p.extend_from_slice(b"data");
        let mut h = Hasher::new();
        h.update(&p);
        let c = h.finish();
        p[22..26].copy_from_slice(&c.to_le_bytes());

        let packets = read_packets(&p).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].serial, 42);
        assert_eq!(packets[0].data, b"data");
    }
}
