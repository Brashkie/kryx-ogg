# Roadmap — @kryxjs/ogg

`@kryxjs/ogg` 0.1.0 reads and writes Ogg with a correct, dependency-free core.
That's the foundation, not the finish line. This roadmap is where it goes next —
from completing the format, to performance, to the things that make it stand out
rather than just be another Ogg parser.

Guided by the Kryx rules: the container never interprets codecs; scope grows by
proven need; zero runtime dependencies; own the architecture, borrow an
algorithm only when it clearly wins.

---

## ✅ 0.1.0 — Foundation (shipped)

- Reading: pages, CRC-32 validation, packet reassembly across pages, multiple
  logical streams by serial.
- Writing: `OggWriter` — segmenting, page packing, BOS/EOS, sequence, CRC.
- Streaming API (`streams()` → `packets()`) over async iterators.
- Pure-Rust core (zero deps) + napi bridge + TypeScript SDK.
- Pairs with `@kryxjs/codecs-opus` to read/write real `.opus` end to end.

---

## Horizon 1 — Completing Ogg (0.2.x)

The rest of the format, so nothing valid is unreadable and nothing useful is
unwritable.

- **Incremental reading (`from_reader`)** — parse from a byte source that
  arrives in chunks (a socket, a stream) instead of one complete buffer. Same
  reassembly engine, second data source. Turns the "streaming API over an eager
  engine" into genuine streaming without changing the public shape.
- **Seeking** — bisection seek by granule position, so a player can jump to a
  timestamp without decoding from the start. This is where a real seek table and
  the granule handling earn their design.
- **Chained streams** — Ogg files can concatenate independent logical streams
  (e.g. an album as one file, or a re-muxed broadcast). Read across the chain,
  surfacing each segment's own headers.
- **Multi-stream muxing (writer)** — interleave several logical streams (audio +
  metadata, or multiple audio tracks) into one Ogg, with correct page
  interleaving by granule time. Today's writer is single-stream.
- **Multi-packet page packing (writer)** — pack several small packets into one
  page (today it's one-packet-per-page: valid but not compact). Smaller files,
  fewer pages, closer to what `libogg`/`ffmpeg` emit.

---

## Horizon 2 — Performance (0.3.x)

Correct first, then fast. This is where better algorithms — not just clean code
— make the difference, and where being pure-Rust becomes a measurable advantage.

- **Zero-copy reading** — hand out packet slices that borrow the input buffer
  instead of allocating a `Vec` per packet. A lifetime-parameterized reader for
  the in-memory case; the owned-packet API stays for convenience.
- **SIMD CRC-32** — the CRC is the hot loop of both reading and writing. A
  slice-by-8/16 table method, then hardware CRC where available
  (`SSE4.2 CRCyclic`/PCLMULQDQ on x86, `crc32` on ARM). Ogg's CRC isn't the
  reflected variant those instructions compute directly, so this is a real
  engineering problem — and doing it well is a genuine differentiator.
- **Benchmark suite** — criterion benches for read/write/CRC, plus a comparison
  harness against `libogg` and `ffmpeg -f ogg`. Publish the numbers. "As correct
  as libogg, and here's the graph" is what earns trust.
- **Streaming-first internals** — replace the eager engine underneath the API
  with a true incremental parser, so memory stays flat on huge files. The public
  API already anticipates this; here it becomes real.
- **Allocation profile** — reusable buffers in the writer, arena/pool for page
  assembly, so a long encode doesn't churn the allocator.

---

## Horizon 3 — What makes it stand out (0.4.x+)

Beyond parsing correctly — being the Ogg library people reach for.

- **Robust/repair reading** — real files get truncated, corrupted, or start
  mid-stream. A resync mode that scans for the next valid `OggS`, reports the
  damage, and recovers the readable packets instead of throwing. Most parsers
  give up; this one wouldn't.
- **Rich diagnostics** — a `describe()` / inspector that reports page count,
  stream layout, granule continuity, CRC health, and structural warnings — an
  `ogginfo` built in. Invaluable for debugging real-world media.
- **WASM target** — the pure-Rust core has no Node coupling, so it can compile
  to WASM for the browser: read/write Ogg client-side with the same code. This
  is the payoff of the M2 decision to keep `ogg-core` decoupled.
- **More container-codec bridges** — the same pattern as `OpusStream`, for
  Vorbis and FLAC-in-Ogg (`VorbisStream`, `FlacStream` in their codec packages).
  Proves the container really is codec-generic.
- **CLI (`kryx-ogg`)** — inspect, validate, remux, split/join chained streams
  from the terminal. A thin binary over the core.
- **Fuzzing + conformance corpus** — `cargo-fuzz` on the reader, plus a committed
  corpus of real-world and adversarial `.ogg`/`.opus` files. Fuzz-clean parsing
  is a credibility marker for anything that reads untrusted bytes.

---

## Non-goals (for now)

- Interpreting codec payloads inside `@kryxjs/ogg` — that stays in the codec
  packages, always (Rule 1).
- Vendoring `libogg` — the format is simple enough to own outright; wrapping a C
  library here would trade control for nothing.

---

*This roadmap is a direction, not a contract. Items move by proven need — real
files and real users decide what matters next.*
