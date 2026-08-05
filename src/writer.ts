/**
 * The public writer API for @kryxjs/ogg.
 *
 * Symmetric to OggReader:
 *
 *   const writer = new OggWriter(serial)
 *   writer.write(packetBytes, granulePosition)
 *   writer.write(moreBytes, nextGranule)
 *   const oggBytes = writer.finish()
 *
 * The caller supplies packets plus a granule position; the writer owns every
 * mechanic — segmenting large packets, packing pages, BOS/EOS flags, sequence
 * numbers, and CRCs. Granule position is first-class from day one so a codec
 * layer (e.g. @kryxjs/codecs-opus) can write real `.opus` timestamps without an
 * API change.
 *
 * Like the reader, this accumulates packets and produces the whole stream at
 * `finish()`. A truly incremental writer can replace the internals later
 * without changing this surface.
 */

import { native } from './native'

/** A packet queued for writing. */
interface QueuedPacket {
  data: Buffer
  granulePosition: bigint
}

/**
 * Writes a single logical Ogg stream.
 */
export class OggWriter {
  readonly serial: number
  #packets: QueuedPacket[] = []
  #finished = false

  /**
   * @param serial Serial number identifying the logical stream. Defaults to a
   *   random 32-bit value, which is the conventional way to pick an Ogg serial.
   */
  constructor(serial: number = (Math.random() * 0xffffffff) >>> 0) {
    this.serial = serial >>> 0
  }

  /**
   * Queue a packet, stamping `granulePosition` on the page where it completes.
   *
   * @param data The packet payload.
   * @param granulePosition Codec-defined time position (e.g. Opus: 48 kHz
   *   samples). Accepts a `bigint` (granule is 64-bit) or a `number` for
   *   convenience.
   */
  write(data: Buffer | Uint8Array, granulePosition: bigint | number = 0n): this {
    if (this.#finished) {
      throw new Error('OggWriter: cannot write after finish()')
    }
    const buf = Buffer.isBuffer(data)
      ? data
      : Buffer.from(data.buffer, data.byteOffset, data.byteLength)
    this.#packets.push({
      data: buf,
      granulePosition:
        typeof granulePosition === 'bigint' ? granulePosition : BigInt(granulePosition),
    })
    return this
  }

  /**
   * Finish the stream and return the complete Ogg bytes. The final page is
   * marked end-of-stream. After this, the writer can't be written to again.
   */
  finish(): Buffer {
    if (this.#finished) {
      throw new Error('OggWriter: finish() already called')
    }
    this.#finished = true
    return native().writePackets(this.serial, this.#packets)
  }
}
