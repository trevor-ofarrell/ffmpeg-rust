# Architecture

This repository is a Rust workspace for a compatibility-oriented FFmpeg 8.1.1 rewrite. FFmpeg itself is an oracle for tests, inventory, and documentation; it is not linked or invoked as runtime implementation code.

## Workspace Layout

- `crates/avutil`: shared errors, rational/timebase math, logging, packets, frames, and future buffers, dictionaries, options, pixel/sample formats, hashes, bit I/O, and memory abstractions.
- `crates/avcodec`: codec registries, parser traits, send/receive model, bitstream filters, threading, DSP primitives, and native codecs.
- `crates/avformat`: AVIO-like I/O, protocols, probing, demuxers, muxers, streams, interleaving, metadata, chapters, programs, and stream groups.
- `crates/avfilter`: filter registry, graph parser, scheduler, framesync, format negotiation, sources, sinks, and filters.
- `crates/swscale`: pixel conversion, colorspace conversion, scaling, chroma handling, and dithering.
- `crates/swresample`: sample conversion, channel remixing, resampling, dithering, and matrix handling.
- `crates/avdevice`: platform input/output devices behind Cargo features.
- `crates/fftools`: `ffmpeg-rs` and `ffprobe-rs` command-line compatibility entry points.
- `crates/oracle`: inventory capture from a pinned FFmpeg binary.
- `crates/fate-runner`: FATE mapping and execution front end.
- `xtask`: repeatable project command runner.

## Type Model

Early shared types intentionally encode invariants at construction boundaries. `Rational` rejects zero denominators and normalizes signs. `ByteReader` bounds-checks every read and returns typed EOF errors without advancing past failed reads. `ByteWriter` validates constrained widths such as 24-bit integer fields. `BitReader` performs bounded MSB-first bit reads, peeks, skips, and byte alignment without advancing on failed reads. `BitWriter` writes MSB-first fields, validates requested widths, and zero-pads byte alignment explicitly. `Dictionary` preserves metadata insertion order, uses ASCII case-insensitive matching by default, and rejects empty/NUL-containing keys. `OptionSet` stores AVOption-like descriptors and current values with type/range validation before mutation. `Packet` represents missing PTS/DTS with `Option<i64>` over an internal `AV_NOPTS_VALUE`. `Frame` currently models typed audio/video payload shells with explicit shape validation.

## CLI Compatibility Model

CLI compatibility is treated as a first-class surface. The current implementation supports only version banners for `ffmpeg-rs` and `ffprobe-rs`; unsupported command forms exit non-zero and are recorded as incomplete in the ledger.

## Test Architecture

The test hierarchy is unit tests first, followed by golden parser tests, differential tests against the pinned oracle, FATE-derived tests, fuzz harnesses, integration tests, and performance benchmarks. Current coverage is limited to unit and CLI integration tests for the first primitives.

## FFI And Export Policy

The runtime implementation must not link against FFmpeg/libav*. FFI is reserved for platform APIs, hardware APIs, SIMD support, and eventual C ABI compatibility layers. No unsafe Rust is present in the initial slice.

## Unsafe Policy

Unsafe Rust requires a safety comment and a test or documented invariant. Crates currently forbid unsafe code unless a later component documents why it is required.

## Performance Policy

Correctness and parity come before performance. Performance-sensitive code should keep allocation behavior measurable and should gain benchmarks once semantic parity exists.
