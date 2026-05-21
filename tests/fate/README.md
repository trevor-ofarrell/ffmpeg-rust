# FATE Tests

`mappings.txt` is the default local mapping file consumed by `cargo run -p fate-runner -- run`.
`upstream-mappings.txt` contains sample-backed oracle/FATE-style rows that must be selected explicitly with `--mappings tests/fate/upstream-mappings.txt`.

Each non-comment row is pipe-separated:

```text
component_id|target|workdir|program|arg1|arg2|...
```

Arguments with the form `env:NAME=value` are consumed by `fate-runner` as mapping-scoped environment variables and are not passed to the child process. Placeholder resolution applies to environment values, so `env:FFMPEG_ORACLE={oracle_ffmpeg}` can inject a validated oracle path for differential mappings.

Mappings may use `{samples}` and `{oracle_ffmpeg}` placeholders in the workdir, program, or args fields. Selected mappings that reference those placeholders accept `--samples <path>` and/or `--oracle-ffmpeg <path>`, and the runner validates that the samples path is a directory and the oracle path is a file before executing the command. If explicit flags are omitted, the runner tries `FATE_SAMPLES`, `SAMPLES`, and standard local sample directories (`third_party/fate-samples`, `third_party/fate-suite`, `fate-suite`) for `{samples}`, and `FFMPEG_ORACLE` plus `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)` for `{oracle_ffmpeg}`. Invalid explicit or environment paths fail instead of falling through silently.

List configured mappings without selecting a component:

```sh
cargo run -p fate-runner -- mappings
cargo run -p fate-runner -- mappings --target local-self-test
cargo run -p fate-runner -- mappings --check-prereqs --samples <path> --oracle-ffmpeg <path>
```

The first command reports configured commands without resolving placeholders. Repeated `--target <name>` filters listing to exact target names. The `--check-prereqs` command resolves placeholders and validates prerequisites for every listed mapping, but does not execute the mapped commands.

Use `--dry-run` to audit selected mappings without executing them:

```sh
cargo run -p fate-runner -- run --dry-run --component fate-runner
cargo run -p fate-runner -- run --dry-run --component fate-runner --target local-self-test
cargo run -p fate-runner -- run --dry-run --component avformat-rawvideo-demuxer --component avformat-rawvideo-muxer
cargo run -p fate-runner -- run --dry-run --changed
```

Dry-run mode still resolves placeholders and validates any required `--samples` or `--oracle-ffmpeg` paths for the selected mappings. Repeated `--component <id>` flags select multiple explicit components in one invocation; duplicate component IDs are deduplicated before mappings run. Repeated `--target <name>` flags narrow selected mappings to exact target names and do not silently pass unmatched components; a selected component with no matching filtered row fails as unmapped.

Run local smoke mappings:

```sh
cargo run -p fate-runner -- run --component avformat-mov-demuxer
cargo run -p fate-runner -- run --component avutil-buffer
cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer --component avformat-rawvideo-muxer
cargo run -p fate-runner -- run --component fftools-option-parser
cargo run -p fate-runner -- run --component fftools-basic-io
cargo run -p fate-runner -- run --component fftools-ffmpeg-mov-framecrc-null
cargo run -p fate-runner -- run --component fftools-ffprobe-mov-show-format
```

The current mapping file contains local runner smoke coverage, focused local `avutil` unit-test mappings, a local MOV demuxer unit-test smoke mapping, local `fftools` version, hide-banner, option-parser, and I/O-plan mappings, shared local `ffmpeg` unit-test mappings for current `fftools-ffmpeg-*` ledger components, and shared local `ffprobe` unit-test mappings for current `fftools-ffprobe-*` ledger components. These rows prove the FATE runner's mapping and command execution path for selected components; they are not a replacement for upstream FFmpeg FATE sample parity.

Run the first sample-backed mapping only when a pinned oracle and FATE samples tree are available:

```sh
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target fate-wav-pcm-s16le-md5
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avformat-wav-demuxer --target fate-wav-pcm-s16le-md5 --samples <fate-samples> --oracle-ffmpeg <ffmpeg>
```

The oracle-only generated WAV MD5 check lives in `tests/differential/mappings.txt` as `avformat-wav-demuxer|oracle-wav-generated-md5`; use that row when a pinned oracle exists but FATE samples are not installed.
