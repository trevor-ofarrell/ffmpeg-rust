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

`cargo run -p fate-runner -- list` reads `PORTING_LEDGER.toml` and lists known components. `cargo run -p fate-runner -- mappings` reads `tests/fate/mappings.txt` and lists configured component-target commands; add `--check-prereqs` with `--samples <path>` and `--oracle-ffmpeg <path>` to resolve placeholders and validate all configured mapping prerequisites without executing commands. `cargo run -p fate-runner -- run --changed` inspects git changed paths, maps currently covered Rust modules, dependency manifests and lockfile, and cargo-fuzz target files to ledger component IDs, and runs explicit command mappings from `tests/fate/mappings.txt` for selected components. Add `--dry-run` to resolve and print selected mappings, including prerequisite validation, without executing them. The mapping format is documented in `tests/fate/README.md` as `component_id|target|workdir|program|arg1|arg2|...`.

Mappings may reference `{samples}` and `{oracle_ffmpeg}` in the workdir, program, or args fields. When a selected mapping uses those placeholders, pass `--samples <path>` and/or `--oracle-ffmpeg <path>` to `fate-runner`; the runner validates that the samples path is an existing directory and the oracle path is an existing file before executing the mapped command.

The current default mapping file contains `fate-runner|local-self-test`, which validates local runner wiring by invoking `cargo test -p fate-runner`; local `avutil-*|local-avutil-unit` rows, which validate the selected shared primitive through focused `cargo test -p avutil` filters; local `avcodec-*|local-avcodec-unit` rows, which validate the selected initial decoder through focused `cargo test -p avcodec` filters; `avformat-mov-demuxer|local-mov-unit`, which validates that the selected MOV demuxer component can drive `cargo test -p avformat mov::tests` through the FATE runner; local `avformat-rawvideo-*|local-avformat-rawvideo-unit` rows, which validate the current rawvideo demuxer and muxer through focused `cargo test -p avformat rawvideo` filters; `fftools-version|local-version-unit`; `fftools-hide-banner|local-hide-banner-unit`; `fftools-option-parser|local-option-parser-unit`; `fftools-option-parser|local-cli-logging-unit`; `fftools-basic-io|local-io-plan-unit`; shared `local-ffmpeg-unit` rows for the current `fftools-ffmpeg-*` ledger components; and shared `local-ffprobe-unit` rows for the current `fftools-ffprobe-*` ledger components. These mappings do not count as upstream FFmpeg FATE media parity.

Local `avformat-*muxer|local-avformat-*-unit` rows validate the current WAV, raw PCM, rawvideo, yuv4mpegpipe, null, hash, framecrc, framehash, and streamhash unit filters when those muxer components or their shared fuzz target change.

## Differential Tests

Differential tests must compare Rust outputs to the pinned FFmpeg oracle. FFmpeg may be invoked from tests and oracle tooling only, never as runtime implementation.

## Source-Checked Notes

The `pal8` rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h`, `libavutil/pixdesc.c`, `libavutil/imgutils.c`, and `libavcodec/rawdec.c`. FFmpeg's descriptor marks `pal8` as paletted and alpha-bearing, defines 256 RGB32 palette entries for 1024 palette bytes, includes the palette in image buffer sizing, and lets ordinary rawvideo packets carry only the index plane while the decoder supplies palette state separately. The Rust model currently covers the raw packet index plane and constants only.

The 8-bit planar YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuva420p` as 20 bpp with log2 chroma `(1,1)`, `yuva422p` as 24 bpp with `(1,0)`, and `yuva444p` as 32 bpp with `(0,0)`, all with four 8-bit planes and full-resolution alpha.

The high-bit-depth planar YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuva420p9*` as 22.5 bpp (`45/2`) with log2 chroma `(1,1)`, `yuva422p9*` as 27 bpp with `(1,0)`, `yuva444p9*` as 36 bpp with `(0,0)`, `yuva420p10*` as 25 bpp with `(1,1)`, `yuva422p10*` as 30 bpp with `(1,0)`, `yuva444p10*` as 40 bpp with `(0,0)`, `yuva422p12*` as 36 bpp with `(1,0)`, `yuva444p12*` as 48 bpp with `(0,0)`, `yuva420p16*` as 40 bpp with `(1,1)`, `yuva422p16*` as 48 bpp with `(1,0)`, and `yuva444p16*` as 64 bpp with `(0,0)`, all using two stored bytes per component sample and a full-resolution alpha plane. FFmpeg 8.1.1 does not define `yuva420p12*`.

The 9-bit, 10-bit, 12-bit, 14-bit, and 16-bit planar YUV rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuv420p9*` as 13.5 bpp (`27/2`) with log2 chroma `(1,1)`, `yuv422p9*` as 18 bpp with `(1,0)`, `yuv444p9*` as 27 bpp with `(0,0)`, `yuv420p10*` as 15 bpp with `(1,1)`, `yuv422p10*` as 20 bpp with `(1,0)`, `yuv440p10*` as 20 bpp with `(0,1)`, `yuv444p10*` as 30 bpp with `(0,0)`, `yuv420p12*` as 18 bpp with `(1,1)`, `yuv422p12*` as 24 bpp with `(1,0)`, `yuv440p12*` as 24 bpp with `(0,1)`, `yuv444p12*` as 36 bpp with `(0,0)`, `yuv420p14*` as 21 bpp with `(1,1)`, `yuv422p14*` as 28 bpp with `(1,0)`, `yuv444p14*` as 42 bpp with `(0,0)`, `yuv420p16*` as 24 bpp with `(1,1)`, `yuv422p16*` as 32 bpp with `(1,0)`, and `yuv444p16*` as 48 bpp with `(0,0)`, all using two stored bytes per component sample.
