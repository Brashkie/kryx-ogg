//! Ogg page: the on-disk unit of an Ogg bitstream.
//!
//! A page is a 27-byte header, a segment table (the "lacing values"), and a
//! body. Unlike the minimal M6 reader, this captures **every** header field —
//! serial number, granule position, sequence number, and the stored CRC — and
//! validates the CRC. It interprets none of the payload: what the packets mean
//! is the codec's job, one layer up.
//!
//! Reference: RFC 3533 §6.

use crate::crc;
use crate::error::{OggError, OggResult};

/// The 4-byte Ogg page capture pattern: "OggS".
const OGG_MAGIC: &[u8; 4] = b"OggS";

/// Minimum page header size, before the segment table.
const HEADER_LEN: usize = 27;

/// Header-type flag: this page begins with a continued packet.
pub const FLAG_CONTINUATION: u8 = 0x01;
/// Header-type flag: beginning of stream (first page of a logical stream).
pub const FLAG_BOS: u8 = 0x02;
/// Header-type flag: end of stream (last page of a logical stream).
pub const FLAG_EOS: u8 = 0x04;

/// A single parsed Ogg page.
///
/// Fields mirror the wire format. Granule position is exposed **raw**: Ogg
/// defines it per-codec (Opus counts 48 kHz samples; Vorbis counts samples at
/// its own rate; Theora packs a frame number), so interpreting it is not this
/// layer's responsibility.
#[derive(Debug, Clone)]
pub struct OggPage {
    /// Header type flag byte (see `FLAG_*`).
    pub header_type: u8,
    /// Raw granule position (codec-defined meaning; not interpreted here).
    pub granule_position: u64,
    /// Serial number identifying the logical bitstream this page belongs to.
    pub serial: u32,
    /// Page sequence number within its logical bitstream.
    pub sequence: u32,
    /// The lacing values (segment table) for this page.
    pub segment_sizes: Vec<u8>,
    /// The concatenated segment data (page body).
    pub body: Vec<u8>,
}

impl OggPage {
    /// Whether this page begins with a packet continued from the previous page
    /// of the same logical stream.
    pub fn is_continuation(&self) -> bool {
        self.header_type & FLAG_CONTINUATION != 0
    }

    /// Whether this is the first page of its logical stream.
    pub fn is_bos(&self) -> bool {
        self.header_type & FLAG_BOS != 0
    }

    /// Whether this is the last page of its logical stream.
    pub fn is_eos(&self) -> bool {
        self.header_type & FLAG_EOS != 0
    }
}

/// Parse a single Ogg page starting at `data[offset..]`.
///
/// On success returns the page and the offset just past it (where the next
/// page begins). Validates the capture pattern, the version byte, that the
/// segment table and body are fully present, and the stored CRC.
pub fn parse_page(data: &[u8], offset: usize) -> OggResult<(OggPage, usize)> {
    // Need at least the fixed header.
    if offset + HEADER_LEN > data.len() {
        return Err(OggError::Truncated {
            context: "page header",
            offset,
        });
    }

    if &data[offset..offset + 4] != OGG_MAGIC {
        return Err(OggError::BadCapturePattern { offset });
    }

    let version = data[offset + 4];
    if version != 0 {
        return Err(OggError::UnsupportedVersion { version, offset });
    }

    let header_type = data[offset + 5];
    let granule_position = u64::from_le_bytes(
        data[offset + 6..offset + 14]
            .try_into()
            .expect("8 bytes for granule"),
    );
    let serial = u32::from_le_bytes(
        data[offset + 14..offset + 18]
            .try_into()
            .expect("4 bytes for serial"),
    );
    let sequence = u32::from_le_bytes(
        data[offset + 18..offset + 22]
            .try_into()
            .expect("4 bytes for sequence"),
    );
    let stored_crc = u32::from_le_bytes(
        data[offset + 22..offset + 26]
            .try_into()
            .expect("4 bytes for crc"),
    );
    let num_segments = data[offset + 26] as usize;

    // Segment table.
    let seg_table_start = offset + HEADER_LEN;
    let seg_table_end = seg_table_start + num_segments;
    if seg_table_end > data.len() {
        return Err(OggError::Truncated {
            context: "segment table",
            offset,
        });
    }
    let segment_sizes: Vec<u8> = data[seg_table_start..seg_table_end].to_vec();
    let body_len: usize = segment_sizes.iter().map(|&s| s as usize).sum();

    // Body.
    let body_start = seg_table_end;
    let body_end = body_start + body_len;
    if body_end > data.len() {
        return Err(OggError::Truncated {
            context: "page body",
            offset,
        });
    }

    // Validate CRC: recompute over the whole page with the CRC field zeroed.
    let page_bytes = &data[offset..body_end];
    let computed = crc_of_page(page_bytes);
    if computed != stored_crc {
        return Err(OggError::CrcMismatch {
            offset,
            stored: stored_crc,
            computed,
        });
    }

    let page = OggPage {
        header_type,
        granule_position,
        serial,
        sequence,
        segment_sizes,
        body: data[body_start..body_end].to_vec(),
    };
    Ok((page, body_end))
}

/// Compute the Ogg CRC of a full page image, treating the 4-byte CRC field
/// (bytes 22..26) as zero, as the spec requires — without mutating the input.
fn crc_of_page(page: &[u8]) -> u32 {
    // The CRC field is always within the fixed 27-byte header.
    debug_assert!(page.len() >= HEADER_LEN);
    let mut crc = crc::Hasher::new();
    crc.update(&page[..22]);
    crc.update(&[0, 0, 0, 0]);
    crc.update(&page[26..]);
    crc.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a minimal valid one-page Ogg stream carrying a single small packet,
    // with a correct CRC, so we can exercise real parsing + CRC validation.
    fn make_page(serial: u32, body: &[u8], header_type: u8) -> Vec<u8> {
        assert!(
            body.len() < 255,
            "test helper only handles a single segment"
        );
        let mut page = Vec::new();
        page.extend_from_slice(b"OggS");
        page.push(0); // version
        page.push(header_type);
        page.extend_from_slice(&0u64.to_le_bytes()); // granule
        page.extend_from_slice(&serial.to_le_bytes());
        page.extend_from_slice(&0u32.to_le_bytes()); // sequence
        page.extend_from_slice(&0u32.to_le_bytes()); // crc placeholder
        page.push(1); // one segment
        page.push(body.len() as u8); // lacing value
        page.extend_from_slice(body);

        // Fill in the CRC over the page with the field zeroed (already zero).
        let crc = crc_of_page(&page);
        page[22..26].copy_from_slice(&crc.to_le_bytes());
        page
    }

    #[test]
    fn parses_a_valid_page() {
        let data = make_page(0xDEAD_BEEF, b"hello", FLAG_BOS);
        let (page, next) = parse_page(&data, 0).expect("valid page");
        assert_eq!(page.serial, 0xDEAD_BEEF);
        assert_eq!(page.body, b"hello");
        assert!(page.is_bos());
        assert_eq!(next, data.len());
    }

    #[test]
    fn rejects_bad_magic() {
        let mut data = make_page(1, b"x", 0);
        data[0] = b'X';
        assert!(matches!(
            parse_page(&data, 0),
            Err(OggError::BadCapturePattern { offset: 0 })
        ));
    }

    #[test]
    fn rejects_bad_crc() {
        let mut data = make_page(1, b"x", 0);
        data[22] ^= 0xFF; // corrupt the stored CRC
        assert!(matches!(
            parse_page(&data, 0),
            Err(OggError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn rejects_truncated_body() {
        let mut data = make_page(1, b"abcde", 0);
        data.truncate(data.len() - 2);
        assert!(matches!(
            parse_page(&data, 0),
            Err(OggError::Truncated { .. })
        ));
    }
}
