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
