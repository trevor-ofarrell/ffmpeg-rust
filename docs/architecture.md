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

Early shared types intentionally encode invariants at construction boundaries. `Rational` rejects zero denominators and normalizes signs. `ByteReader` bounds-checks every read and returns typed EOF errors without advancing past failed reads. `ByteWriter` validates constrained widths such as 24-bit integer fields. `BitReader` performs bounded MSB-first bit reads, peeks, skips, and byte alignment without advancing on failed reads. `BitWriter` writes MSB-first fields, validates requested widths, and zero-pads byte alignment explicitly. `Dictionary` preserves metadata insertion order, uses ASCII case-insensitive matching by default, and rejects empty/NUL-containing keys. `OptionSet` stores AVOption-like descriptors and current values with type/range validation before mutation. `Adler32` and `Crc32` provide streaming checksum state plus one-shot helpers. `Packet` represents missing PTS/DTS with `Option<i64>` over an internal `AV_NOPTS_VALUE`. `Frame` currently models typed audio/video payload shells with explicit shape validation.

`AvioReader` and `AvioWriter` wrap seekable byte streams with typed `avutil` errors. Exact reads rewind to the starting offset on EOF so parsers can fail without losing their previous probe position. `ProbeRegistry` records format probe descriptors and deterministically scores prefix or offset signature, MIME type, and extension matches. The current hand-written descriptors cover AVI and MOV/MP4.

`NullMuxer` implements the first muxer-shaped sink: it discards packet payload bytes while preserving observable accounting for packets, bytes, stream indexes, durations, and last timestamps.

`HashMuxer` implements an initial packet-data hash sink over write order, using `avutil` checksum state for Adler-32 and IEEE CRC-32 while tracking packet and byte counts.

`FrameCrcMuxer` records one CRC-32 line per packet with stream index, timestamps, duration, payload size, and checksum in write order.

`RawVideoDecoder` implements the first codec-shaped decoder path for fixed-size raw video packets and emits `avutil::Frame` values with validated pixel format geometry.

`PcmS16leDecoder` decodes packed little-endian signed 16-bit PCM packets into `avutil::AudioFrame` values, requiring packets to contain whole interleaved sample frames.

`WavDemuxer` implements the first demuxer-shaped parser path for RIFF/WAVE PCM s16le data. It validates the RIFF/WAVE container, `fmt ` and `data` chunks, PCM stream fields, RIFF word padding, and whole sample-frame payloads before emitting the data chunk as a stream-0 packet.

`WavMuxer` implements the matching initial muxer path for canonical RIFF/WAVE PCM s16le output. It validates stream parameters, stream-0 packet ownership, whole sample-frame packet payloads, and classic RIFF size limits before rendering a WAV byte stream.

`AviDemuxer` implements an initial constrained RIFF AVI parser for video-only files. It parses `avih`, `strh`, and `strf` metadata, walks simple `movi` chunks including `rec ` lists, emits packet payloads with per-stream PTS/DTS counters, and provides an AVI probe descriptor for the offset-8 `AVI ` form tag plus common extensions and MIME types.

`AviMuxer` implements the matching initial constrained RIFF AVI writer for one RGB24 video stream. It validates stream-0 fixed-size packet payloads, writes classic `avih`/`strh`/`strf` headers, emits `00db` chunks in a `movi` list, handles RIFF word padding, and round-trips through the current demuxer.

`MovDemuxer` implements the first constrained ISOBMFF/MOV/MP4 parser path. It validates top-level and nested box bounds, supports classic, extended-size, and size-zero boxes, parses `ftyp`, `moov/mvhd`, `trak/tkhd`, and `mdia/mdhd` metadata, extracts common movie-level and track-level `udta/meta/ilst` metadata values from `data` atoms for UTF-8 and UTF-16 text fields, classic `gnre` genre indexes, one-byte integer/boolean atoms, iTunes-style freeform `----` atoms, `covr` cover-art payloads, and track/disc number pairs, provides a MOV/MP4 probe descriptor for the `ftyp` box signature plus common extensions and MIME types, rejects fragmented `mvex`/`moof` layouts, edit-list `edts` boxes, multiple populated tracks, and multiple `stsd` sample entries as unsupported, and emits packets for a single populated track with simple `stsd`, `stts`, `ctts`, `stsc`, `stsz`, `stss`, and `stco`/`co64` sample tables, including generic sample-entry codec parameter records, VisualSampleEntry field parsing for known video sample entries and raw entries with full visual payloads, VisualSampleEntry child-box validation with structured `avcC` version/profile/level/NAL-length-size/SPS/PPS parsing, structured `hvcC` profile/timing/NAL-length-size/NAL-array parsing, parsed `pasp` pixel aspect ratio, parsed `nclx`/`nclc` `colr` color parameters, `rICC`/`prof` ICC profile payloads, multi-chunk `stsc` entry transitions, and sample ranges split across multiple `mdat` boxes.

`Yuv4MpegDemuxer` implements an initial yuv4mpegpipe parser for progressive 4:2:0 `C420jpeg` streams. It parses the stream header, validates dimensions, frame rate, interlace, chroma, and frame headers, then emits one packet per raw frame with monotonically increasing PTS.

`Yuv4MpegMuxer` implements the matching initial yuv4mpegpipe muxer path. It writes a canonical progressive `C420jpeg` stream header and one `FRAME` record per exact-size yuv420p stream-0 packet.

`Image2Demuxer` implements an initial image2 packetizer over caller-provided image entries. It supports a single literal image path or a contiguous `%d`/`%0Nd` numbered sequence, sorts frames by frame number, rejects gaps and duplicates, and emits one packet per image with path side data.

`Image2Muxer` implements the matching initial image2 muxer path. It maps stream-0 packet payloads to single-image or numbered output entries without touching the filesystem, validating path generation and non-empty image payloads before recording entries.

`RawVideoDemuxer` implements an initial rawvideo packet slicer for fixed-size `gray`, `rgb24`, `rgba`, and `yuv420p` frame payloads. It validates dimensions, frame rate, yuv420p parity, and whole-frame byte counts before emitting monotonically timed packets.

`RawVideoMuxer` implements the matching initial rawvideo muxer path. It validates stream-0 packet ownership and exact fixed-size frame payloads before concatenating raw frame bytes and updating frame accounting.

`PcmS16leDemuxer` implements an initial raw PCM audio packet slicer for packed little-endian signed 16-bit samples. It validates sample rate, channel count, packet sample count, and whole interleaved sample-frame input before emitting packets with sample-count durations.

`PcmS16leMuxer` implements the matching initial raw PCM audio muxer path. It validates stream-0 packet ownership and whole interleaved sample-frame payloads before concatenating payload bytes and updating packet/sample accounting.

## CLI Compatibility Model

CLI compatibility is treated as a first-class surface. The current implementation supports version banners for `ffmpeg-rs` and `ffprobe-rs` plus an internal parser that groups a small known set of FFmpeg-style options onto global scope, the next `-i` input, or the next output filename. `IoPlan` turns parsed input/output URLs into validated file, pipe, or protocol endpoints for command execution. `ffmpeg-rs` has an initial constrained Rust-native execution path for one local MOV/MP4, AVI, PCM s16le RIFF/WAVE, raw `pcm_s16le`, explicit `rawvideo`, yuv4mpegpipe, or explicit image2 single-file/numbered-sequence input to stdout `-f null -` or `-f framecrc -`, using `MovDemuxer`, `AviDemuxer`, `WavDemuxer`, `PcmS16leDemuxer`, `RawVideoDemuxer`, `Yuv4MpegDemuxer`, `Image2Demuxer`, `NullMuxer`, and `FrameCrcMuxer` without FFmpeg runtime linkage. It can also packet-copy explicit raw `pcm_s16le` input to a local `-f s16le` file through `PcmS16leMuxer`, explicit raw `pcm_s16le` input to a local `-f wav` file through `WavMuxer`, explicit rawvideo input to a local `-f rawvideo` file through `RawVideoMuxer`, explicit raw `yuv420p` input to a local `-f yuv4mpegpipe` file through `Yuv4MpegMuxer`, explicit raw `rgb24` input to a local `-f avi` file through `AviMuxer`, and explicit image2 input to local `-f image2` file or numbered-pattern outputs through `Image2Muxer`, all with create-new output semantics. The rawvideo path currently requires explicit `-f rawvideo`, `-pix_fmt`, `-s`, and `-r` input options; AVI is accepted with explicit `-f avi`, RIFF/AVI signature, or `.avi` extension; yuv4mpegpipe is accepted with explicit `-f yuv4mpegpipe` or YUV4MPEG2/.y4m detection; and image2 currently requires explicit `-f image2` plus `-framerate` or `-r` for a single local image file or contiguous `%d`/`%0Nd` numbered sequence discovered in the pattern's parent directory, with file-scoped `-start_number` support on image2 input and output groups. `ffprobe-rs` also has Rust-native command execution paths for local seekable MOV/MP4 and constrained AVI files: auto-detection registers AVI and MOV descriptors in `ProbeRegistry`, routes matched inputs to `AviDemuxer` or `MovDemuxer`, `-f avi`/`-f mov`/`-f mp4` can force those constrained demuxer paths, and both render small `-show_format`, `-show_streams`, and `-show_packets` summaries with default and JSON writers. Stream summaries include initial codec type/name, MOV codec profile and level when derived from parsed `avcC` or `hvcC`, MOV `avcC`-derived `is_avc` and `nal_length_size`, FourCC tag string and numeric tag, dimensions, AVI `bits_per_raw_sample` from parsed BITMAPINFOHEADER bit counts, MOV raw VisualSampleEntry depth as `bits_per_raw_sample`, MOV sample-entry `extradata_size` when parsed extra data is non-empty, MOV visual sample aspect ratio, display aspect ratio, color range, color space, color transfer, color primaries, timing, average frame rate when derivable, durations, frame counts, and tags. Unsupported command forms still exit non-zero and are recorded as incomplete in the ledger.

## Test Architecture

The test hierarchy is unit tests first, followed by golden parser tests, differential tests against the pinned oracle, FATE-derived tests, fuzz harnesses, integration tests, and performance benchmarks. Current coverage is limited to unit and in-process command tests for the first primitives and the initial ffprobe MOV path; separate cargo-built integration test executables are currently blocked by Windows Application Control in this environment.

## FFI And Export Policy

The runtime implementation must not link against FFmpeg/libav*. FFI is reserved for platform APIs, hardware APIs, SIMD support, and eventual C ABI compatibility layers. No unsafe Rust is present in the initial slice.

## Unsafe Policy

Unsafe Rust requires a safety comment and a test or documented invariant. Crates currently forbid unsafe code unless a later component documents why it is required.

## Performance Policy

Correctness and parity come before performance. Performance-sensitive code should keep allocation behavior measurable and should gain benchmarks once semantic parity exists.
