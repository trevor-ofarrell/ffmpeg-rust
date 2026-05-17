# FATE Tests

`mappings.txt` is the default local mapping file consumed by `cargo run -p fate-runner -- run`.

Each non-comment row is pipe-separated:

```text
component_id|target|workdir|program|arg1|arg2|...
```

The current mapping file contains only local runner smoke coverage. These rows prove the FATE runner's mapping and command execution path; they are not a replacement for upstream FFmpeg FATE sample parity.
