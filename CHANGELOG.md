# Changelog

All notable changes to `@kryxjs/ogg` are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-08-05

First stable release. Reads and writes the Ogg container format from Rust and
JavaScript, with zero runtime dependencies.

### Added

- **Reading** (`OggReader`): parse Ogg pages, validate CRC-32, reassemble
  packets (including packets that span pages), and separate multiple interleaved
  logical streams by serial number. Streaming API (`streams()` → `packets()`)
  over async iterators.
- **Writing** (`OggWriter`): turn packets plus granule positions back into valid
  Ogg pages — segmenting large packets, packing pages, setting BOS/EOS flags,
  incrementing sequence numbers, and computing CRCs. The caller never builds a
  page by hand.
- Raw granule positions and serial numbers are exposed but never interpreted:
  what a packet means is the codec's job, one layer up. This keeps the container
  generic across Opus, Vorbis, FLAC, and Theora.
- Pure-Rust core (`ogg-core`, zero dependencies) plus a napi bridge
  (`ogg-node`) and a TypeScript SDK.
- Pairs with `@kryxjs/codecs-opus` (via its `OpusStream`) to read and write real
  `.opus` files end to end.

### Notes

- The writer emits one packet per page (valid Ogg, not maximally packed);
  multi-packet page packing may come later without an API change.
- The public API is a "streaming API over an eager engine": the internals can
  become truly incremental later without any user-facing change.

[0.1.0]: https://github.com/Brashkie/kryx-ogg/releases/tag/v0.1.0
