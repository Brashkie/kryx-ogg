# @kryxjs/ogg

**English** · [Español](./README.es.md)

[![CI](https://github.com/Brashkie/kryx-ogg/actions/workflows/ci.yml/badge.svg)](https://github.com/Brashkie/kryx-ogg/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/@kryxjs/ogg)](https://www.npmjs.com/package/@kryxjs/ogg)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](./LICENSE)

A small, dependency-free reader and writer for the **Ogg** container format
(RFC 3533), written in pure Rust with a TypeScript SDK. Part of the
[Kryx](https://github.com/Brashkie) multimedia ecosystem.

> **Status: stable (0.1.0).** Reads and writes Ogg from Rust and JavaScript:
> pages, CRC-32 validation, packet reassembly, logical streams, and a writer —
> all with zero runtime dependencies. Pairs with `@kryxjs/codecs-opus` to read
> and write real `.opus` files end to end.

## Philosophy

`@kryxjs/ogg` knows **only Ogg**. It hands out raw packet bytes plus each
packet's serial number and (uninterpreted) granule position. What a packet
*means* — an `OpusHead`, a Vorbis header, an audio frame — is the codec's job,
one layer up. The same reader serves Opus, Vorbis, FLAC, and Theora without
knowing any of them.

This is the first fully-owned, zero-C package in Kryx: no vendored library, no
Zig, just Rust. It's what makes `@kryxjs/codecs-opus` self-sufficient
end-to-end — read and write real `.opus` files with no external tool.

## Crates

- **`ogg-core`** — the pure-Rust engine. Zero dependencies. Reusable from WASM,
  a CLI, Tauri, other languages, or pure-Rust tests — it has no Node coupling.
- **`ogg-node`** — the napi bridge to Node.js.

## Roadmap

| Milestone | Scope |
|-----------|-------|
| **M1** ✅ | `ogg-core` reading: pages, CRC-32 validation, packet reassembly, logical streams |
| **M2** ✅ | Public API + napi (`OggReader` → `streams()` → `packets()`) + TS SDK |
| **M3** ✅ | `ogg-core` writing: `OggWriter` (valid pages, CRC, mux one stream) |
| **M4** ✅ | Opus integration (`OpusStream` in `codecs-opus` reads `OpusHead`/`OpusTags`) |
| **M5** ✅ | Stable 0.1.0 |

See **[ROADMAP.md](./ROADMAP.md)** for what comes next: completing the
format (incremental reading, seeking, chained streams, muxing),
performance (zero-copy, SIMD CRC-32, benchmarks vs libogg/ffmpeg), and the
differentiators (robust/repair reading, diagnostics, WASM, a CLI, fuzzing).

## References

- RFC 3533 — The Ogg Encapsulation Format
- RFC 7845 — Ogg Encapsulation for the Opus Audio Codec

## Usage

### Reading

```ts
import { readFile } from 'node:fs/promises'
import { OggReader } from '@kryxjs/ogg'

const bytes = await readFile('audio.opus')
const reader = new OggReader(bytes)

for await (const stream of reader.streams()) {
  console.log('logical stream', stream.serial)
  for await (const packet of stream.packets()) {
    // packet.data       → Buffer (raw packet bytes)
    // packet.serial     → number (logical stream serial)
    // packet.granulePosition → bigint | null (codec-defined; not interpreted)
  }
}
```

### Writing

```ts
import { OggWriter } from '@kryxjs/ogg'

const bytes = new OggWriter(serial)
  .write(packetBytes, 960n)   // packet + granule position (bigint or number)
  .write(moreBytes, 1920n)
  .finish()                    // → Buffer with the complete Ogg stream
```

The writer owns every mechanic — segmenting large packets, packing pages,
BOS/EOS flags, sequence numbers, and CRCs. You supply packets and granule
positions; you never build a page by hand.

`@kryxjs/ogg` returns and accepts **raw packets** — it does not interpret them.
To turn an Ogg-Opus file's packets into audio, pair it with
`@kryxjs/codecs-opus`, whose `OpusStream` reads the `OpusHead`/`OpusTags`
headers over this package.

The API is a *streaming API over an eager engine*: today the native layer parses
the whole buffer up front and the SDK yields through async iterators. The public
shape is final — a future incremental engine can replace the internals without
any user-facing change.

## License

[Apache-2.0](./LICENSE) © Brashkie
