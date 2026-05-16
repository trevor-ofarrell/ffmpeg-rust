# Agent State

## Current Status

Initial Rust workspace slice is implemented and committed for the FFmpeg 8.1.1 compatibility rewrite. The repository started empty and was initialized as a Git repository during this turn.

## Last Successful Commands

- `git init`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo run -p oracle -- --help`
- `cargo run -p xtask -- quick`

## Last Failing Commands

- `git status --short` before initialization failed because the directory was not a Git repository.

## Current Focus Component

`avutil-byteio` is the next highest-priority incomplete component after this initial slice.

## Next 3 Concrete Actions

1. Run formatting, unit tests, CLI integration tests, and clippy for the workspace.
2. Fix any compile, test, or lint failures from the initial slice.
3. Implement `avutil-byteio` with endian-aware read/write tests.

## Known Blockers

- No pinned FFmpeg 8.1.1 oracle binary exists at `third_party/ffmpeg-oracle/build/bin/ffmpeg`, so oracle snapshots and differential tests have not been generated.
- FATE samples and target mappings are not configured.
- `./xtask quick` cannot be a file command while `xtask/` is a crate directory on this filesystem; use `cargo run -p xtask -- quick`.

## Summary Of Latest Commit Or Changes

Committed initial workspace slice: `Initialize FFmpeg rewrite workspace`.
