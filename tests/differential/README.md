# Differential Tests

Differential tests compare Rust outputs against the pinned FFmpeg 8.1.1 oracle. FFmpeg may be invoked only from these tests, oracle tooling, or inventory tooling.

The first rawvideo oracle harness lives in `crates/fftools/tests/rawvideo_oracle.rs` and is ignored by default because the pinned oracle binary is not checked in:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg \
  cargo test -p fftools --test rawvideo_oracle -- --ignored
```

On Windows PowerShell:

```powershell
$env:FFMPEG_ORACLE = ".\third_party\ffmpeg-oracle\build\bin\ffmpeg.exe"
cargo test -p fftools --test rawvideo_oracle -- --ignored
```

The current harness writes deterministic rawvideo inputs, runs Rust `ffmpeg-rs` to produce rawvideo file output, runs the pinned FFmpeg oracle with `-c:v copy -f rawvideo`, and compares the output bytes exactly.

`crates/avutil/tests/channel_layout_oracle.rs` is an ignored oracle harness for `ffmpeg -layouts`. It compares the oracle's individual-channel names/descriptions and standard-layout decompositions against `avutil::Channel::ALL` and `avutil::ChannelLayout::known_layouts()`:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg \
  cargo test -p avutil --test channel_layout_oracle -- --ignored
```

On Windows PowerShell:

```powershell
$env:FFMPEG_ORACLE = ".\third_party\ffmpeg-oracle\build\bin\ffmpeg.exe"
cargo test -p avutil --test channel_layout_oracle -- --ignored
```

The same oracle tests can be selected through the differential mapping file once a pinned oracle is available:

```sh
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg --component fftools-ffmpeg-rawvideo-file-output
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg --component avutil-channel-layout
```

On Windows PowerShell:

```powershell
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --oracle-ffmpeg .\third_party\ffmpeg-oracle\build\bin\ffmpeg.exe --component fftools-ffmpeg-rawvideo-file-output
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --oracle-ffmpeg .\third_party\ffmpeg-oracle\build\bin\ffmpeg.exe --component avutil-channel-layout
```

`tests/differential/mappings.txt` uses `env:FFMPEG_ORACLE={oracle_ffmpeg}` so `fate-runner` validates the oracle path and injects it into the ignored Rust integration test.

When `--oracle-ffmpeg` is omitted, `fate-runner` also checks `FFMPEG_ORACLE` and the standard `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)` paths. Invalid environment paths still fail before execution.
