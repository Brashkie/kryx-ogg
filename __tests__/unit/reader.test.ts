/**
 * Unit tests for the @kryxjs/ogg SDK logic.
 *
 * These run against a mocked native contract: `readPackets` returns whatever
 * the test programs into `mockState`, and we assert the SDK handles it — stream
 * grouping, async iteration, error propagation. They do NOT test Ogg parsing
 * (that's `cargo test`) or the napi boundary (that's the integration tests).
 *
 * The mock targets `../../index.js` — the repo-root loader that `src/native.ts`
 * imports as `../index.js`. Both resolve to the same module.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest'

// Mock state, created in the hoisted scope so the vi.mock factory (which vitest
// hoists above imports) can safely reference it. Programmable per test.
const mockState = vi.hoisted(() => ({
  packets: [] as { serial: number; data: Buffer; granulePosition: bigint | null }[],
  throwMessage: null as string | null,
  // Captures what the SDK passed to the native writer, for write-side asserts.
  lastWrite: null as { serial: number; packets: unknown[] } | null,
}))

// Mock the native addon at the path native.ts imports (../index.js from src/,
// which is ../../index.js from here — same module).
vi.mock('../../index.js', () => ({
  readPackets(_data: Buffer) {
    if (mockState.throwMessage !== null) {
      throw new Error(mockState.throwMessage)
    }
    return mockState.packets
  },
  writePackets(serial: number, packets: unknown[]) {
    mockState.lastWrite = { serial, packets }
    return Buffer.from([0x4f, 0x67, 0x67, 0x53]) // "OggS" — a stand-in
  },
  coreVersion: () => '0.0.0-mock',
}))

// Import the SDK AFTER the mock is registered.
const { OggReader, OggStream, OggWriter, VERSION } = await import('../../src/index')

type MockPacket = { serial: number; data: Buffer; granulePosition: bigint | null }

function pkt(serial: number, bytes: number[], granule: bigint | null = null): MockPacket {
  return { serial, data: Buffer.from(bytes), granulePosition: granule }
}

function resetMock(): void {
  mockState.packets = []
  mockState.throwMessage = null
  mockState.lastWrite = null
}

beforeEach(() => {
  resetMock()
})

describe('VERSION', () => {
  it('is the stable version', () => {
    expect(VERSION).toBe('0.1.0')
  })
})

describe('OggReader.streams()', () => {
  it('groups packets by serial into one stream per serial', async () => {
    mockState.packets = [pkt(1, [1]), pkt(1, [2]), pkt(2, [3])]
    const reader = new OggReader(Buffer.alloc(0))

    const streams: InstanceType<typeof OggStream>[] = []
    for await (const s of reader.streams()) streams.push(s)

    expect(streams).toHaveLength(2)
    expect(streams.map((s) => s.serial)).toEqual([1, 2])
  })

  it('preserves first-seen order of streams', async () => {
    mockState.packets = [pkt(5, [1]), pkt(2, [2]), pkt(5, [3]), pkt(9, [4])]
    const reader = new OggReader(Buffer.alloc(0))
    const streams = await reader.toArray()
    expect(streams.map((s) => s.serial)).toEqual([5, 2, 9])
  })

  it('yields the packets belonging to each stream', async () => {
    mockState.packets = [pkt(1, [10]), pkt(2, [20]), pkt(1, [11])]
    const reader = new OggReader(Buffer.alloc(0))

    const byStream = new Map<number, number[][]>()
    for await (const stream of reader.streams()) {
      const collected: number[][] = []
      for await (const p of stream.packets()) collected.push([...p.data])
      byStream.set(stream.serial, collected)
    }

    expect(byStream.get(1)).toEqual([[10], [11]])
    expect(byStream.get(2)).toEqual([[20]])
  })
})

describe('OggStream', () => {
  it('exposes its serial and toArray()', async () => {
    mockState.packets = [pkt(7, [1]), pkt(7, [2])]
    const reader = new OggReader(Buffer.alloc(0))
    const [stream] = await reader.toArray()
    expect(stream.serial).toBe(7)
    const packets = await stream.toArray()
    expect(packets).toHaveLength(2)
  })

  it('carries granule position through unchanged', async () => {
    mockState.packets = [pkt(1, [1], 48000n)]
    const reader = new OggReader(Buffer.alloc(0))
    const [stream] = await reader.toArray()
    const [packet] = await stream.toArray()
    expect(packet.granulePosition).toBe(48000n)
  })
})

describe('OggReader.packets()', () => {
  it('iterates every packet across all streams', async () => {
    mockState.packets = [pkt(1, [1]), pkt(2, [2]), pkt(1, [3])]
    const reader = new OggReader(Buffer.alloc(0))
    const all = []
    for await (const p of reader.packets()) all.push(p)
    expect(all).toHaveLength(3)
  })
})

describe('input handling', () => {
  it('accepts a Uint8Array as well as a Buffer', async () => {
    mockState.packets = [pkt(1, [1])]
    const reader = new OggReader(new Uint8Array([0, 1, 2]))
    const streams = await reader.toArray()
    expect(streams).toHaveLength(1)
  })

  it('parses lazily and caches — result is stable after first parse', async () => {
    mockState.packets = [pkt(1, [1])]
    const reader = new OggReader(Buffer.alloc(0))
    await reader.toArray()
    mockState.packets = [pkt(9, [9])]
    const streams = await reader.toArray()
    expect(streams.map((s) => s.serial)).toEqual([1])
  })
})

describe('error propagation', () => {
  it('surfaces a native error when the addon throws', async () => {
    mockState.throwMessage = 'Ogg CRC mismatch at page starting byte 0'
    const reader = new OggReader(Buffer.alloc(0))
    await expect(async () => {
      for await (const _ of reader.streams()) {
        // consume
      }
    }).rejects.toThrow(/CRC mismatch/)
  })
})

describe('OggWriter', () => {
  it('passes queued packets to the native writer with its serial', () => {
    const writer = new OggWriter(77)
    writer.write(Buffer.from([1, 2, 3]), 960n)
    writer.write(Buffer.from([4, 5]), 1920n)
    writer.finish()

    expect(mockState.lastWrite?.serial).toBe(77)
    expect(mockState.lastWrite?.packets).toHaveLength(2)
  })

  it('coerces a number granule to bigint', () => {
    const writer = new OggWriter(1)
    writer.write(Buffer.from([1]), 480) // number, not bigint
    writer.finish()

    const p = mockState.lastWrite?.packets[0] as { granulePosition: bigint }
    expect(p.granulePosition).toBe(480n)
  })

  it('defaults granule to 0n when omitted', () => {
    const writer = new OggWriter(1)
    writer.write(Buffer.from([1]))
    writer.finish()
    const p = mockState.lastWrite?.packets[0] as { granulePosition: bigint }
    expect(p.granulePosition).toBe(0n)
  })

  it('accepts a Uint8Array payload', () => {
    const writer = new OggWriter(1)
    writer.write(new Uint8Array([9, 9, 9]), 0n)
    writer.finish()
    expect(mockState.lastWrite?.packets).toHaveLength(1)
  })

  it('is chainable', () => {
    const writer = new OggWriter(1)
    const result = writer.write(Buffer.from([1])).write(Buffer.from([2]))
    expect(result).toBe(writer)
    writer.finish()
    expect(mockState.lastWrite?.packets).toHaveLength(2)
  })

  it('throws if written to after finish()', () => {
    const writer = new OggWriter(1)
    writer.write(Buffer.from([1]))
    writer.finish()
    expect(() => writer.write(Buffer.from([2]))).toThrow(/after finish/)
  })

  it('throws if finish() is called twice', () => {
    const writer = new OggWriter(1)
    writer.finish()
    expect(() => writer.finish()).toThrow(/already called/)
  })

  it('exposes its serial (random default is a 32-bit uint)', () => {
    const writer = new OggWriter()
    expect(writer.serial).toBeGreaterThanOrEqual(0)
    expect(writer.serial).toBeLessThanOrEqual(0xffffffff)
    expect(Number.isInteger(writer.serial)).toBe(true)
  })
})
