/**
 * Loads the native addon (ogg-node) for the current platform.
 *
 * Pattern proven by @kryxjs/core: a STATIC `import * as addon from
 * '../index.js'`. `napi build` generates that `index.js` loader at the repo
 * root; it already resolves the correct `.node` binary (local dev build or the
 * platform-specific optional-dependency package) for every supported triple.
 * tsup turns this import into a literal `require('../index.js')` in the CJS
 * output and a literal `import` in the ESM output — both handled natively by
 * Node — and `external` in tsup.config keeps the loader out of the bundle.
 *
 * The addon's type contract is declared inline here rather than depending on
 * napi's auto-generated `../index.d.ts`, which can be missing during a fresh
 * build or change between napi versions.
 */

import * as addon from '../index.js'

/** Shape of the native addon surface we rely on. */
export interface NativeOgg {
  readPackets(data: Buffer): NativePacket[]
  writePackets(serial: number, packets: NativeWritePacket[]): Buffer
  coreVersion(): string
}

/** A packet to write, as the native layer expects it. */
export interface NativeWritePacket {
  data: Buffer
  granulePosition: bigint
}

/** A packet as returned by the native layer (serial widened to number). */
export interface NativePacket {
  serial: number
  data: Buffer
  granulePosition: bigint | null
}

/** Get the loaded native addon. */
export function native(): NativeOgg {
  return addon as unknown as NativeOgg
}
