# FATE Tests

`mappings.txt` is the default local mapping file consumed by `cargo run -p fate-runner -- run`.

Each non-comment row is pipe-separated:

```text
component_id|target|workdir|program|arg1|arg2|...
```

Mappings may use `{samples}` and `{oracle_ffmpeg}` placeholders in the workdir, program, or args fields. Selected mappings that reference those placeholders require `--samples <path>` and/or `--oracle-ffmpeg <path>`, and the runner validates that the samples path is a directory and the oracle path is a file before executing the command.

The current mapping file contains only local runner smoke coverage. These rows prove the FATE runner's mapping and command execution path; they are not a replacement for upstream FFmpeg FATE sample parity.
