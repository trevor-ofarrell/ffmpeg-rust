# FATE Tests

`mappings.txt` is the default local mapping file consumed by `cargo run -p fate-runner -- run`.

Each non-comment row is pipe-separated:

```text
component_id|target|workdir|program|arg1|arg2|...
```

Mappings may use `{samples}` and `{oracle_ffmpeg}` placeholders in the workdir, program, or args fields. Selected mappings that reference those placeholders require `--samples <path>` and/or `--oracle-ffmpeg <path>`, and the runner validates that the samples path is a directory and the oracle path is a file before executing the command.

List configured mappings without selecting a component:

```sh
cargo run -p fate-runner -- mappings
cargo run -p fate-runner -- mappings --check-prereqs --samples <path> --oracle-ffmpeg <path>
```

The first command reports configured commands without resolving placeholders. The second command resolves placeholders and validates prerequisites for every configured mapping, but does not execute the mapped commands.

Use `--dry-run` to audit selected mappings without executing them:

```sh
cargo run -p fate-runner -- run --dry-run --component fate-runner
cargo run -p fate-runner -- run --dry-run --changed
```

Dry-run mode still resolves placeholders and validates any required `--samples` or `--oracle-ffmpeg` paths for the selected mappings.

Run the local MOV demuxer unit-test smoke mapping:

```sh
cargo run -p fate-runner -- run --component avformat-mov-demuxer
```

The current mapping file contains local runner smoke coverage and a local MOV demuxer unit-test smoke mapping. These rows prove the FATE runner's mapping and command execution path for selected components; they are not a replacement for upstream FFmpeg FATE sample parity.
