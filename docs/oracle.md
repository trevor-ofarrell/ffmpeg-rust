# Oracle

## Pinned Upstream

The oracle is FFmpeg 8.1.1 "Hoare", built from the `release/8.1` branch with the initial default-native profile:

```sh
./configure --disable-gpl --disable-nonfree --disable-doc
make -j
```

The expected binary path for automated inventory is:

```text
./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

That binary is not checked into this repository.

## Inventory Generation

Run:

```sh
cargo run -p oracle -- inventory --ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg --out compat/ffmpeg-8.1.1
```

The inventory tool captures:

- `ffmpeg -version`
- `ffmpeg -buildconf`
- `ffmpeg -formats`
- `ffmpeg -codecs`
- `ffmpeg -decoders`
- `ffmpeg -encoders`
- `ffmpeg -muxers`
- `ffmpeg -demuxers`
- `ffmpeg -protocols`
- `ffmpeg -filters`
- `ffmpeg -bsfs`
- `ffmpeg -pix_fmts`
- `ffmpeg -sample_fmts`
- `ffmpeg -layouts`
- `ffmpeg -colors`

Each command is written as a text snapshot under `compat/ffmpeg-8.1.1/`, with `inventory.toml` recording command status.

## FATE Samples

FATE samples are expected to be obtained using upstream FFmpeg's documented `make fate-rsync` flow against a local samples directory. This repository does not yet contain samples or an upstream media FATE target mapping.

`cargo run -p fate-runner -- list` reads `PORTING_LEDGER.toml` and lists known components. `cargo run -p fate-runner -- mappings` reads `tests/fate/mappings.txt` and lists configured component-target commands; add `--check-prereqs` with `--samples <path>` and `--oracle-ffmpeg <path>` to resolve placeholders and validate all configured mapping prerequisites without executing commands. `cargo run -p fate-runner -- run --changed` inspects git changed paths, maps currently covered Rust modules and cargo-fuzz target files to ledger component IDs, and runs explicit command mappings from `tests/fate/mappings.txt` for selected components. Add `--dry-run` to resolve and print selected mappings, including prerequisite validation, without executing them. The mapping format is documented in `tests/fate/README.md` as `component_id|target|workdir|program|arg1|arg2|...`.

Mappings may reference `{samples}` and `{oracle_ffmpeg}` in the workdir, program, or args fields. When a selected mapping uses those placeholders, pass `--samples <path>` and/or `--oracle-ffmpeg <path>` to `fate-runner`; the runner validates that the samples path is an existing directory and the oracle path is an existing file before executing the mapped command.

The current default mapping file contains `fate-runner|local-self-test`, which validates local runner wiring by invoking `cargo test -p fate-runner`, and `avformat-mov-demuxer|local-mov-unit`, which validates that the selected MOV demuxer component can drive `cargo test -p avformat mov::tests` through the FATE runner. These mappings do not count as upstream FFmpeg FATE media parity.

## Differential Tests

Differential tests must compare Rust outputs to the pinned FFmpeg oracle. FFmpeg may be invoked from tests and oracle tooling only, never as runtime implementation.
