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

`cargo run -p fate-runner -- list` reads `PORTING_LEDGER.toml` and lists known components. `cargo run -p fate-runner -- mappings` reads `tests/fate/mappings.txt` and lists configured component-target commands; add `--check-prereqs` with `--samples <path>` and `--oracle-ffmpeg <path>` to resolve placeholders and validate all configured mapping prerequisites without executing commands. `cargo run -p fate-runner -- run --changed` inspects git changed paths, maps currently covered Rust modules, dependency manifests and lockfile, cargo-fuzz target files, and the rawvideo oracle integration harness to ledger component IDs, and runs explicit command mappings from `tests/fate/mappings.txt` for selected components. Explicit runs may pass `--component <id>` more than once to select multiple components in one invocation; duplicate component IDs are coalesced before execution. Add `--dry-run` to resolve and print selected mappings, including prerequisite validation, without executing them. The mapping format is documented in `tests/fate/README.md` as `component_id|target|workdir|program|arg1|arg2|...`.

Mappings may reference `{samples}` and `{oracle_ffmpeg}` in the workdir, program, or args fields. When a selected mapping uses those placeholders, pass `--samples <path>` and/or `--oracle-ffmpeg <path>` to `fate-runner`; the runner validates that the samples path is an existing directory and the oracle path is an existing file before executing the mapped command.

Mapping args may also include `env:NAME=value`; `fate-runner` treats those as child-process environment variables rather than command arguments. Placeholder resolution applies to the environment value, so differential mappings can use `env:FFMPEG_ORACLE={oracle_ffmpeg}` to validate and inject the pinned oracle path for ignored Rust oracle tests.

The current default mapping file contains `fate-runner|local-self-test`, which validates local runner wiring by invoking `cargo test -p fate-runner`; local `avutil-*|local-avutil-unit` rows, which validate the selected shared primitive through focused `cargo test -p avutil` filters; local `avcodec-*|local-avcodec-unit` rows, which validate the selected initial decoder through focused `cargo test -p avcodec` filters; `avformat-mov-demuxer|local-mov-unit`, which validates that the selected MOV demuxer component can drive `cargo test -p avformat mov::tests` through the FATE runner; local `avformat-rawvideo-*|local-avformat-rawvideo-unit` rows, which validate the current rawvideo demuxer and muxer through focused `cargo test -p avformat rawvideo` filters; `fftools-version|local-version-unit`; `fftools-hide-banner|local-hide-banner-unit`; `fftools-option-parser|local-option-parser-unit`; `fftools-option-parser|local-cli-logging-unit`; `fftools-basic-io|local-io-plan-unit`; shared `local-ffmpeg-unit` rows for the current `fftools-ffmpeg-*` ledger components; and shared `local-ffprobe-unit` rows for the current `fftools-ffprobe-*` ledger components. These mappings do not count as upstream FFmpeg FATE media parity.

Local `avformat-*muxer|local-avformat-*-unit` rows validate the current WAV, raw PCM, rawvideo, yuv4mpegpipe, null, hash, framecrc, framehash, and streamhash unit filters when those muxer components or their shared fuzz target change.

## Differential Tests

Differential tests must compare Rust outputs to the pinned FFmpeg oracle. FFmpeg may be invoked from tests and oracle tooling only, never as runtime implementation.

The first rawvideo oracle harness lives at `crates/fftools/tests/rawvideo_oracle.rs` and is ignored by default so ordinary local test runs do not claim oracle parity without an oracle binary. Run it with a pinned FFmpeg 8.1.1 binary:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test rawvideo_oracle -- --ignored
```

On Windows PowerShell:

```powershell
$env:FFMPEG_ORACLE = ".\third_party\ffmpeg-oracle\build\bin\ffmpeg.exe"
cargo test -p fftools --test rawvideo_oracle -- --ignored
```

The harness currently compares Rust constrained `-f rawvideo ... -f rawvideo <file>` output bytes against pinned FFmpeg `-c:v copy -f rawvideo` output for `rgb24` and `gbrp10msble`. If `FFMPEG_ORACLE` is unset and `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)` is absent, the ignored tests fail before comparison instead of silently passing.

The same harness is wired into `tests/differential/mappings.txt`, which can be executed through `fate-runner` with `--mappings tests/differential/mappings.txt --oracle-ffmpeg <path> --component fftools-ffmpeg-rawvideo-file-output` once the pinned oracle binary exists.

## Source-Checked Notes

The `avutil-error` string slice was checked against FFmpeg 8.1.1 `libavutil/error.h` and `libavutil/error.c`. The pinned header defines the tag-based `AVERROR_*` constants and `AV_ERROR_MAX_STRING_SIZE`; the pinned C implementation maps the `AVERROR_LIST` entries to user-facing strings and falls back to `Error number <n> occurred` when no description is found. The Rust model covers the FFmpeg-defined table but still leaves platform `AVERROR(errno)` string parity to a later platform profile.

The `avutil-rational` helper slices were checked against FFmpeg 8.1.1 `libavutil/rational.h` and `libavutil/rational.c`. The pinned header documents `av_cmp_q`, `av_nearer_q`, `av_find_nearest_q_idx`, `av_q2intfloat`, and `av_gcd_q`; the pinned implementation treats positive and negative zero-denominator rationals as infinities for comparison, treats `0/0` as indeterminate, scans a `{0,0}`-terminated nearest list while preserving the first candidate on ties, converts rationals to platform-independent IEEE single-precision bit patterns with `av_q2intfloat`, and returns `av_gcd_q` results as raw `gcd(num)/lcm(den)` rationals only when `lcm < max_den`. The Rust model uses explicit slices instead of C sentinels, rejects zero-denominator nearest candidates with typed errors, models `av_q2intfloat` special values including FFmpeg's source-shaped zero-denominator result, rejects raw `i32::MIN` negation cases that would be signed-overflow in C, and currently models `av_gcd_q` for finite positive-denominator inputs.

The `avutil-timebase` helper slices were checked against FFmpeg 8.1.1 `libavutil/mathematics.h` and `libavutil/mathematics.c`. The pinned source defines `AV_ROUND_*`, `AV_ROUND_PASS_MINMAX`, `av_rescale*`, `av_compare_ts`, `av_compare_mod`, `av_rescale_delta`, and `av_add_stable`; the current Rust model covers checked rescale helpers, `av_compare_ts`, `av_compare_mod`, `av_rescale_delta`-style stateful duration-preserving timestamp conversion for nonnegative FFmpeg-int durations, and `av_add_stable`-style timestamp increments with exact positive tick addition, exact negative tick subtraction, sub-tick/fractional-negative no-op behavior, and positive fractional no-drift updates. Rust returns typed errors for invalid inputs and out-of-range exact stable-add results where the C path relies on bounded timestamp assumptions or source-level undefined behavior; pinned oracle vectors are still needed before claiming differential parity for those edges.

The `avutil-channel-layout` slice was checked against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c`. The pinned header defines `AV_CHAN_FRONT_LEFT_OF_CENTER`, `AV_CHAN_FRONT_RIGHT_OF_CENTER`, and `AV_CHAN_BACK_CENTER` after `AV_CHAN_BACK_RIGHT`, plus height-channel enum entries including `AV_CHAN_TOP_FRONT_LEFT` and `AV_CHAN_TOP_FRONT_RIGHT`; it maps `AV_CH_FRONT_LEFT_OF_CENTER`, `AV_CH_FRONT_RIGHT_OF_CENTER`, `AV_CH_BACK_CENTER`, `AV_CH_TOP_FRONT_LEFT`, and `AV_CH_TOP_FRONT_RIGHT` to their native mask bits, and defines the currently modeled native-mask layouts including `AV_CH_LAYOUT_2POINT1`, `AV_CH_LAYOUT_SURROUND`, `AV_CH_LAYOUT_2_1`, `AV_CH_LAYOUT_4POINT0`, `AV_CH_LAYOUT_2_2`, `AV_CH_LAYOUT_3POINT1`, `AV_CH_LAYOUT_5POINT0_BACK`, `AV_CH_LAYOUT_5POINT0`, `AV_CH_LAYOUT_4POINT1`, `AV_CH_LAYOUT_6POINT0`, `AV_CH_LAYOUT_6POINT0_FRONT`, `AV_CH_LAYOUT_HEXAGONAL`, `AV_CH_LAYOUT_6POINT1`, `AV_CH_LAYOUT_6POINT1_BACK`, `AV_CH_LAYOUT_6POINT1_FRONT`, `AV_CH_LAYOUT_7POINT0`, `AV_CH_LAYOUT_7POINT0_FRONT`, `AV_CH_LAYOUT_7POINT1`, `AV_CH_LAYOUT_7POINT1_WIDE`, `AV_CH_LAYOUT_7POINT1_WIDE_BACK`, `AV_CH_LAYOUT_5POINT1POINT2`, and `AV_CH_LAYOUT_5POINT1POINT2_BACK`. The pinned `channel_layout_map` names those layouts `2.1`, `3.0`, `3.0(back)`, `4.0`, `quad(side)`, `3.1`, `5.0`, `5.0(side)`, `4.1`, `6.0`, `6.0(front)`, `hexagonal`, `6.1`, `6.1(back)`, `6.1(front)`, `7.0`, `7.0(front)`, `7.1`, `7.1(wide)`, `7.1(wide-side)`, `5.1.2`, and `5.1.2(back)`, with `7.1(wide)` mapped to `AV_CH_LAYOUT_7POINT1_WIDE_BACK` and `7.1(wide-side)` mapped to `AV_CH_LAYOUT_7POINT1_WIDE`. `av_channel_layout_default` selects the first map entry with the requested channel count. The Rust model exposes those source-checked shapes and models default counts 1, 2, 3, 4, 5, 6, 7, and 8 as mono, stereo, 2.1, 4.0, 5.0, 5.1, 6.1, and 7.1, while leaving broader height-channel, downmix, custom, ambisonic, and the remaining `ffmpeg -layouts` inventory for later oracle-backed slices.

The `avutil-sample-format` table-string, buffer-layout, fill-array, allocation, silence-fill, and sample-copy slices were checked against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` and its implementations of `av_get_sample_fmt_name`, `av_get_sample_fmt`, `av_get_alt_sample_fmt`, `av_get_packed_sample_fmt`, `av_get_planar_sample_fmt`, `av_get_sample_fmt_string`, `av_get_bytes_per_sample`, `av_sample_fmt_is_planar`, `av_samples_get_buffer_size`, `av_samples_fill_arrays`, `av_samples_alloc`, `av_samples_alloc_array_and_samples`, `av_samples_set_silence`, and `av_samples_copy`. The upstream table-string helper prints the fixed header `name   depth` for negative format values and formats valid native sample formats as a 6-column left-aligned name, three spaces, a 2-column depth, and a trailing space. The upstream buffer-size helper rejects missing sample size, nonpositive sample count, and nonpositive channel count; treats `align=0` as automatic 32-sample padding with byte alignment 1; aligns packed line size as `samples * bytes_per_sample * channels`; aligns planar line size as `samples * bytes_per_sample`; and returns total size as one line for packed data or one line per channel for planar data. The upstream fill-array helper writes a single packed plane or one line-size-spaced pointer per planar channel, returns the computed buffer size, and supports a null input buffer as size-only calculation. The upstream allocation helper allocates one contiguous buffer, fills the plane pointer array, then applies silence for the originally requested sample count; `av_samples_alloc_array_and_samples` first allocates the pointer array and delegates to the same sample allocation path. The upstream silence helper computes packed byte spans with all channels, planar byte spans per channel plane, fills `u8`/`u8p` samples with `0x80`, and fills all other native formats with `0x00`. The upstream copy helper computes packed byte spans with all channels, planar byte spans per channel plane, multiplies source and destination sample offsets by that block alignment, then uses overlap-aware movement when source and destination ranges overlap. The Rust model exposes the table-string shape through `SampleFormat::sample_fmt_string_header` and `SampleFormat::sample_fmt_string`, exposes those storage values through `SampleBufferLayout`, exposes contiguous-buffer plane ranges and safe split helpers through `SampleArrayLayout`, exposes owned contiguous allocation through `SampleAllocation`, exposes silence byte/range/fill helpers through `SampleSilenceRange`, exposes copy byte spans and safe copy helpers through `SampleCopyRange`, and rejects invalid or overflowing bounded Rust inputs before mutation instead of exposing implementation-defined C pointer or integer-overflow behavior. Unlike FFmpeg's raw `av_malloc` tail storage, Rust allocation keeps alignment padding and auto-aligned tail bytes deterministically zeroed.

The `pal8` rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h`, `libavutil/pixdesc.c`, `libavutil/imgutils.c`, and `libavcodec/rawdec.c`. FFmpeg's descriptor marks `pal8` as paletted and alpha-bearing, defines 256 RGB32 palette entries for 1024 palette bytes, includes the palette in image buffer sizing, and lets ordinary rawvideo packets carry only the index plane while the decoder supplies palette state separately. The Rust model currently covers the raw packet index plane and constants only.

The 8-bit planar YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuva420p` as 20 bpp with log2 chroma `(1,1)`, `yuva422p` as 24 bpp with `(1,0)`, and `yuva444p` as 32 bpp with `(0,0)`, all with four 8-bit planes and full-resolution alpha.

The high-bit-depth planar YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuva420p9*` as 22.5 bpp (`45/2`) with log2 chroma `(1,1)`, `yuva422p9*` as 27 bpp with `(1,0)`, `yuva444p9*` as 36 bpp with `(0,0)`, `yuva420p10*` as 25 bpp with `(1,1)`, `yuva422p10*` as 30 bpp with `(1,0)`, `yuva444p10*` as 40 bpp with `(0,0)`, `yuva422p12*` as 36 bpp with `(1,0)`, `yuva444p12*` as 48 bpp with `(0,0)`, `yuva420p16*` as 40 bpp with `(1,1)`, `yuva422p16*` as 48 bpp with `(1,0)`, and `yuva444p16*` as 64 bpp with `(0,0)`, all using two stored bytes per component sample and a full-resolution alpha plane. FFmpeg 8.1.1 does not define `yuva420p12*`.

The semi-planar NV rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `nv16` as 8-bit 4:2:2 with log2 chroma `(1,0)` and 16 bpp, `nv20le`/`nv20be` as 10-bit 4:2:2 with `(1,0)` and 20 bpp using two stored bytes per component sample, and `nv24`/`nv42` as 8-bit 4:4:4 with `(0,0)` and 24 bpp. All five are two-plane formats with one luma plane and one interleaved chroma plane; `nv42` swaps the U/V component order relative to `nv24`.

The high-bit semi-planar P-family rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `p010*`, `p012*`, and `p016*` as 4:2:0 two-plane formats with log2 chroma `(1,1)` and 15/18/24 bpp; `p210*`, `p212*`, and `p216*` as 4:2:2 with `(1,0)` and 20/24/32 bpp; and `p410*`, `p412*`, and `p416*` as 4:4:4 with `(0,0)` and 30/36/48 bpp. All use one luma plane plus one interleaved chroma plane with two stored bytes per component sample.

The high-bit packed YUV 4:2:2 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `y210*`, `y212*`, and `y216*` as one-plane packed 4:2:2 YUV with log2 chroma `(1,0)`, 10/12/16-bit component descriptors, 20/24/32 logical average bpp, and four stored bytes per pixel; `be` variants carry the big-endian descriptor flag.

The packed UYYVYY411 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned enum comment describes packed YUV 4:1:1 storage as `Cb Y0 Y1 Cr Y2 Y3`; the descriptor names it `uyyvyy411`, sets log2 chroma `(2,0)`, 12 logical bpp, one plane, three 8-bit components, and component offsets matching one 6-byte group per 4 pixels.

The packed AYUV64 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `ayuv64le` and `ayuv64be` as one-plane packed AYUV 4:4:4:4 with four 16-bit components, alpha, log2 chroma `(0,0)`, 64 bpp, and eight stored bytes per pixel; the `be` variant carries the big-endian descriptor flag.

The packed XYZ12 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `xyz12le` and `xyz12be` as one-plane packed XYZ 4:4:4 with three 12-bit components, log2 chroma `(0,0)`, 36 bpp, six stored bytes per pixel, and lower four bits of each two-byte component unused; the `be` variant carries the big-endian descriptor flag.

The packed X2RGB10/X2BGR10 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `x2rgb10le`, `x2rgb10be`, `x2bgr10le`, and `x2bgr10be` as one-plane packed RGB-class 10:10:10 formats with three exposed color components, log2 chroma `(0,0)`, 30 bpp, four stored bytes per pixel, and an unused two-bit X lane; the `be` variants carry the big-endian descriptor flag.

The packed X/V YUV 4:4:4 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `xv30le`/`xv30be` and `v30xle`/`v30xbe` as one-plane packed 10-bit 4:4:4 YUV-family formats with three exposed components, no alpha, log2 chroma `(0,0)`, 30 bpp, and four stored bytes per pixel; `xv36le`/`xv36be` have 12-bit components, 36 bpp, and six stored bytes per pixel; `xv48le`/`xv48be` have 16-bit components, 48 bpp, and eight stored bytes per pixel. The X lane is undefined storage padding rather than alpha, and `be` variants carry the big-endian descriptor flag.

The Bayer rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned enum defines `bayer_bggr8`, `bayer_rggb8`, `bayer_gbrg8`, `bayer_grbg8`, and 16-bit little/big-endian variants for each pattern. The pinned descriptors use one plane, three exposed components, no alpha, log2 chroma `(0,0)`, RGB plus Bayer descriptor flags, and the big-endian flag on `be` variants. The Rust model preserves the CFA pattern and byte order in the pixel-format name and sizes payloads as one byte per pixel for 8-bit variants or two bytes per pixel for 16-bit variants.

The packed 8-bit YUV/YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `vuya`, `ayuv`, and `uyva` as one-plane packed 4:4:4:4 YUV-family formats with four 8-bit components, alpha, log2 chroma `(0,0)`, 32 bpp, and four stored bytes per pixel; `vuyx` has three exposed 8-bit components, no alpha, 24 logical bpp, and four stored bytes per pixel with the X lane undefined; `vyu444` has three exposed 8-bit components, no alpha, log2 chroma `(0,0)`, 24 bpp, and three stored bytes per pixel.

The packed floating gray+alpha rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yaf16le`/`yaf16be` as one-plane packed YA formats with two 16-bit IEEE-754 half-float components, alpha, float metadata, log2 chroma `(0,0)`, 32 bpp, and four stored bytes per pixel; `yaf32le`/`yaf32be` are the same shape with two 32-bit IEEE-754 single-precision components, 64 bpp, and eight stored bytes per pixel.

The planar floating GBR rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `gbrpf16le`/`gbrpf16be` as three-plane GBR formats with 16-bit IEEE-754 half-float components, RGB plus planar plus float descriptor flags, no alpha, log2 chroma `(0,0)`, 48 bpp, and two stored bytes per component sample; `gbrpf32le`/`gbrpf32be` use the same layout with 32-bit IEEE-754 single-precision components, 96 bpp, and four stored bytes per component sample.

The packed floating RGB rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `rgbf16le`/`rgbf16be` as one-plane packed RGB formats with 16-bit IEEE-754 half-float components, RGB plus float descriptor flags, no alpha, log2 chroma `(0,0)`, 48 bpp, and six stored bytes per pixel; `rgbf32le`/`rgbf32be` use the same packed shape with 32-bit IEEE-754 single-precision components, 96 bpp, and twelve stored bytes per pixel.

The packed floating RGBA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `rgbaf16le`/`rgbaf16be` as one-plane packed RGBA formats with four 16-bit IEEE-754 half-float components, RGB plus alpha plus float descriptor flags, log2 chroma `(0,0)`, 64 bpp, and eight stored bytes per pixel; `rgbaf32le`/`rgbaf32be` use the same packed shape with 32-bit IEEE-754 single-precision components, 128 bpp, and sixteen stored bytes per pixel.

The packed 32-bit integer RGB/RGBA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `rgb96le`/`rgb96be` as one-plane packed RGB formats with three 32-bit integer components, RGB descriptor flags, no alpha, log2 chroma `(0,0)`, 96 bpp, and twelve stored bytes per pixel; `rgba128le`/`rgba128be` use the same packed integer shape with four 32-bit components, alpha metadata, 128 bpp, and sixteen stored bytes per pixel. The source check also confirmed that `Y410`/`Y412`/`Y416` are documented conceptual aliases in comments around `xv30`/`xv36`/`xv48`, not separate FFmpeg 8.1.1 `AV_PIX_FMT_*` descriptors.

The MSB-aligned planar YUV444/GBR rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuv444p10msble`/`yuv444p10msbbe` and `gbrp10msble`/`gbrp10msbbe` as three-plane 4:4:4 formats with 10-bit components, 30 bpp, two stored bytes per component sample, and descriptor component shifts that place valid bits in the high bits. The `yuv444p12msble`/`yuv444p12msbbe` and `gbrp12msble`/`gbrp12msbbe` descriptors have the same layout with 12-bit components and 36 bpp. The `be` variants carry the big-endian descriptor flag, and the native header aliases map `AV_PIX_FMT_YUV444P10MSB`, `AV_PIX_FMT_YUV444P12MSB`, `AV_PIX_FMT_GBRP10MSB`, and `AV_PIX_FMT_GBRP12MSB` to the target-platform-endian variant.

The 9-bit, 10-bit, 12-bit, 14-bit, and 16-bit planar YUV rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuv420p9*` as 13.5 bpp (`27/2`) with log2 chroma `(1,1)`, `yuv422p9*` as 18 bpp with `(1,0)`, `yuv444p9*` as 27 bpp with `(0,0)`, `yuv420p10*` as 15 bpp with `(1,1)`, `yuv422p10*` as 20 bpp with `(1,0)`, `yuv440p10*` as 20 bpp with `(0,1)`, `yuv444p10*` as 30 bpp with `(0,0)`, `yuv420p12*` as 18 bpp with `(1,1)`, `yuv422p12*` as 24 bpp with `(1,0)`, `yuv440p12*` as 24 bpp with `(0,1)`, `yuv444p12*` as 36 bpp with `(0,0)`, `yuv420p14*` as 21 bpp with `(1,1)`, `yuv422p14*` as 28 bpp with `(1,0)`, `yuv444p14*` as 42 bpp with `(0,0)`, `yuv420p16*` as 24 bpp with `(1,1)`, `yuv422p16*` as 32 bpp with `(1,0)`, and `yuv444p16*` as 48 bpp with `(0,0)`, all using two stored bytes per component sample.
