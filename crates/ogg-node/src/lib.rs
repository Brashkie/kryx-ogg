//! napi bridge for `ogg-core`.
//!
//! This is deliberately thin: it converts a JS `Buffer` into `&[u8]`, calls
//! `ogg_core::read_packets`, and hands back a flat list of packets (serial +
//! bytes + optional granule). Grouping packets into logical streams and the
//! `streams()` / `packets()` shape live in the TypeScript SDK — the native
//! layer stays a minimal, stable data boundary.
//!
//! Per the M2 decision ("a streaming API over an eager engine"), this exposes
//! the eager `read_packets`. The public TS API can later swap this for a true
//! iterator without users changing a line.

use napi::bindgen_prelude::*;
use napi_derive::napi;

/// A single reassembled Ogg packet, as seen from JavaScript.
#[napi(object)]
pub struct NativePacket {
    /// Serial number of the logical stream this packet belongs to.
    /// (u32 widened to i64 — JS has no u32; the value is always ≥ 0.)
    pub serial: i64,
    /// The packet payload bytes.
    pub data: Buffer,
    /// Granule position of the page on which this packet completed, or `null`
    /// if none. (u64 widened via string to avoid JS 53-bit precision loss.)
    pub granule_position: Option<BigInt>,
}

/// Parse a complete Ogg buffer and return all packets, in completion order,
/// tagged with their logical stream serial.
///
/// Throws if the buffer is not valid Ogg (bad capture pattern, unsupported
/// version, truncation, or CRC mismatch).
#[napi]
pub fn read_packets(data: Buffer) -> Result<Vec<NativePacket>> {
    let bytes: &[u8] = &data;
    let packets = ogg_core::read_packets(bytes)
        .map_err(|e| Error::new(Status::InvalidArg, format!("{e}")))?;

    let out = packets
        .into_iter()
        .map(|p| NativePacket {
            serial: i64::from(p.serial),
            data: p.data.into(),
            granule_position: p.granule_position.map(BigInt::from),
        })
        .collect();
    Ok(out)
}

/// The `ogg-core` version this addon was built against.
#[napi]
pub fn core_version() -> &'static str {
    ogg_core::VERSION
}

/// A packet to write into an Ogg stream: payload bytes + granule position.
#[napi(object)]
pub struct WritePacket {
    /// The packet payload bytes.
    pub data: Buffer,
    /// Granule position to stamp on the page where this packet completes.
    /// (u64 as BigInt — JS numbers can't hold the full 64-bit range.)
    pub granule_position: BigInt,
}

/// Write a single logical Ogg stream from a list of packets.
///
/// The writer owns all the mechanics — segmenting large packets, packing pages,
/// setting BOS on the first page and EOS on the last, sequence numbers, and
/// CRCs. Returns the complete Ogg bytes.
#[napi]
pub fn write_packets(serial: i64, packets: Vec<WritePacket>) -> Buffer {
    let mut writer = ogg_core::OggWriter::new(serial as u32);
    for p in &packets {
        let (_signed, granule, _lossless) = p.granule_position.get_u64();
        writer.write_packet(&p.data, granule);
    }
    writer.finish().into()
}
