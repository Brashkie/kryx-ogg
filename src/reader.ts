/**
 * The public reader API for @kryxjs/ogg.
 *
 * Shape:
 *
 *   const reader = new OggReader(bytes)
 *   for await (const stream of reader.streams()) {
 *     for await (const packet of stream.packets()) {
 *       // packet.data: Buffer, packet.serial, packet.granulePosition
 *     }
 *   }
 *
 * This is a *streaming API over an eager engine*: the native layer parses the
 * whole buffer up front, and the SDK groups packets by logical stream and
 * yields them through async iterators. The public shape is the final one — a
 * future incremental engine can replace the internals without changing a line
 * of user code.
 */

import { native } from './native'
import type { OggPacket } from './types'

/**
 * One logical bitstream within an Ogg file, identified by its serial number.
 */
export class OggStream {
  /** Serial number of this logical stream. */
  readonly serial: number

  #packets: OggPacket[]

  /** @internal — constructed by {@link OggReader}. */
  constructor(serial: number, packets: OggPacket[]) {
    this.serial = serial
    this.#packets = packets
  }

  /**
   * Async-iterate the packets of this stream, in order.
   *
   * Async for forward-compatibility: the day the engine becomes truly
   * incremental, this signature already fits — callers don't change.
   */
  async *packets(): AsyncGenerator<OggPacket, void, unknown> {
    for (const packet of this.#packets) {
      yield packet
    }
  }

  /** Collect all packets of this stream into an array. */
  async toArray(): Promise<OggPacket[]> {
    return this.#packets.slice()
  }
}

/**
 * Reads an Ogg bitstream from a complete in-memory buffer.
 */
export class OggReader {
  #streams: Map<number, OggPacket[]> | null = null
  #bytes: Buffer

  /**
   * @param data A complete Ogg bitstream. Accepts a Node `Buffer` or any
   *   `Uint8Array` (the latter is wrapped without copying).
   */
  constructor(data: Buffer | Uint8Array) {
    this.#bytes = Buffer.isBuffer(data)
      ? data
      : Buffer.from(data.buffer, data.byteOffset, data.byteLength)
  }

  /** Parse once (lazily), grouping packets by logical stream serial. */
  #parse(): Map<number, OggPacket[]> {
    if (this.#streams !== null) {
      return this.#streams
    }
    const raw = native().readPackets(this.#bytes)
    const grouped = new Map<number, OggPacket[]>()
    for (const p of raw) {
      const packet: OggPacket = {
        serial: p.serial,
        data: p.data,
        granulePosition: p.granulePosition,
      }
      const list = grouped.get(p.serial)
      if (list === undefined) {
        grouped.set(p.serial, [packet])
      } else {
        list.push(packet)
      }
    }
    this.#streams = grouped
    return grouped
  }

  /**
   * Async-iterate the logical streams in this Ogg file, in first-seen order.
   */
  async *streams(): AsyncGenerator<OggStream, void, unknown> {
    for (const [serial, packets] of this.#parse()) {
      yield new OggStream(serial, packets)
    }
  }

  /** Collect all logical streams into an array. */
  async toArray(): Promise<OggStream[]> {
    const out: OggStream[] = []
    for (const [serial, packets] of this.#parse()) {
      out.push(new OggStream(serial, packets))
    }
    return out
  }

  /**
   * Convenience: iterate every packet across all logical streams, in the order
   * the native layer returned them (completion order).
   */
  async *packets(): AsyncGenerator<OggPacket, void, unknown> {
    for (const packets of this.#parse().values()) {
      for (const packet of packets) {
        yield packet
      }
    }
  }
}
