# Fuzz Targets

Fuzzing is required for parsers, demuxers, decoders, filters, option parsing, and metadata parsing once those components exist.

Initial targets:

- `avutil_byteio`: exercises bounded byte reads, EOF cursor invariants, and byte writer helper paths.
- `avutil_bitreader`: exercises bit reads, peeks, skips, alignment, bit writer width validation, and cursor invariants.
- `avformat_wav`: exercises RIFF/WAVE PCM s16le demuxer opening, packet emission, and parsed stream invariants.
- `avformat_yuv4mpegpipe`: exercises YUV4MPEG2 demuxer opening, frame packet emission, and parsed stream invariants.

Run with cargo-fuzz when the tool is installed:

```sh
cargo fuzz run avutil_byteio
cargo fuzz run avutil_bitreader
cargo fuzz run avformat_wav
cargo fuzz run avformat_yuv4mpegpipe
```

The harness package lives under `fuzz/` and is intentionally separate from the main workspace.
