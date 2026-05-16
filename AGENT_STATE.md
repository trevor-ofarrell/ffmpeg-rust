# Agent State

## Current Status

`avformat-mov-demuxer` now has an initial packet extraction path. It validates ISOBMFF/MOV/MP4 box bounds, parses `ftyp`, `moov/mvhd`, `trak/tkhd`, and `mdia/mdhd` metadata, parses simple `stsd`, `stts`, `ctts`, `stsc`, `stsz`, `stss`, and `stco`/`co64` sample tables for one populated track, handles multi-chunk `stsc` entry transitions and multiple `mdat` ranges, and emits packets with PTS/DTS/duration, composition-offset PTS, sync-sample key flags, and MOV side data.

## Last Successful Commands

- `git init`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
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
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`

## Last Failing Commands

- `git status --short` before initialization failed because the directory was not a Git repository.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a test literal grouping in `byteio.rs`; the literal was normalized and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a loop-counter pattern and boolean assert style in `bitreader.rs`; both were corrected and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a redundant closure in `probe.rs`; it was replaced with the function reference and clippy passed on rerun.
- `rg "read_u64|write_u64" crates/avutil/src crates/avformat/src` failed because ripgrep is not installed in this shell; PowerShell-native file inspection was used instead.
- `cargo test -p avformat mov` initially failed because the malformed `ftyp` fixture truncated the next top-level box before reaching the intended validation; the fixture was narrowed and `cargo test -p avformat mov::tests` passed.
- `cargo test -p avformat mov::tests` was initially blocked by Windows Application Control for the generated test executable; rerunning with the approved `cargo test` prefix passed.

## Current Focus Component

`avformat-mov-demuxer` remains the current focus; the next slice is fragmented/movie-fragment unsupported-layout handling.

## Next 3 Concrete Actions

1. Add tests for fragmented/movie-fragment boxes as unsupported rather than silently ignored where sample extraction would be misleading.
2. Add an explicit unsupported path for compressed or non-raw sample descriptions once codec parameters are represented.
3. Keep multi-track extraction blocked until it has tests for stream selection, timing, and packet interleaving.

## Known Blockers

- No pinned FFmpeg 8.1.1 oracle binary exists at `third_party/ffmpeg-oracle/build/bin/ffmpeg`, so oracle snapshots and differential tests have not been generated.
- FATE samples and target mappings are not configured.
- `./xtask quick` cannot be a file command while `xtask/` is a crate directory on this filesystem; use `cargo run -p xtask -- quick`.

## Summary Of Latest Commit Or Changes

Latest slice: strengthen `avformat-mov-demuxer` with explicit multiple-`mdat` fixture coverage so chunk offsets can point into separate top-level media-data boxes.
