# Fuzz Targets

Fuzzing is required for parsers, demuxers, decoders, filters, option parsing, and metadata parsing once those components exist.

Initial targets:

- `avcodec_basic_decoders`: exercises rawvideo and pcm_s16le decoder constructor validation, packet-size rejection, frame shape, payload preservation, and PTS propagation.
- `avutil_byteio`: exercises bounded byte reads, EOF cursor invariants, and byte writer helper paths.
- `avutil_bitreader`: exercises bit reads, peeks, skips, alignment, bit writer width validation, and cursor invariants.
- `avutil_metadata_options`: exercises metadata dictionary mutation, key/value validation, AVOption-like definition validation, string parsing, type/range checks, and failed-mutation invariants.
- `avutil_core_models`: exercises typed error constructor and IO classification invariants, rational arithmetic/timebase rounding and sentinel handling, pixel/sample/channel layout validation, packet timestamp/position/duration/flag/side-data invariants, frame shape and tightly packed line-size validation, and streaming checksum equivalence.
- `avformat_probe`: exercises probe descriptor validation, generated registry mutation, AVI/MOV descriptors, extension/MIME/signature scoring, deterministic tie behavior, and explainable matches.
- `avformat_wav`: exercises RIFF/WAVE PCM s16le demuxer opening, packet emission, and parsed stream invariants.
- `avformat_yuv4mpegpipe`: exercises YUV4MPEG2 demuxer opening, frame packet emission, and parsed stream invariants.
- `avformat_pcm_s16le`: exercises raw PCM s16le demuxer parameter validation, packet slicing, timing, and side-data invariants.
- `avformat_rawvideo`: exercises rawvideo demuxer geometry/format/rate validation, frame slicing, timing, and side-data invariants.
- `avformat_avi`: exercises constrained RIFF AVI demuxer chunk parsing, stream metadata, packet timing, and side-data invariants.
- `avformat_avi_muxer`: exercises constrained RGB24 AVI muxer constructor validation, packet validation, header/render stability, padding behavior, finish behavior, and demuxer round trips.
- `avformat_mov`: exercises constrained MOV/MP4 box parsing, sample-table packet extraction, stream metadata, packet timing, and side-data invariants.
- `avformat_image2`: exercises image2 pattern parsing, entry sequence validation, packet timing/path side-data invariants, muxer path generation, and mux-demux round trips.
- `avformat_basic_muxers`: exercises WAV, raw PCM s16le, rawvideo, and yuv4mpegpipe muxer packet validation, accounting, render/finish behavior, and demuxer round trips.
- `avformat_packet_muxers`: exercises null/hash/framecrc packet muxer accounting, hash digest stability, CRC record fields, timestamp propagation, and finish behavior.
- `fftools_option_parser`: exercises FFmpeg-style option grouping, value handling, valid loglevel values, stream-specifier option names, and parse/render/parse stability.

Run with cargo-fuzz when the tool is installed:

```sh
cargo fuzz run avcodec_basic_decoders
cargo fuzz run avutil_byteio
cargo fuzz run avutil_bitreader
cargo fuzz run avutil_metadata_options
cargo fuzz run avutil_core_models
cargo fuzz run avformat_probe
cargo fuzz run avformat_wav
cargo fuzz run avformat_yuv4mpegpipe
cargo fuzz run avformat_pcm_s16le
cargo fuzz run avformat_rawvideo
cargo fuzz run avformat_avi
cargo fuzz run avformat_avi_muxer
cargo fuzz run avformat_mov
cargo fuzz run avformat_image2
cargo fuzz run avformat_basic_muxers
cargo fuzz run avformat_packet_muxers
cargo fuzz run fftools_option_parser
```

The harness package lives under `fuzz/` and is intentionally separate from the main workspace.
