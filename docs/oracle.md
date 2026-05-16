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

FATE samples are expected to be obtained using upstream FFmpeg's documented `make fate-rsync` flow against a local samples directory. This repository does not yet contain samples or a FATE target mapping.

## Differential Tests

Differential tests must compare Rust outputs to the pinned FFmpeg oracle. FFmpeg may be invoked from tests and oracle tooling only, never as runtime implementation.

