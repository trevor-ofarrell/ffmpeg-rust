# Fuzz Targets

Fuzzing is required for parsers, demuxers, decoders, filters, option parsing, and metadata parsing once those components exist.

Initial targets:

- `avutil_byteio`: exercises bounded byte reads, EOF cursor invariants, and byte writer helper paths.
- `avutil_bitreader`: exercises bit reads, peeks, skips, alignment, bit writer width validation, and cursor invariants.

Run with cargo-fuzz when the tool is installed:

```sh
cargo fuzz run avutil_byteio
cargo fuzz run avutil_bitreader
```

The harness package lives under `fuzz/` and is intentionally separate from the main workspace.
