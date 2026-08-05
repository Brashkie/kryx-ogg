/**
 * Integration tests — the real napi boundary.
 *
 * These load the ACTUAL native addon (no mock) and drive real Ogg bytes through
 * it, verifying that the TS SDK and the Rust core talk correctly: Buffer → &[u8]
 * conversion, packet objects marshalled back (serial as number, data as Buffer,
 * granule as bigint|null), and Rust errors surfaced as JS exceptions.
 *
 * Requires the native addon to be built first:
 *   npm run build:native:debug
 *
 * They intentionally do NOT use the mock. Run them separately from the unit
 * coverage run (they exercise the boundary, not SDK-logic coverage).
 */

import { describe, it, expect } from 'vitest'
import { OggReader, OggWriter, coreVersion } from '../../src/index'

// Ogg CRC-32: poly 0x04C11DB7, init 0, no reflection, no final xor.
function oggCrc(data: Uint8Array): number {
  let crc = 0 >>> 0
  for (const byte of data) {
    crc = (crc ^ (byte << 24)) >>> 0
    for (let i = 0; i < 8; i++) {
      crc = (crc & 0x80000000) !== 0 ? ((crc << 1) ^ 0x04c11db7) >>> 0 : (crc << 1) >>> 0
    }
  }
  return crc >>> 0
}

function makePage(serial: number, headerType: number, segs: number[], body: Uint8Array): Uint8Array {
  const page: number[] = []
  for (const c of 'OggS') page.push(c.charCodeAt(0))
  page.push(0, headerType)
  for (let i = 0; i < 8; i++) page.push(0) // granule
  page.push(serial & 0xff, (serial >> 8) & 0xff, (serial >> 16) & 0xff, (serial >> 24) & 0xff)
  page.push(0, 0, 0, 0) // sequence
  page.push(0, 0, 0, 0) // crc placeholder
  page.push(segs.length)
  for (const s of segs) page.push(s)
  for (const b of body) page.push(b)
  const arr = Uint8Array.from(page)
  const crc = oggCrc(arr)
  arr[22] = crc & 0xff
  arr[23] = (crc >> 8) & 0xff
  arr[24] = (crc >> 16) & 0xff
  arr[25] = (crc >> 24) & 0xff
  return arr
}

const FLAG_BOS = 0x02

describe('native boundary', () => {
  it('reads a real packet through the actual addon', async () => {
    const page = makePage(42, FLAG_BOS, [2], Uint8Array.from([104, 105]))
    const reader = new OggReader(page)
    const streams = await reader.toArray()
    expect(streams).toHaveLength(1)
    expect(streams[0].serial).toBe(42)
    const [packet] = await streams[0].toArray()
    expect([...packet.data]).toEqual([104, 105])
  })

  it('separates two interleaved logical streams through the real core', async () => {
    const a = makePage(1, FLAG_BOS, [3], Uint8Array.from([65, 65, 65]))
    const b = makePage(2, FLAG_BOS, [3], Uint8Array.from([66, 66, 66]))
    const combined = new Uint8Array(a.length + b.length)
    combined.set(a, 0)
    combined.set(b, a.length)
    const reader = new OggReader(combined)
    const streams = await reader.toArray()
    expect(streams.map((s) => s.serial).sort()).toEqual([1, 2])
  })

  it('surfaces a real CRC error from Rust as a JS exception', async () => {
    const page = makePage(1, FLAG_BOS, [2], Uint8Array.from([1, 2]))
    page[22] ^= 0xff // corrupt stored CRC
    const reader = new OggReader(page)
    await expect(async () => {
      for await (const _ of reader.streams()) {
        // consume
      }
    }).rejects.toThrow()
  })

  it('reports the real ogg-core version through the addon', () => {
    // Validates the full TS → napi → Rust chain: the .node loaded, napi exported
    // the symbol, and Rust returned its CARGO_PKG_VERSION. A mock can't prove
    // this — that's why coreVersion() is covered here, not in the unit suite.
    expect(coreVersion()).toMatch(/^\d+\.\d+\.\d+/)
  })
})

describe('writer → reader roundtrip (real addon)', () => {
  it('round-trips a single packet through write then read', async () => {
    const bytes = new OggWriter(42).write(Buffer.from('hello'), 960n).finish()

    const reader = new OggReader(bytes)
    const streams = await reader.toArray()
    expect(streams).toHaveLength(1)
    expect(streams[0].serial).toBe(42)
    const [packet] = await streams[0].toArray()
    expect(packet.data.toString()).toBe('hello')
    expect(packet.granulePosition).toBe(960n)
  })

  it('round-trips multiple packets in order', async () => {
    const bytes = new OggWriter(7)
      .write(Buffer.from('AAA'), 100n)
      .write(Buffer.from('BB'), 200n)
      .write(Buffer.from('C'), 300n)
      .finish()

    const packets = []
    for await (const p of new OggReader(bytes).packets()) packets.push(p)
    expect(packets.map((p) => p.data.toString())).toEqual(['AAA', 'BB', 'C'])
    expect(packets[2].granulePosition).toBe(300n)
  })

  it('round-trips a large packet that spans pages', async () => {
    const big = Buffer.alloc(70_000, 0xab)
    const bytes = new OggWriter(1).write(big, 1234n).finish()

    const [packet] = await new OggReader(bytes).toArray().then((s) => s[0].toArray())
    expect(packet.data.length).toBe(70_000)
    expect(packet.data.equals(big)).toBe(true)
  })
})
