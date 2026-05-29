# AGENTS.md - Rust FFmpeg Rewrite Goal Loop

## Mission

Build a complete Rust rewrite of FFmpeg pinned to FFmpeg 8.1.1 "Hoare".

The end state is a one-to-one compatible Rust implementation of the FFmpeg 8.1.1 command-line tools and libraries, including ffmpeg, ffprobe, practical ffplay behavior, libavutil, libavcodec, libavformat, libavfilter, libavdevice, libswresample, libswscale, and the supported codecs, formats, filters, protocols, devices, options, timestamps, side data, metadata, logging, and probe behavior.

## Active Milestone

The first milestone, 10% strict parity, has been reached. Continue the same strict parity-ledger loop toward 100% FFmpeg 8.1.1 default-native compatibility, then expand to later GPL/version3/nonfree/external-library/platform profiles.

For the current 96-row `PORTING_LEDGER.toml`, 10% strict parity meant at least 10 components marked `complete` under the normal completion definition below. The initial high-confidence infrastructure push covered or prioritized:
- `avutil-error`
- `avutil-rational`
- `avutil-timebase`
- `avutil-byteio`
- `avutil-bitreader`
- `avutil-bitwriter`
- `avutil-dict`
- `avutil-options`
- `avutil-logging`
- `avutil-hash`

Work should continue to prioritize completion evidence over optimistic surface area. That means installing or using a pinned FFmpeg 8.1.1 oracle where available, generating inventory snapshots, adding/running differential tests, adding/running FATE mappings or documenting why FATE is not applicable, running fuzz targets where relevant, closing known limitations for the selected component, and only then changing ledger status to `complete`.

Pinned upstream target:
- FFmpeg version: 8.1.1
- Release branch: release/8.1
- Build profile 1: default upstream build, no `--enable-gpl`, no `--enable-nonfree`, no optional external libraries
- Later profiles: GPL, version3, nonfree, external-library, platform/hardware, and historical compatibility profiles

## Orchestrator Workflow

The main Codex agent is the orchestrator and merge captain. Its job is to keep the parity ledger honest, choose the next strict evidence slice, delegate safe parallel work, review returned patches, run final gates, update persistent state, and commit coherent changes.

Parallel subagents are allowed for this goal. Use up to 10 concurrent subagents when useful, but prefer 3-5 active agents unless the work has clearly disjoint write sets and independent validation. More agents are only helpful when they reduce wall-clock time without increasing merge risk.

Use these roles:
- Explorers: read-only auditors for specific questions. They may inspect code and report blockers or task maps, but must not edit files.
- Workers: implementation agents with explicit ownership of a component and file set. They may edit only their assigned files and must report changed files, tests run, pass/fail, and remaining blockers.

Before spawning workers:
1. Inspect `git status --short`.
2. Finish, commit, or explicitly reserve any dirty files.
3. Define each worker's component id, exact owned files/directories, forbidden files, expected tests/oracle commands, and final report format.
4. Ensure no two active workers own the same source module, oracle harness, fuzz target, mapping file, docs file, ledger entry, or Cargo manifest.
5. Keep the immediate blocking task on the main thread if the next step depends on it.

Files reserved for the orchestrator unless explicitly delegated to exactly one worker at a time:
- `PORTING_LEDGER.toml`
- `AGENT_STATE.md`
- `AGENTS.md`
- `docs/architecture.md`
- `docs/compatibility.md`
- `docs/oracle.md`
- `tests/differential/mappings.txt`
- `tests/fate/*.txt`
- workspace manifests and lockfile
- crate root files such as `crates/*/src/lib.rs`

Workers must be told that other agents may be active in the codebase. They must not revert edits they did not make, must not broaden their write scope, and must not mark components complete. A worker may advance implementation and tests for a bounded slice, but only the orchestrator may change ledger status, update global docs, or commit.

Preferred parallel lanes after the current dirty slice is committed:
- `avutil-options`
- `avutil-logging`
- `avutil-packet`
- `avformat-wav-demuxer`
- `avformat-yuv4mpegpipe-demuxer`
- `avformat-image2-demuxer`
- `avformat-pcm-s16le-demuxer`
- `fftools-version`
- FATE/oracle mapping expansion
- isolated fuzz corpus and harness strengthening

Each worker should use a unique target directory such as `target-par-<component>` to avoid Cargo artifact contention. If separate git worktrees or branches are available, prefer one per worker. Merge worker output serially: review diffs, run narrow tests, run required oracle/FATE/fuzz gates, update ledger/state/docs centrally, then commit.

## Non-Negotiable Rules

1. Do not link against FFmpeg/libav* at runtime.
2. Do not use `ffmpeg-sys`, `libav*-sys`, `ffmpeg-next`, or any wrapper that calls FFmpeg as the implementation.
3. Do not shell out to FFmpeg except inside tests, oracle generation, or inventory tools.
4. Do not copy, transliterate, or mechanically port FFmpeg C functions line-by-line unless the project owner explicitly selects a derivative-license mode.
5. Do not mark a feature complete because a stub exists.
6. Do not make tests pass by weakening assertions, deleting assertions, hiding failures, or silently skipping unimplemented behavior.
7. Do not introduce `TODO: implement` code paths that return successful outputs.
8. All untrusted input parsers must reject invalid data with typed errors, not panics.
9. Unsafe Rust is allowed only for FFI boundaries, SIMD, hardware APIs, allocation/layout work, or documented performance-critical sections. Every unsafe block must include a safety comment and a test or invariant.
10. Every completed component must have tests in the parity ledger.

## Required Persistent Files

Maintain these files continuously:
- `AGENT_STATE.md`
- `PORTING_LEDGER.toml`
- `docs/architecture.md`
- `docs/compatibility.md`
- `docs/oracle.md`

## Required Workspace Shape

Use a Rust workspace with these crates unless a better structure is justified in `docs/architecture.md`:
- `crates/avutil`
- `crates/avcodec`
- `crates/avformat`
- `crates/avfilter`
- `crates/avdevice`
- `crates/swscale`
- `crates/swresample`
- `crates/fftools`
- `crates/fate-runner`
- `crates/oracle`
- `fuzz/`
- `tests/differential/`
- `tests/integration/`
- `tests/fate/`

## Build And Test Commands

Create and maintain these commands:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p oracle -- inventory --ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg --out compat/ffmpeg-8.1.1`
- `cargo run -p fate-runner -- list`
- `cargo run -p fate-runner -- run --component <component>`
- `cargo run -p fate-runner -- run --changed`
- `cargo fuzz run <target>` where available
- `cargo run -p xtask -- quick`
- `cargo run -p xtask -- changed`
- `cargo run -p xtask -- full`
- `cargo run -p xtask -- oracle-doctor`
- `cargo run -p xtask -- inventory --ffmpeg <path> --out compat/ffmpeg-8.1.1`

If a command cannot exist yet, implement the smallest useful version and record the limitation in `AGENT_STATE.md`.

## Test Hierarchy

Use this order:
1. Unit tests for small primitives.
2. Golden tests for parsers and option parsing.
3. Differential tests against pinned FFmpeg oracle.
4. FATE-derived tests.
5. Fuzz harnesses with seed corpora.
6. End-to-end CLI integration tests.
7. Performance benchmarks.

For media output comparison, prefer bit-exact comparison where valid; otherwise compare semantic packet, frame, metadata, timestamp, side-data, and checksum behavior according to FFmpeg/FATE expectations.

## Component Priority

Always choose the next task using this order unless `PORTING_LEDGER.toml` shows a more urgent blocker:
1. Infrastructure: errors, rational/timebase math, byte I/O, bit reader/writer, packet/frame model, metadata/dictionary, option parser, logging, hash/checksum helpers.
2. Minimal CLI and oracle tooling.
3. Simple formats and codecs: rawvideo, PCM, WAV, null muxer, hash/framehash/framecrc, image2 basics, yuv4mpegpipe.
4. Containers.
5. Audio codecs.
6. Video codecs.
7. Filters.
8. Hardware/platform features.

## Per-Turn Loop

At the start of every Codex turn:
1. Read `AGENTS.md`.
2. Read `AGENT_STATE.md`.
3. Read `PORTING_LEDGER.toml`.
4. Run `git status --short`.
5. Identify the highest-priority failing or incomplete component.
6. Make a concrete plan for this turn.
7. Decide what the main thread will do locally on the critical path.
8. When parallel work is useful, spawn explorers or workers with disjoint ownership and continue local non-overlapping work.
9. Implement or integrate one coherent slice.
10. Add or strengthen tests before marking progress.
11. Run the narrowest relevant tests.
12. Run formatting and clippy if the change is Rust code.
13. Review any worker results before integration; reject broad, stubby, untested, or conflicting changes.
14. Update `PORTING_LEDGER.toml`.
15. Update `AGENT_STATE.md`.
16. Commit if tests pass and the change is coherent.
17. End with a concise summary.

## Completion Definition For A Component

A component may be marked `complete` only when it is implemented without FFmpeg runtime linkage, has unit tests, has differential tests where applicable, has FATE coverage or a documented reason why no FATE coverage exists, has fuzz coverage if it parses untrusted bytes, has invalid-input tests, has no known limitations for the selected profile, passes formatting/tests, and lists exact test names and commands in the ledger.

## Blockers

If blocked by missing samples, unclear behavior, unavailable specs, or environment limits:
1. Create a failing or ignored test documenting the desired behavior where useful.
2. Record the blocker in `PORTING_LEDGER.toml`.
3. Record the blocker in `AGENT_STATE.md`.
4. Move to the next highest-priority unblocked component.
5. Never invent compatibility behavior without an oracle, spec, or test.
