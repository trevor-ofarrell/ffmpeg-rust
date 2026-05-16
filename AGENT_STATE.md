# Agent State

## Current Status

`avformat-avi-muxer` is implemented and verified for a constrained one-stream RGB24 RIFF AVI path. It writes classic `avih`, `strh`, and `strf` headers, emits stream-0 `00db` chunks in a `movi` list, validates fixed-size packet payloads, handles RIFF word padding, and round-trips through the current AVI demuxer. The next priority component is `avformat-mov-demuxer`.

## Last Successful Commands

- `git init`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo test -p avformat pcm`
- `cargo fmt --all`
- `cargo test -p avformat pcm::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p avformat rawvideo`
- `cargo fmt --all`
- `cargo test -p avformat rawvideo`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p avformat image2`
- `cargo fmt --all`
- `cargo test -p avformat image2`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo fmt --all`
- `cargo test -p avformat yuv4mpegpipe`
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
- `cargo fmt --all`
- `cargo test -p avformat pcm::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat rawvideo`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat image2`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat avi`
- `cargo test --workspace --all-features`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avformat avi::tests`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat avi::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`

## Last Failing Commands

- `git status --short` before initialization failed because the directory was not a Git repository.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a test literal grouping in `byteio.rs`; the literal was normalized and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a loop-counter pattern and boolean assert style in `bitreader.rs`; both were corrected and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a redundant closure in `probe.rs`; it was replaced with the function reference and clippy passed on rerun.

## Current Focus Component

`avformat-mov-demuxer` is the next highest-priority incomplete component after the initial constrained AVI demuxer/muxer pair.

## Next 3 Concrete Actions

1. Add an initial ISOBMFF atom reader for box size/type parsing, extended sizes, and bounds validation.
2. Add a constrained MOV/MP4 demuxer metadata path for `ftyp`, `moov`, `mvhd`, `trak`, `tkhd`, and `mdhd` using small fixtures.
3. Record unsupported media/sample tables explicitly until packet extraction is implemented.

## Known Blockers

- No pinned FFmpeg 8.1.1 oracle binary exists at `third_party/ffmpeg-oracle/build/bin/ffmpeg`, so oracle snapshots and differential tests have not been generated.
- FATE samples and target mappings are not configured.
- `./xtask quick` cannot be a file command while `xtask/` is a crate directory on this filesystem; use `cargo run -p xtask -- quick`.

## Summary Of Latest Commit Or Changes

Latest slice: add `avformat-avi-muxer` for constrained one-stream RGB24 RIFF AVI writing with classic headers, `00db` movi output, packet validation, odd-size chunk padding, demuxer round-trip tests, and ledger/docs updates.
