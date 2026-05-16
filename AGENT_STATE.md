# Agent State

## Current Status

`avformat-wav-muxer` is implemented and verified for a narrow RIFF/WAVE PCM s16le path. It writes canonical headers and stream-0 packet payloads, validates stream parameters, rejects split sample frames, and round-trips through the current WAV demuxer. The next priority component is `avformat-yuv4mpegpipe-demuxer`.

## Last Successful Commands

- `git init`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo run -p oracle -- --help`
- `cargo run -p xtask -- quick`
- `cargo fmt --all`
- `cargo test -p avutil byteio`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test --workspace --all-features`
- `cargo test -p avutil bitreader`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil bitwriter`
- `cargo test -p avutil dict`
- `cargo test -p avutil options`
- `cargo test -p avutil hash`
- `cargo test -p fftools option_parser`
- `cargo test -p fftools --test version hide_banner`
- `cargo test -p fftools io_plan`
- `cargo test -p avformat avio`
- `cargo test -p avformat probe`
- `cargo test -p avformat null_muxer`
- `cargo test -p avformat hash_muxer`
- `cargo test -p avformat framecrc_muxer`
- `cargo test -p avcodec rawvideo`
- `cargo test -p avcodec pcm`
- `cargo test -p avformat wav`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat wav`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`

## Last Failing Commands

- `git status --short` before initialization failed because the directory was not a Git repository.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a test literal grouping in `byteio.rs`; the literal was normalized and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a loop-counter pattern and boolean assert style in `bitreader.rs`; both were corrected and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a redundant closure in `probe.rs`; it was replaced with the function reference and clippy passed on rerun.

## Current Focus Component

`avformat-yuv4mpegpipe-demuxer` is the next highest-priority incomplete component after the initial WAV muxer.

## Next 3 Concrete Actions

1. Add an internal YUV4MPEG2 demuxer for a narrow yuv4mpegpipe profile.
2. Add unit tests for header parsing, frame extraction, invalid headers, and unsupported chroma/rate fields.
3. Update the ledger, docs, and state after the demuxer slice passes focused and workspace checks.

## Known Blockers

- No pinned FFmpeg 8.1.1 oracle binary exists at `third_party/ffmpeg-oracle/build/bin/ffmpeg`, so oracle snapshots and differential tests have not been generated.
- FATE samples and target mappings are not configured.
- `./xtask quick` cannot be a file command while `xtask/` is a crate directory on this filesystem; use `cargo run -p xtask -- quick`.

## Summary Of Latest Commit Or Changes

Latest slice: add `avformat-wav-muxer` for canonical RIFF/WAVE PCM s16le output with header, packet validation, round-trip, and finished-state tests.
