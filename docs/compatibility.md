# Compatibility

## Target

- FFmpeg version: 8.1.1 "Hoare"
- Release branch: `release/8.1`
- Initial profile: `ffmpeg-8.1.1-default-native`
- Oracle configure baseline: `./configure --disable-gpl --disable-nonfree --disable-doc`

## Compatible Today

- `ffmpeg-rs -version` and `ffmpeg-rs -hide_banner -version` print a version banner naming the pinned target and ABI versions.
- `ffprobe-rs -version` and `ffprobe-rs -hide_banner -version` print a version banner naming the pinned target and ABI versions.
- `fftools` has an initial option parser for a small known FFmpeg option set, preserving option ordering for input/output grouping.
- `fftools` has an initial I/O planning layer that validates input/output presence and classifies file, pipe, and protocol endpoints.
- `avformat` has initial AVIO-like read/write wrappers for seekable byte streams with typed EOF/external errors.
- `avformat` has an initial probe registry for signature, MIME type, and extension scoring.
- `avformat` has an initial null muxer sink that discards packets while reporting packet, byte, duration, and timestamp statistics.
- `avformat` has an initial packet-data hash muxer sink for Adler-32 and IEEE CRC-32.
- `avformat` has an initial framecrc-style packet checksum sink with one CRC-32 record per packet.
- `avformat` has an initial RIFF/WAVE PCM s16le demuxer that validates `fmt ` and `data` chunks and emits the data chunk as one packet.
- `avformat` has an initial RIFF/WAVE PCM s16le muxer that writes canonical headers and stream-0 packet payloads.
- `avformat` has an initial constrained RIFF AVI demuxer for video-only `avih`/`strh`/`strf` files with simple `movi` packet extraction.
- `avformat` has an initial constrained RIFF AVI muxer for one RGB24 video stream with classic headers and `00db` `movi` packet output.
- `avformat` has an initial MOV/MP4 demuxer path for metadata boxes, simple one-track sample table packet extraction, composition-offset PTS, and sync-sample key flags.
- `avformat` has initial yuv4mpegpipe demuxer and muxer support for progressive 4:2:0 `C420jpeg` streams.
- `avformat` has initial image2 demuxer and muxer support for single-image entries and contiguous `%d`/`%0Nd` numbered sequences.
- `avformat` has an initial rawvideo demuxer for fixed-size `gray`, `rgb24`, `rgba`, and `yuv420p` frame payloads.
- `avformat` has an initial rawvideo muxer for fixed-size `gray`, `rgb24`, `rgba`, and `yuv420p` frame payloads.
- `avformat` has an initial raw `pcm_s16le` demuxer for whole interleaved sample-frame packet slicing.
- `avformat` has an initial raw `pcm_s16le` muxer that concatenates validated stream-0 packet payloads.
- `avcodec` has an initial rawvideo decoder for `gray`, `rgb24`, `rgba`, and `yuv420p` packet payloads.
- `avcodec` has an initial packed `pcm_s16le` decoder for mono and multichannel packet payloads.
- `avutil` has initial typed errors, rational normalization/comparison, timestamp rescaling, bounded byte I/O helpers, MSB-first bit reader/writer helpers, metadata dictionary helpers, AVOption-like descriptor/value validation, Adler-32 and IEEE CRC-32 checksum helpers, packet timestamp/flag/side-data skeletons, frame shape validation, and an in-memory logging abstraction.
- `oracle inventory` can execute a pinned FFmpeg binary and capture the required inventory command outputs.

## Incomplete

All media parsing, decoding, encoding, muxing, demuxing, filtering, playback, probing, stream mapping, option ordering, metadata semantics, hardware acceleration, devices, FATE execution, fuzzing, and differential media parity remain incomplete unless a later ledger entry proves otherwise.

## Known Behavior Deltas

- CLI execution support is limited to `-version`; most FFmpeg/ffprobe options return unsupported-command errors.
- The option parser intentionally rejects unknown options and only covers a small initial compatibility set.
- The I/O planner does not execute demuxers, muxers, protocols, or media transforms yet.
- AVIO support is limited to seekable Rust `Read`/`Write` objects and is not yet wired to protocol implementations.
- Probe support is a registry/scoring primitive only; no real demuxer probe table has been generated from the pinned oracle yet.
- Null muxer support is an internal packet sink only; it is not wired to `ffmpeg-rs -f null` execution or FATE yet.
- Hash muxer support is internal and limited to Adler-32/CRC-32 packet-data hashing; it is not wired to CLI execution, MD5/SHA variants, or FATE yet.
- Framecrc muxer support is internal and not byte-identical to FFmpeg framecrc output yet.
- WAV demuxing is internal only and currently limited to RIFF/WAVE PCM s16le with one packet for the data chunk. RF64, WAVE64, WAVE_FORMAT_EXTENSIBLE, float/non-PCM formats, packet chunking, probing, CLI execution, differential tests, FATE, and fuzzing are pending.
- WAV muxing is internal only and currently limited to canonical RIFF/WAVE PCM s16le stream-0 payloads. RF64, WAVE64, WAVE_FORMAT_EXTENSIBLE, float/non-PCM formats, metadata chunks, CLI execution, differential tests, FATE, and fuzzing are pending.
- AVI demuxing is internal only and currently limited to constrained video-only RIFF AVI files with `avih`, `strh`, `strf`, and simple `movi` chunks. Audio streams, indexes, OpenDML, palette handling, non-BI_RGB video, interleaving semantics, seeking, probing, CLI execution, differential tests, FATE, and fuzzing are pending.
- AVI muxing is internal only and currently limited to one RGB24 stream-0 packet sequence with classic RIFF AVI headers and `00db` chunks. Audio streams, indexes, OpenDML, palettes, compressed video, interleaving, metadata, CLI execution, differential tests, FATE, and fuzzing are pending.
- MOV/MP4 demuxing is internal only and currently limited to box-bound validation, `ftyp`/movie/track/media metadata, and one populated track using simple `stsd`, `stts`, `ctts`, `stsc`, `stsz`, `stss`, and `stco`/`co64` tables to extract packet payloads from `mdat` with PTS/DTS separation and key flags. Multiple populated tracks, chunk groups beyond the initial mapping, edit lists, fragments, metadata atoms, codec parameters, probing, CLI execution, differential tests, FATE, and fuzzing are pending.
- yuv4mpegpipe demuxing and muxing are internal only and currently limited to progressive 4:2:0 `C420jpeg` packet extraction/writing. Other chroma modes, interlaced modes, frame header overrides, probing, CLI execution, differential tests, FATE, and fuzzing are pending.
- image2 demuxing and muxing are internal only and currently limited to caller-provided entries for a single image or contiguous `%d`/`%0Nd` numbered sequence. Filesystem discovery/writes, glob patterns, timestamp modes, looping, codec probing, CLI execution, differential tests, FATE, and fuzzing are pending.
- Rawvideo demuxing and muxing are internal only and currently limited to fixed-size packet slicing/writing for `gray`, `rgb24`, `rgba`, and `yuv420p`; CLI execution, probing, more pixel formats, differential tests, FATE, and fuzzing are pending.
- Raw PCM demuxing and muxing are internal only and currently limited to packed little-endian signed 16-bit samples; CLI execution, probing, other PCM sample formats, differential tests, FATE, and fuzzing are pending.
- Rawvideo decoding is internal only and supports a small initial pixel-format set; CLI demux/decode wiring is pending.
- PCM decoding is internal only and currently limited to packed little-endian signed 16-bit samples.
- The version banner is compatibility-oriented but not byte-identical to upstream FFmpeg.
- No inventory snapshot has been generated because no pinned FFmpeg oracle binary exists in this workspace yet.
- FATE components are listed from the ledger, but runnable FATE mappings have not been implemented.
