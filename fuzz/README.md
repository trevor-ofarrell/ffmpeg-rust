# Fuzz Targets

Fuzzing is required for parsers, demuxers, decoders, filters, option parsing, and metadata parsing once those components exist.

Initial targets:

- `avutil_byteio`: exercises bounded byte reads, EOF cursor invariants, and byte writer helper paths.
- `avutil_bitreader`: exercises bit reads, peeks, skips, alignment, bit writer width validation, and cursor invariants.
- `avutil_metadata_options`: exercises metadata dictionary mutation, key/value validation, AVOption-like definition validation, string parsing, type/range checks, and failed-mutation invariants.
- `avformat_wav`: exercises RIFF/WAVE PCM s16le demuxer opening, packet emission, and parsed stream invariants.
- `avformat_yuv4mpegpipe`: exercises YUV4MPEG2 demuxer opening, frame packet emission, and parsed stream invariants.
- `avformat_pcm_s16le`: exercises raw PCM s16le demuxer parameter validation, packet slicing, timing, and side-data invariants.
- `avformat_rawvideo`: exercises rawvideo demuxer geometry/format/rate validation, frame slicing, timing, and side-data invariants.
- `avformat_avi`: exercises constrained RIFF AVI demuxer chunk parsing, stream metadata, packet timing, and side-data invariants.
- `avformat_mov`: exercises constrained MOV/MP4 box parsing, sample-table packet extraction, stream metadata, packet timing, and side-data invariants.
- `avformat_image2`: exercises image2 pattern parsing, entry sequence validation, packet timing/path side-data invariants, muxer path generation, and mux-demux round trips.
- `fftools_option_parser`: exercises FFmpeg-style option grouping, value handling, stream-specifier option names, and parse/render/parse stability.

Run with cargo-fuzz when the tool is installed:

```sh
cargo fuzz run avutil_byteio
cargo fuzz run avutil_bitreader
cargo fuzz run avutil_metadata_options
cargo fuzz run avformat_wav
cargo fuzz run avformat_yuv4mpegpipe
cargo fuzz run avformat_pcm_s16le
cargo fuzz run avformat_rawvideo
cargo fuzz run avformat_avi
cargo fuzz run avformat_mov
cargo fuzz run avformat_image2
cargo fuzz run fftools_option_parser
```

The harness package lives under `fuzz/` and is intentionally separate from the main workspace.
