//! Reading an Ogg bitstream from a complete in-memory buffer.
//!
//! This is the streaming-first core: [`PageReader`] is a plain `Iterator` over
//! pages, and packet reassembly is built on top of it, per logical stream. It
//! deliberately knows nothing about codecs — it hands out raw packet bytes and
//! the granule/serial metadata; interpreting them is the codec's job.
//!
//! M1 scope: parse from a full `&[u8]`. An incremental `from_reader` source
//! will be added later (M2+) feeding the *same* reassembly logic — one parser,
//! two data sources.

use crate::error::OggResult;
use crate::page::{parse_page, OggPage};

/// An iterator over the raw pages of an Ogg buffer.
///
/// Yields `OggResult<OggPage>` so a malformed or corrupt page surfaces as an
/// error item rather than silently ending iteration. After an error the
/// iterator stops (returns `None` thereafter).
pub struct PageReader<'a> {
    data: &'a [u8],
    pos: usize,
    done: bool,
}

impl<'a> PageReader<'a> {
    /// Create a page reader over a complete Ogg buffer.
    pub fn new(data: &'a [u8]) -> Self {
        PageReader {
            data,
            pos: 0,
            done: false,
        }
    }
}

impl Iterator for PageReader<'_> {
    type Item = OggResult<OggPage>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.pos >= self.data.len() {
            return None;
        }
        match parse_page(self.data, self.pos) {
            Ok((page, next)) => {
                self.pos = next;
                Some(Ok(page))
            }
            Err(e) => {
                // Stop after the first error — the stream position is no longer
                // trustworthy once a page fails to parse.
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

/// A raw packet reassembled from one logical stream, with the granule position
/// of the page on which it completed (codec-defined meaning; not interpreted).
#[derive(Debug, Clone)]
pub struct Packet {
    /// Serial number of the logical stream this packet belongs to.
    pub serial: u32,
    /// The packet payload bytes.
    pub data: Vec<u8>,
    /// Granule position of the page on which this packet finished. `None` if it
    /// finished on a page whose granule is -1 (no position), per the spec.
    pub granule_position: Option<u64>,
}

/// Per-logical-stream reassembly state.
struct StreamState {
    /// Bytes of a packet still open from a previous page.
    partial: Vec<u8>,
    /// Whether we currently hold an open (incomplete) packet.
    open: bool,
}

impl StreamState {
    fn new() -> Self {
        StreamState {
            partial: Vec::new(),
            open: false,
        }
    }
}

/// Reassemble all packets from a complete Ogg buffer, grouped by logical
/// stream (serial number).
///
/// Unlike the minimal M6 reader, this correctly supports multiple interleaved
/// logical streams: each page is routed to its serial's reassembly state, so
/// packets from different streams never bleed into each other. Packets that
/// span pages are stitched via the continuation flag within their own stream.
///
/// Returns packets in completion order across the whole buffer.
pub fn read_packets(data: &[u8]) -> OggResult<Vec<Packet>> {
    use std::collections::HashMap;

    let mut states: HashMap<u32, StreamState> = HashMap::new();
    let mut out: Vec<Packet> = Vec::new();

    for page in PageReader::new(data) {
        let page = page?;
        let serial = page.serial;
        let granule = page.granule_position;
        // Ogg uses all-ones (-1 as i64) to mean "no packet completes here".
        let granule_opt = if granule == u64::MAX {
            None
        } else {
            Some(granule)
        };

        let state = states.entry(serial).or_insert_with(StreamState::new);

        // A page that does NOT begin with a continuation, yet we still hold an
        // open partial for this stream, means the previous partial actually
        // ended a packet. Flush it.
        if !page.is_continuation() && state.open {
            out.push(Packet {
                serial,
                data: std::mem::take(&mut state.partial),
                granule_position: None,
            });
            state.open = false;
        }

        // Walk this page's segments, appending to the stream's partial and
        // emitting a packet each time a lacing value < 255 terminates one.
        let mut offset = 0usize;
        for &seg in &page.segment_sizes {
            let len = seg as usize;
            state
                .partial
                .extend_from_slice(&page.body[offset..offset + len]);
            offset += len;
            state.open = true;

            if seg < 255 {
                out.push(Packet {
                    serial,
                    data: std::mem::take(&mut state.partial),
                    granule_position: granule_opt,
                });
                state.open = false;
            }
        }
    }

    // Any packet still open at end-of-stream: flush it (best-effort — a clean
    // stream ends on a packet boundary, but we don't drop trailing data).
    for (serial, state) in states.iter_mut() {
        if state.open && !state.partial.is_empty() {
            out.push(Packet {
                serial: *serial,
                data: std::mem::take(&mut state.partial),
                granule_position: None,
            });
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crc;
    use crate::page::{FLAG_BOS, FLAG_CONTINUATION};

    /// Build one Ogg page with an explicit segment table + body, correct CRC.
    fn page(serial: u32, seq: u32, header_type: u8, segs: &[u8], body: &[u8]) -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(b"OggS");
        p.push(0);
        p.push(header_type);
        p.extend_from_slice(&0u64.to_le_bytes()); // granule
        p.extend_from_slice(&serial.to_le_bytes());
        p.extend_from_slice(&seq.to_le_bytes());
        p.extend_from_slice(&0u32.to_le_bytes()); // crc placeholder
        p.push(segs.len() as u8);
        p.extend_from_slice(segs);
        p.extend_from_slice(body);
        let mut h = crc::Hasher::new();
        h.update(&p);
        let c = h.finish();
        p[22..26].copy_from_slice(&c.to_le_bytes());
        p
    }

    #[test]
    fn single_packet_single_page() {
        let data = page(1, 0, FLAG_BOS, &[5], b"hello");
        let packets = read_packets(&data).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].data, b"hello");
        assert_eq!(packets[0].serial, 1);
    }

    #[test]
    fn two_packets_one_page() {
        // Two segments, both < 255 → two separate packets.
        let mut body = Vec::new();
        body.extend_from_slice(b"AAA");
        body.extend_from_slice(b"BB");
        let data = page(1, 0, FLAG_BOS, &[3, 2], &body);
        let packets = read_packets(&data).unwrap();
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].data, b"AAA");
        assert_eq!(packets[1].data, b"BB");
    }

    #[test]
    fn packet_spanning_two_pages() {
        // First page: one 255 segment (packet continues).
        let big = vec![b'x'; 255];
        let p1 = page(1, 0, FLAG_BOS, &[255], &big);
        // Second page: continuation, one 4-byte segment (terminates).
        let p2 = page(1, 1, FLAG_CONTINUATION, &[4], b"tail");
        let mut data = p1;
        data.extend_from_slice(&p2);

        let packets = read_packets(&data).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].data.len(), 259);
        assert_eq!(&packets[0].data[255..], b"tail");
    }

    #[test]
    fn two_interleaved_logical_streams() {
        // Serial 1 and serial 2 interleaved — must not bleed together.
        let a = page(1, 0, FLAG_BOS, &[3], b"AAA");
        let b = page(2, 0, FLAG_BOS, &[3], b"BBB");
        let mut data = a;
        data.extend_from_slice(&b);

        let packets = read_packets(&data).unwrap();
        assert_eq!(packets.len(), 2);
        let s1: Vec<_> = packets.iter().filter(|p| p.serial == 1).collect();
        let s2: Vec<_> = packets.iter().filter(|p| p.serial == 2).collect();
        assert_eq!(s1.len(), 1);
        assert_eq!(s2.len(), 1);
        assert_eq!(s1[0].data, b"AAA");
        assert_eq!(s2[0].data, b"BBB");
    }

    #[test]
    fn page_reader_yields_all_pages() {
        let a = page(1, 0, FLAG_BOS, &[3], b"AAA");
        let b = page(1, 1, 0, &[3], b"BBB");
        let mut data = a;
        data.extend_from_slice(&b);
        let pages: Result<Vec<_>, _> = PageReader::new(&data).collect();
        assert_eq!(pages.unwrap().len(), 2);
    }
}
