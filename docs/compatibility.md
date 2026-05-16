# Compatibility

## Target

- FFmpeg version: 8.1.1 "Hoare"
- Release branch: `release/8.1`
- Initial profile: `ffmpeg-8.1.1-default-native`
- Oracle configure baseline: `./configure --disable-gpl --disable-nonfree --disable-doc`

## Compatible Today

- `ffmpeg-rs -version` and `ffmpeg-rs -hide_banner -version` print a version banner naming the pinned target and ABI versions.
- `ffmpeg-rs` can execute one local seekable MOV/MP4, PCM s16le RIFF/WAVE, raw `pcm_s16le`, explicit `rawvideo`, yuv4mpegpipe, or explicit image2 single-file/numbered-sequence input to stdout with explicit `-f null -` or `-f framecrc -`, using Rust demuxers plus null/framecrc muxers.
- `ffmpeg-rs` can packet-copy explicit raw `pcm_s16le` input to a local raw `-f s16le` output file through the Rust PCM demuxer and muxer.
- `ffmpeg-rs` can packet-copy explicit raw `pcm_s16le` input to a local RIFF/WAVE `-f wav` output file through the Rust PCM demuxer and WAV muxer.
- `ffmpeg-rs` can packet-copy explicit rawvideo input to a local raw `-f rawvideo` output file through the Rust rawvideo demuxer and muxer.
- `ffmpeg-rs` can packet-copy explicit raw `yuv420p` input to a local `-f yuv4mpegpipe` output file through the Rust rawvideo demuxer and YUV4MPEG2 muxer.
- `ffmpeg-rs` can packet-copy explicit image2 single-file or contiguous numbered-sequence input to local `-f image2` file or numbered-pattern outputs through the Rust image2 demuxer and muxer.
- `ffmpeg-rs` supports file-scoped image2 `-start_number` for constrained numbered-sequence input discovery and numbered output filename generation.
- `ffprobe-rs -version` and `ffprobe-rs -hide_banner -version` print a version banner naming the pinned target and ABI versions.
- `ffprobe-rs -show_format`, `ffprobe-rs -show_streams`, and `ffprobe-rs -show_packets` can open local seekable MOV/MP4 files through the Rust MOV probe descriptor and demuxer, then render small default or JSON summaries.
- `fftools` has an initial option parser for a small known FFmpeg option set, preserving option ordering for input/output grouping.
- `fftools` has an initial I/O planning layer that validates input/output presence and classifies file, pipe, and protocol endpoints.
- `avformat` has initial AVIO-like read/write wrappers for seekable byte streams with typed EOF/external errors.
- `avformat` has an initial probe registry for prefix or offset signature, MIME type, and extension scoring, plus a MOV/MP4 probe descriptor for `ftyp`, common filename extensions, and common MIME types.
- `avformat` has an initial null muxer sink that discards packets while reporting packet, byte, duration, and timestamp statistics.
- `avformat` has an initial packet-data hash muxer sink for Adler-32 and IEEE CRC-32.
- `avformat` has an initial framecrc-style packet checksum sink with one CRC-32 record per packet.
- `avformat` has an initial RIFF/WAVE PCM s16le demuxer that validates `fmt ` and `data` chunks and emits the data chunk as one packet.
- `avformat` has an initial RIFF/WAVE PCM s16le muxer that writes canonical headers and stream-0 packet payloads.
- `avformat` has an initial constrained RIFF AVI demuxer for video-only `avih`/`strh`/`strf` files with simple `movi` packet extraction.
- `avformat` has an initial constrained RIFF AVI muxer for one RGB24 video stream with classic headers and `00db` `movi` packet output.
- `avformat` has an initial MOV/MP4 demuxer path for metadata boxes, common movie-level and track-level `udta/meta/ilst` metadata value extraction from `data` atoms for UTF-8 and UTF-16 text fields, classic `gnre` genre indexes, one-byte integer/boolean atoms, iTunes-style freeform `----` atoms, `covr` cover-art payloads, and track/disc pairs, generic sample-entry codec parameter records, VisualSampleEntry field parsing and child-box validation for known video sample entries including structured `avcC` version/profile/level/NAL-length-size/SPS/PPS parsing, structured `hvcC` profile/timing/NAL-length-size/NAL-array parsing, `pasp` pixel aspect ratio, `nclx`/`nclc` `colr` color parameters, and `rICC`/`prof` `colr` ICC profile payloads, explicit fragmented-layout, edit-list, multiple-`stsd`-entry, and multiple-populated-track rejection, simple one-track sample table packet extraction, multi-chunk sample-to-chunk mapping, multiple `mdat` ranges, composition-offset PTS, and sync-sample key flags.
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

- CLI execution support is limited to `-version`, one local MOV/MP4, PCM s16le RIFF/WAVE, raw `pcm_s16le`, explicit `rawvideo`, yuv4mpegpipe, or explicit image2 single-file/numbered-sequence `ffmpeg-rs` input to stdout `-f null -` or `-f framecrc -`, explicit raw `pcm_s16le` packet-copy to local `-f s16le` and `-f wav` files, explicit rawvideo packet-copy to local `-f rawvideo` and yuv420p-only `-f yuv4mpegpipe` files, explicit image2 packet-copy to local `-f image2` file or numbered-pattern outputs with image2 `-start_number`, and a small `ffprobe-rs -show_format`/`-show_streams`/`-show_packets` path for local MOV/MP4 files; most FFmpeg/ffprobe options return unsupported-command errors.
- The ffprobe MOV output is a deterministic compatibility-oriented summary, not byte-identical upstream output, and it has no pinned-oracle differential coverage yet.
- The option parser intentionally rejects unknown options and only covers a small initial compatibility set.
- The I/O planner does not execute demuxers, muxers, protocols, or media transforms yet.
- AVIO support is limited to seekable Rust `Read`/`Write` objects and is not yet wired to protocol implementations.
- Probe support is still an early registry/scoring primitive with a hand-written MOV/MP4 descriptor; no full demuxer probe table has been generated from the pinned oracle yet.
- Null muxer support is wired only to the constrained MOV/MP4, WAV, raw PCM, rawvideo, yuv4mpegpipe, and image2 single-file/numbered-sequence `ffmpeg-rs -f null -` execution paths; it does not have broad CLI, differential, or FATE coverage yet.
- Hash muxer support is internal and limited to Adler-32/CRC-32 packet-data hashing; it is not wired to CLI execution, MD5/SHA variants, or FATE yet.
- Framecrc muxer support is wired only to the constrained MOV/MP4, WAV, raw PCM, rawvideo, yuv4mpegpipe, and image2 single-file/numbered-sequence `ffmpeg-rs -f framecrc -` execution paths and is not byte-identical to FFmpeg framecrc output yet.
- WAV demuxing is currently limited to RIFF/WAVE PCM s16le with one packet for the data chunk and constrained `ffmpeg-rs -f null -`/`-f framecrc -` CLI output. RF64, WAVE64, WAVE_FORMAT_EXTENSIBLE, float/non-PCM formats, packet chunking, probing, broad CLI execution, differential tests, FATE, and fuzzing are pending.
- WAV muxing is currently limited to canonical RIFF/WAVE PCM s16le stream-0 payloads and constrained explicit raw `pcm_s16le` input to local `-f wav` file execution. RF64, WAVE64, WAVE_FORMAT_EXTENSIBLE, float/non-PCM formats, metadata chunks, WAV output from other inputs, broad CLI execution, differential tests, FATE, and fuzzing are pending.
- AVI demuxing is internal only and currently limited to constrained video-only RIFF AVI files with `avih`, `strh`, `strf`, and simple `movi` chunks. Audio streams, indexes, OpenDML, palette handling, non-BI_RGB video, interleaving semantics, seeking, probing, CLI execution, differential tests, FATE, and fuzzing are pending.
- AVI muxing is internal only and currently limited to one RGB24 stream-0 packet sequence with classic RIFF AVI headers and `00db` chunks. Audio streams, indexes, OpenDML, palettes, compressed video, interleaving, metadata, CLI execution, differential tests, FATE, and fuzzing are pending.
- MOV/MP4 demuxing is currently limited to box-bound validation, `ftyp`/movie/track/media metadata, common movie-level and track-level `udta/meta/ilst` metadata extraction for UTF-8 and UTF-16 text fields, classic `gnre` genre indexes, one-byte integer/boolean atoms, iTunes-style freeform `----` atoms, `covr` cover-art payloads, and track/disc number pairs, a hand-written `ftyp`/extension/MIME probe descriptor, generic `stsd` sample-entry records, VisualSampleEntry field parsing and child-box validation for known video sample entries with structured `avcC` version/profile/level/NAL-length-size/SPS/PPS parsing, structured `hvcC` profile/timing/NAL-length-size/NAL-array parsing, parsed `pasp` pixel aspect ratio, parsed `nclx`/`nclc` `colr` color parameters, and parsed `rICC`/`prof` ICC profile payloads, explicit `mvex`/`moof`, `edts`, multiple `stsd` sample entries, and multiple-populated-track unsupported errors, one populated track using simple `stsd`, `stts`, `ctts`, `stsc`, `stsz`, `stss`, and `stco`/`co64` tables to extract packet payloads from one or more `mdat` boxes with PTS/DTS separation and key flags, initial ffprobe `-show_format`/`-show_streams`/`-show_packets` CLI summary wiring, and initial ffmpeg `-f null -`/`-f framecrc -` CLI wiring. Packet extraction for multiple populated tracks, sample description indexes other than 1, edit-list timeline application, actual fragment parsing, deeper VPS/SPS/PPS bitstream validation, broader CLI execution, differential tests, FATE, and fuzzing are pending.
- yuv4mpegpipe demuxing and muxing are currently limited to progressive 4:2:0 `C420jpeg` packet extraction/writing; constrained `ffmpeg-rs -f yuv4mpegpipe -i <file> -f null/framecrc -` CLI execution, YUV4MPEG2/.y4m detection, and raw `yuv420p` to local `-f yuv4mpegpipe` file execution are implemented. Other chroma modes, interlaced modes, frame header overrides, broad CLI execution, differential tests, FATE, and fuzzing are pending.
- image2 demuxing and muxing are currently limited to a single image or contiguous `%d`/`%0Nd` numbered sequence; constrained explicit `ffmpeg-rs -f image2 -framerate <rate> [-start_number <n>] -i <file-or-pattern> -f null/framecrc -` CLI execution is implemented for single files and parent-directory sequence discovery, and constrained image2 input to local `-f image2 [-start_number <n>]` filesystem output is implemented with no-overwrite semantics. Glob patterns, timestamp modes, looping, codec probing, broad CLI execution, differential tests, FATE, and fuzzing are pending.
- Rawvideo demuxing and muxing are currently limited to fixed-size packet slicing/writing for `gray`, `rgb24`, `rgba`, and `yuv420p`; constrained explicit `ffmpeg-rs -f rawvideo -pix_fmt <fmt> -s <WxH> -r <rate> -i <file> -f null/framecrc -` stdout execution, rawvideo to local `-f rawvideo` file execution, and yuv420p-only local `-f yuv4mpegpipe` file execution are implemented. The raw file-output paths create new files only and do not implement overwrite confirmation flags. Probing, more pixel formats, broad CLI execution, differential tests, FATE, and fuzzing are pending.
- Raw PCM demuxing and muxing are currently limited to packed little-endian signed 16-bit samples, with constrained raw `pcm_s16le` `ffmpeg-rs -f null -`/`-f framecrc -` stdout execution and raw `pcm_s16le` to local `-f s16le` or `-f wav` file execution requiring explicit `-ar` and `-ac`. The raw file-output paths create new files only and do not implement overwrite confirmation flags. Probing, other PCM sample formats, broad CLI execution, differential tests, FATE, and fuzzing are pending.
- Rawvideo decoding is internal only and supports a small initial pixel-format set; CLI demux/decode wiring is pending.
- PCM decoding is internal only and currently limited to packed little-endian signed 16-bit samples.
- The version banner is compatibility-oriented but not byte-identical to upstream FFmpeg.
- No inventory snapshot has been generated because no pinned FFmpeg oracle binary exists in this workspace yet.
- FATE components are listed from the ledger, but runnable FATE mappings have not been implemented.
