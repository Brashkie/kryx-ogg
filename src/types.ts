/**
 * Public types for @kryxjs/ogg.
 *
 * These describe what the reader hands back. They are intentionally
 * codec-agnostic: a packet is just bytes plus the metadata Ogg itself defines
 * (which logical stream it came from, and the raw granule position). What the
 * bytes *mean* is the codec's job.
 */

/**
 * A single reassembled Ogg packet.
 */
export interface OggPacket {
  /** Serial number of the logical stream this packet belongs to. */
  readonly serial: number
  /** The packet payload bytes. */
  readonly data: Buffer
  /**
   * Raw granule position of the page on which this packet completed, or `null`
   * if none. Its meaning is codec-defined (Opus: 48 kHz samples; Vorbis:
   * samples at the stream rate; Theora: a frame number) — @kryxjs/ogg does not
   * interpret it. A `bigint` because granule positions are 64-bit.
   */
  readonly granulePosition: bigint | null
}
