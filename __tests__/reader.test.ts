import { describe, expect, it } from 'vitest'
import { OggReader, type OggStream, VERSION } from '../src/index'

/**
 * Build a minimal valid Ogg page in JS, with a correct CRC, so tests can
 * exercise the SDK end-to-end through the native addon.
 *
 * NOTE: these tests require the native addon to be built
 * (`npm run build:native:debug`), since they parse through ogg-node.
 */

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

function makePage(
  serial: number,
  sequence: number,
  headerType: number,
  segs: number[],
  body: Uint8Array,
): Uint8Array {
  const page: number[] = []
  for (const c of 'OggS') page.push(c.charCodeAt(0))
  page.push(0) // version
  page.push(headerType)
  for (let i = 0; i < 8; i++) page.push(0) // granule = 0
  // serial (LE)
  page.push(serial & 0xff, (serial >> 8) & 0xff, (serial >> 16) & 0xff, (serial >> 24) & 0xff)
  // sequence (LE)
  page.push(
    sequence & 0xff,
    (sequence >> 8) & 0xff,
    (sequence >> 16) & 0xff,
    (sequence >> 24) & 0xff,
  )
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

describe('@kryxjs/ogg', () => {
  it('exposes a version', () => {
    expect(VERSION).toBe('0.1.0-alpha.0')
  })

  it('reads a single packet from a single-page stream', async () => {
    const body = Uint8Array.from([104, 105]) // "hi"
    const page = makePage(7, 0, FLAG_BOS, [2], body)
    const reader = new OggReader(page)

    const streams: OggStream[] = []
    for await (const s of reader.streams()) streams.push(s)

    expect(streams).toHaveLength(1)
    expect(streams[0].serial).toBe(7)

    const packets = await streams[0].toArray()
    expect(packets).toHaveLength(1)
    expect([...packets[0].data]).toEqual([104, 105])
  })

  it('separates two interleaved logical streams', async () => {
    const a = makePage(1, 0, FLAG_BOS, [3], Uint8Array.from([65, 65, 65]))
    const b = makePage(2, 0, FLAG_BOS, [3], Uint8Array.from([66, 66, 66]))
    const combined = new Uint8Array(a.length + b.length)
    combined.set(a, 0)
    combined.set(b, a.length)

    const reader = new OggReader(combined)
    const streams = await reader.toArray()
    expect(streams.map((s) => s.serial).sort()).toEqual([1, 2])
  })

  it('iterates all packets across streams via reader.packets()', async () => {
    const a = makePage(1, 0, FLAG_BOS, [1], Uint8Array.from([1]))
    const b = makePage(2, 0, FLAG_BOS, [1], Uint8Array.from([2]))
    const combined = new Uint8Array(a.length + b.length)
    combined.set(a, 0)
    combined.set(b, a.length)

    const reader = new OggReader(combined)
    const all = []
    for await (const p of reader.packets()) all.push(p)
    expect(all).toHaveLength(2)
  })

  it('throws on a corrupt CRC', async () => {
    const page = makePage(1, 0, FLAG_BOS, [2], Uint8Array.from([1, 2]))
    page[22] ^= 0xff // corrupt stored CRC
    const reader = new OggReader(page)
    await expect(async () => {
      for await (const _ of reader.streams()) {
        // consume
      }
    }).rejects.toThrow()
  })
})
