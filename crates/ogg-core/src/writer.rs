//! Writing an Ogg bitstream.
//!
//! The inverse of `reader`: it takes packets and produces valid Ogg pages
//! (capture pattern, header, segment table, body, correct CRC). It handles a
//! single logical stream — the common case, and exactly what an Opus audio
//! stream needs. Multi-stream muxing is a later milestone.
//!
//! Design (per the M3 decision): the caller supplies packets plus a granule
//! position; the writer owns the mechanics — segmenting large packets across
//! 255-byte lacing values, packing segments into pages (max 255 per page),
//! setting BOS on the first page and EOS on the last, incrementing the sequence
//! number, and computing each page's CRC. The caller never builds a page by
//! hand.
//!
//! Granule position is a first-class parameter from day one: designing it in
//! now means `@kryxjs/codecs-opus` can write real `.opus` files later without
//! an API break.
//!
//! To keep M3 simple and always correct, the writer emits **one packet per
//! page** (a packet that fits) — valid Ogg, if not maximally packed. Packets
//! too large for one page span multiple pages via the continuation flag.
//! Multi-packet page packing can come later without an API change.
//!
//! Reference: RFC 3533 §6.

use crate::crc;
use crate::page::{FLAG_BOS, FLAG_CONTINUATION, FLAG_EOS};

/// Maximum segments per page (the segment-table length is a single byte).
const MAX_SEGMENTS_PER_PAGE: usize = 255;

/// Writes a single logical Ogg stream.
///
/// Feed packets with [`write_packet`](OggWriter::write_packet); call
/// [`finish`](OggWriter::finish) to emit the final page (marked end-of-stream)
/// and get the complete Ogg bytes.
pub struct OggWriter {
    serial: u32,
    sequence: u32,
    out: Vec<u8>,
    /// Packets written but not yet flushed to a page. Held so the last one can
    /// be marked end-of-stream at `finish`.
    pending: Vec<(Vec<u8>, u64)>,
    started: bool,
}

impl OggWriter {
    /// Create a writer for the logical stream identified by `serial`.
    pub fn new(serial: u32) -> Self {
        OggWriter {
            serial,
            sequence: 0,
            out: Vec::new(),
            pending: Vec::new(),
            started: false,
        }
    }

    /// Queue a packet, stamping `granule_position` on the page where it ends.
    ///
    /// The packet before it (if any) is flushed to a page now; the newest
    /// packet is held back so `finish` can mark the final page end-of-stream.
    pub fn write_packet(&mut self, data: &[u8], granule_position: u64) {
        // Flush all but keep the just-added packet pending until we know whether
        // it's the last (for the EOS flag).
        if let Some((prev_data, prev_granule)) = self.pending.pop() {
            self.emit_packet(&prev_data, prev_granule, false);
        }
        self.pending.push((data.to_vec(), granule_position));
    }

    /// Finish the stream: flush the final packet, mark it end-of-stream, and
    /// return the complete Ogg bytes.
    pub fn finish(mut self) -> Vec<u8> {
        if let Some((data, granule)) = self.pending.pop() {
            self.emit_packet(&data, granule, true);
        } else if !self.started {
            // No packets at all: emit a single empty BOS+EOS page so the output
            // is still a structurally valid (empty) stream.
            self.emit_page(&[], &[], true, false, true, u64::MAX);
        }
        self.out
    }

    /// Compute the lacing values (segment table) for a body of `len` bytes.
    ///
    /// Every full 255-byte run emits a 255 lacing value; the packet terminates
    /// with a value < 255 — including an explicit 0 when `len` is an exact
    /// multiple of 255, so the reader knows the packet ended here.
    fn lacing_for(len: usize) -> Vec<u8> {
        let mut lacing = Vec::new();
        let mut remaining = len;
        loop {
            if remaining >= 255 {
                lacing.push(255);
                remaining -= 255;
            } else {
                lacing.push(remaining as u8);
                break;
            }
        }
        lacing
    }

    /// Emit one packet as one or more pages, splitting when it needs more than
    /// one page of segments. `eos` marks the page on which the packet ends as
    /// end-of-stream. Granule is stamped only on that final page; earlier
    /// spanning pages carry -1 (no packet completes there).
    fn emit_packet(&mut self, data: &[u8], granule_position: u64, eos: bool) {
        let lacing = Self::lacing_for(data.len());
        let total_segs = lacing.len();

        let mut seg_offset = 0usize;
        let mut byte_offset = 0usize;

        while seg_offset < total_segs {
            let seg_end = (seg_offset + MAX_SEGMENTS_PER_PAGE).min(total_segs);
            let page_lacing = &lacing[seg_offset..seg_end];
            let body_len: usize = page_lacing.iter().map(|&s| s as usize).sum();
            let body = &data[byte_offset..byte_offset + body_len];

            let is_final_chunk = seg_end == total_segs;
            let bos = !self.started;
            let continuation = seg_offset > 0;
            let granule = if is_final_chunk {
                granule_position
            } else {
                u64::MAX
            };
            let page_eos = eos && is_final_chunk;

            self.emit_page(page_lacing, body, bos, continuation, page_eos, granule);

            seg_offset = seg_end;
            byte_offset += body_len;
        }
    }

    /// Write one complete Ogg page to the output buffer.
    #[allow(clippy::too_many_arguments)]
    fn emit_page(
        &mut self,
        segment_sizes: &[u8],
        body: &[u8],
        bos: bool,
        continuation: bool,
        eos: bool,
        granule_position: u64,
    ) {
        debug_assert!(segment_sizes.len() <= MAX_SEGMENTS_PER_PAGE);

        let mut header_type = 0u8;
        if continuation {
            header_type |= FLAG_CONTINUATION;
        }
        if bos {
            header_type |= FLAG_BOS;
        }
        if eos {
            header_type |= FLAG_EOS;
        }

        let start = self.out.len();

        self.out.extend_from_slice(b"OggS");
        self.out.push(0); // version
        self.out.push(header_type);
        self.out.extend_from_slice(&granule_position.to_le_bytes());
        self.out.extend_from_slice(&self.serial.to_le_bytes());
        self.out.extend_from_slice(&self.sequence.to_le_bytes());
        self.out.extend_from_slice(&[0, 0, 0, 0]); // CRC placeholder
        self.out.push(segment_sizes.len() as u8);
        self.out.extend_from_slice(segment_sizes);
        self.out.extend_from_slice(body);

        // Compute CRC over the full page image with the CRC field zeroed, then
        // patch it in place.
        let page = &self.out[start..];
        let mut h = crc::Hasher::new();
        h.update(page);
        let checksum = h.finish();
        self.out[start + 22..start + 26].copy_from_slice(&checksum.to_le_bytes());

        self.sequence = self.sequence.wrapping_add(1);
        self.started = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::read_packets;

    #[test]
    fn single_packet_roundtrips() {
        let mut w = OggWriter::new(42);
        w.write_packet(b"hello", 960);
        let bytes = w.finish();

        let packets = read_packets(&bytes).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].serial, 42);
        assert_eq!(packets[0].data, b"hello");
        assert_eq!(packets[0].granule_position, Some(960));
    }

    #[test]
    fn multiple_packets_roundtrip_in_order() {
        let mut w = OggWriter::new(7);
        w.write_packet(b"AAA", 100);
        w.write_packet(b"BB", 200);
        w.write_packet(b"C", 300);
        let bytes = w.finish();

        let packets = read_packets(&bytes).unwrap();
        assert_eq!(packets.len(), 3);
        assert_eq!(packets[0].data, b"AAA");
        assert_eq!(packets[1].data, b"BB");
        assert_eq!(packets[2].data, b"C");
        assert_eq!(packets[2].granule_position, Some(300));
    }

    #[test]
    fn first_page_is_bos_last_is_eos() {
        let mut w = OggWriter::new(1);
        w.write_packet(b"x", 10);
        w.write_packet(b"y", 20);
        let bytes = w.finish();

        // Parse pages directly to check flags.
        use crate::page::parse_page;
        let (p0, next) = parse_page(&bytes, 0).unwrap();
        assert!(p0.is_bos());
        assert!(!p0.is_eos());
        let (p1, _) = parse_page(&bytes, next).unwrap();
        assert!(!p1.is_bos());
        assert!(p1.is_eos());
    }

    #[test]
    fn large_packet_spans_pages() {
        // A packet needing > 255 segments must span pages. 255 segments cover
        // up to 255*255 = 65025 bytes; use more than that.
        let big = vec![0xABu8; 70_000];
        let mut w = OggWriter::new(1);
        w.write_packet(&big, 1234);
        let bytes = w.finish();

        let packets = read_packets(&bytes).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].data.len(), 70_000);
        assert_eq!(packets[0].data, big);
        assert_eq!(packets[0].granule_position, Some(1234));
    }

    #[test]
    fn exact_multiple_of_255_terminates() {
        // A 255-byte packet needs a 255 lacing then an explicit 0 terminator,
        // so the reader sees exactly one 255-byte packet (not an open one).
        let data = vec![0x55u8; 255];
        let mut w = OggWriter::new(1);
        w.write_packet(&data, 0);
        let bytes = w.finish();

        let packets = read_packets(&bytes).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].data.len(), 255);
    }

    #[test]
    fn empty_stream_is_valid() {
        let w = OggWriter::new(9);
        let bytes = w.finish();
        // Should parse as a valid (empty) stream: one BOS+EOS page, no packets.
        let packets = read_packets(&bytes).unwrap();
        assert_eq!(packets.len(), 0);
    }
}
