/**
 * @kryxjs/ogg — a small, dependency-free reader for the Ogg container format.
 *
 * @kryxjs/ogg knows only Ogg. It hands out raw packet bytes plus each packet's
 * serial number and (uninterpreted) granule position. What a packet *means* —
 * an OpusHead, a Vorbis header, an audio frame — is the codec's job, one layer
 * up. The same reader serves Opus, Vorbis, FLAC, and Theora without knowing any
 * of them.
 *
 * @example
 * ```ts
 * import { readFile } from 'node:fs/promises'
 * import { OggReader } from '@kryxjs/ogg'
 *
 * const bytes = await readFile('audio.opus')
 * const reader = new OggReader(bytes)
 *
 * for await (const stream of reader.streams()) {
 *   console.log('stream', stream.serial)
 *   for await (const packet of stream.packets()) {
 *     // hand packet.data to a codec
 *   }
 * }
 * ```
 *
 * ## Status (0.1.1 — M2)
 *
 * - ✅ M1: Ogg core in Rust — pages, CRC-32 validation, packet reassembly,
 *   logical streams (zero dependencies).
 * - ✅ M2: napi bridge + this TypeScript SDK (`OggReader` / `OggStream`).
 * - ✅ M3: writing (`OggWriter`) — read AND write Ogg from JavaScript.
 * - ⏸ M4: Opus integration (`OpusStream` in `@kryxjs/codecs-opus`).
 * - ⏸ M5: stable 0.1.1.
 */

export { OggReader, OggStream } from './reader'
export { OggWriter } from './writer'
export type { OggPacket } from './types'
import { native } from './native'

/** The package version. */
export const VERSION = '0.1.1' as const

/** The version of the native `ogg-core` the loaded addon was built against. */
export function coreVersion(): string {
  return native().coreVersion()
}
