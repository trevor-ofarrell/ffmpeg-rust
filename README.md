# ffmpegrust

`ffmpegrust` is an experimental Rust rewrite of FFmpeg, pinned to FFmpeg 8.1.1 "Hoare" for compatibility work.

The project is not a wrapper around FFmpeg. FFmpeg is used only as an oracle for inventory, tests, documentation, and behavior comparison. Runtime implementation code must not link against `libav*`, use `ffmpeg-sys` or similar wrappers, or shell out to FFmpeg.

## Current Status

This repository is in the early infrastructure and constrained-media phase.

The current code has meaningful local coverage for shared `avutil` primitives, including typed errors, rational and timebase helpers, byte and bit I/O, buffers, packets, frames, dictionaries, options, logging, hashing, pixel formats, sample formats, colors, and channel layouts. It also has initial Rust-native `avformat`, `avcodec`, and `fftools` support for constrained paths such as rawvideo, PCM s16le, WAV, yuv4mpegpipe, image2, AVI, MOV/MP4 probing/demuxing, null/hash/framecrc-style outputs, and `ffmpeg-rs` / `ffprobe-rs` version and selected inspection behavior.

Strict parity status is still low:

- Ledger rows currently tracked: 96
- Rows marked `implemented`: 45
- Rows marked `scaffolded`: 1
- Rows marked `complete`: 11
- Rows marked `differential_pass`, `fate_pass`, or `fuzzed`: 39
- Pinned FFmpeg oracle: installed locally through WSL wrappers under ignored `third_party/ffmpeg-oracle/`
- Generated FFmpeg inventory snapshot: present locally under ignored `compat/ffmpeg-8.1.1/`
- Upstream FATE sample execution: limited; one WAV sample is installed locally, but the full sample tree is still absent
- Actual local `cargo-fuzz` execution: available through WSL; avutil smoke targets have run locally

Estimated completion:

- Strict parity completion: about 11% (`11/96`)
- Practical engineering-progress estimate: about 2% of a complete FFmpeg 8.1.1 default-native rewrite

The practical estimate is intentionally conservative. A lot of reusable foundation exists, but the full target includes the FFmpeg command-line tools, all core libraries, codecs, demuxers, muxers, protocols, filters, devices, scaling, resampling, hardware profiles, FATE parity, differential oracle coverage, and fuzz coverage. Most of that surface is not implemented yet.

## Current Operating Milestone

The 10% strict-parity milestone has been reached. The active goal now broadens back out toward 100% FFmpeg 8.1.1 default-native compatibility while keeping the same strict ledger rules.

For the current 96-row ledger, the first milestone meant at least 10 components marked `complete`; that threshold is now satisfied by low-level infrastructure and oracle tooling. The next completion push should keep prioritizing high-leverage infrastructure and CLI/oracle surfaces such as `avutil-options`, `avutil-logging`, `fftools-version`, `avutil-channel-layout`, and simple muxer/hash paths, but only when strict completion evidence exists.

The work remains mostly proof work before broad new surface area: keep the pinned FFmpeg 8.1.1 oracle available, generate or refresh inventory snapshots when needed, run or add differential tests, run or justify FATE coverage, run fuzz targets where relevant, close known limitations for each selected component, and update `PORTING_LEDGER.toml` only when the project completion definition is actually satisfied.

The pinned FFmpeg oracle should remain installed locally for this project. It is intentionally ignored by git because it contains upstream build artifacts, including generated `.d` dependency files, object files, libraries, and binaries.

## Repository Layout

- `crates/avutil`: shared utility types and low-level media primitives.
- `crates/avcodec`: codec registry, decoder traits, parser traits, and initial native decoder work.
- `crates/avformat`: AVIO-like I/O, probe logic, demuxers, muxers, streams, and metadata.
- `crates/avfilter`: filter graph and filter compatibility surface.
- `crates/avdevice`: platform device compatibility surface.
- `crates/swscale`: scaling and pixel conversion compatibility surface.
- `crates/swresample`: resampling and sample/channel conversion compatibility surface.
- `crates/fftools`: `ffmpeg-rs` and `ffprobe-rs` CLI entry points.
- `crates/oracle`: pinned FFmpeg inventory capture tooling.
- `crates/fate-runner`: local and future upstream FATE mapping runner.
- `fuzz`: cargo-fuzz target package.
- `tests/differential`: mappings for pinned FFmpeg oracle comparisons.
- `tests/fate`: local smoke mappings and sample-backed FATE mapping definitions.
- `xtask`: project command runner and runtime-wrapper guard.

## Useful Commands

Run the local test suite:

```sh
cargo test --workspace --all-features
```

Run formatting and lint checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Run the project guard and quick command set:

```sh
cargo run -p xtask -- quick
```

Verify the local pinned FFmpeg oracle install:

```sh
cargo run -p xtask -- oracle-doctor
```

Run the Rust FFmpeg-like version commands:

```sh
cargo run -p fftools --bin ffmpeg-rs -- -version
cargo run -p fftools --bin ffprobe-rs -- -version
```

List or run mapped local parity smoke tests:

```sh
cargo run -p fate-runner -- list
cargo run -p fate-runner -- mappings
cargo run -p fate-runner -- run --changed
```

Generate a pinned FFmpeg inventory once an oracle binary exists:

```sh
cargo run -p oracle -- inventory --ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg --out compat/ffmpeg-8.1.1
```

## Oracle And FATE

The intended oracle is FFmpeg 8.1.1 built from `release/8.1` with the default-native profile:

```sh
./configure --disable-gpl --disable-nonfree --disable-doc
make -j
```

On this Windows workspace, the repeatable no-sudo bootstrap path is WSL:

```sh
wsl -d Ubuntu --exec bash -lc "cd /mnt/c/Users/trevo/code/ffmpegrust && ./scripts/bootstrap_ffmpeg_oracle_wsl.sh"
```

That script builds the pinned Linux oracle under ignored `third_party/ffmpeg-oracle/wsl/` and generates wrappers under:

```text
./third_party/ffmpeg-oracle/build/bin/ffmpeg
./third_party/ffmpeg-oracle/build/bin/ffmpeg.cmd
```

Windows Rust oracle tests prefer `ffmpeg.exe`, then `ffmpeg.cmd`, then the Unix-style `ffmpeg` path. WSL/Linux runs prefer the Unix-style wrapper. The `third_party/` tree is ignored and should not be committed.

`cargo run -p xtask -- oracle-doctor` checks the default `ffmpeg` and `ffprobe` oracle paths, runs `-version`, and rejects a build that does not report FFmpeg 8.1.1 with the pinned libav* ABI versions. Use `--ffmpeg <path>` and `--ffprobe <path>` to validate non-default oracle paths.

FATE samples are expected under a local samples tree obtained through upstream FFmpeg's documented `make fate-rsync` flow. This repository does not check in oracle binaries or media samples.

## Progress Tracking

This project is run as a parity ledger loop. The main persistent files are:

- `AGENT_STATE.md`: latest work state, commands, blockers, and next actions.
- `PORTING_LEDGER.toml`: component-by-component implementation and test ledger.
- `docs/architecture.md`: current architecture and type model.
- `docs/compatibility.md`: what works, what is partial, and what is not yet compatible.
- `docs/oracle.md`: oracle, inventory, differential, and FATE workflow notes.

No stub counts as progress. A component should only be marked complete after implementation, unit tests, applicable differential tests, FATE coverage or a documented exception, fuzz coverage where relevant, invalid-input coverage, and passing validation commands.

## License Notes

FFmpeg is used as a behavioral oracle and documentation/source-reference target. This project should avoid direct line-by-line translation of FFmpeg C code unless the project owner intentionally accepts derivative-license implications. This README is not legal advice.
