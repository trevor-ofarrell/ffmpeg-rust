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

That binary is not checked into this repository. On this Windows workspace, use the WSL bootstrap script to build the pinned oracle without requiring system package changes:

```sh
wsl -d Ubuntu --exec bash -lc "cd /mnt/c/Users/trevo/code/ffmpegrust && ./scripts/bootstrap_ffmpeg_oracle_wsl.sh"
```

The script clones tag `n8.1.1`, configures FFmpeg with `--disable-gpl --disable-nonfree --disable-doc --disable-x86asm`, installs the Linux oracle under ignored `third_party/ffmpeg-oracle/wsl/`, and generates wrappers under `third_party/ffmpeg-oracle/build/bin/`. Windows-side tests prefer `ffmpeg.exe`, then the generated `ffmpeg.cmd` WSL wrapper, then the Unix-style `ffmpeg` wrapper; WSL/Linux-side commands use the Unix-style wrapper directly.

The generated Windows `.cmd` wrappers propagate the WSL FFmpeg process exit code. If an older local wrapper returns success for failing oracle invocations, rerun the bootstrap script to refresh it.

Verify the installed oracle before relying on strict completion evidence:

```sh
cargo run -p xtask -- oracle-doctor
```

The doctor command locates the default local `ffmpeg` and `ffprobe` oracle wrappers, runs `-version`, and fails unless both tools report FFmpeg 8.1.1 with the pinned library ABI versions (`libavutil 60.26.101`, `libavcodec 62.28.101`, `libavformat 62.12.101`, `libavdevice 62.3.101`, `libavfilter 11.14.101`, `libswscale 9.5.101`, and `libswresample 6.3.101`). Non-default paths can be checked with `--ffmpeg <path>` and `--ffprobe <path>`.

## Inventory Generation

Run:

```sh
cargo run -p oracle -- inventory --ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg --out compat/ffmpeg-8.1.1
```

On Windows, the generated WSL wrapper can be used directly:

```sh
cargo run -p oracle -- inventory --ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg.cmd --out compat/ffmpeg-8.1.1
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

The inventory tool validates that the supplied oracle reports a first-line `ffmpeg version 8.1.1` banner and fails if any required inventory command exits nonzero. This prevents snapshots from being labeled as the pinned target when the caller accidentally points at another FFmpeg build.

## FATE Samples

FATE samples are expected to be obtained using upstream FFmpeg's documented `make fate-rsync` flow against a local samples directory. This repository does not yet contain samples or an upstream media FATE target mapping.

## Fuzz Execution

Local fuzz execution currently works through WSL Ubuntu with Rust nightly and `cargo-fuzz` installed:

```sh
wsl -d Ubuntu --exec bash -lc "cd /mnt/c/Users/trevo/code/ffmpegrust && . /home/trevo/.cargo/env && cargo fuzz run avutil_core_models -- -runs=1"
```

The same form is used for `avutil_bitreader`, `avutil_byteio`, and `avutil_metadata_options`. Windows-side `cargo-fuzz` is installed, but MSVC ASan runtime/path behavior has been unreliable in this workspace, so WSL is the current recorded fuzz-evidence path.

`cargo run -p fate-runner -- list` reads `PORTING_LEDGER.toml` and lists known components. `cargo run -p fate-runner -- mappings` reads `tests/fate/mappings.txt` and lists configured component-target commands; add repeated `--target <name>` filters to narrow the listing to exact target names, or add `--check-prereqs` with `--samples <path>` and `--oracle-ffmpeg <path>` to resolve placeholders and validate all listed mapping prerequisites without executing commands. `cargo run -p fate-runner -- run --changed` inspects git changed paths, maps currently covered Rust modules, dependency manifests and lockfile, cargo-fuzz target files, `tests/differential/` files, and the rawvideo/dict/pixel-format/sample-format/channel-layout/color/WAV oracle integration harnesses to ledger component IDs, and runs explicit command mappings from the selected mapping file for selected components. Explicit runs may pass `--component <id>` more than once to select multiple components in one invocation; duplicate component IDs are coalesced before execution. Repeated `--target <name>` filters narrow run selection to exact target names, and a selected component whose mappings are all filtered out still fails as unmapped rather than silently passing. Add `--dry-run` to resolve and print selected mappings, including prerequisite validation, without executing them. The mapping format is documented in `tests/fate/README.md` as `component_id|target|workdir|program|arg1|arg2|...`.

Mappings may reference `{samples}` and `{oracle_ffmpeg}` in the workdir, program, or args fields. When a selected mapping uses those placeholders, pass `--samples <path>` and/or `--oracle-ffmpeg <path>` to `fate-runner`; the runner validates that the samples path is an existing directory and the oracle path is an existing file before executing the mapped command. If explicit flags are omitted, `fate-runner` tries `FATE_SAMPLES`, `SAMPLES`, and standard local sample directories (`third_party/fate-samples`, `third_party/fate-suite`, `fate-suite`) for `{samples}`, and `FFMPEG_ORACLE` plus the standard local oracle paths under `third_party/ffmpeg-oracle/build/bin/` for `{oracle_ffmpeg}`. Invalid explicit or environment paths fail before execution rather than falling through silently.

Mapping args may also include `env:NAME=value`; `fate-runner` treats those as child-process environment variables rather than command arguments. Placeholder resolution applies to the environment value, so differential mappings can use `env:FFMPEG_ORACLE={oracle_ffmpeg}` to validate and inject the pinned oracle path for ignored Rust oracle tests. Mapping rows are parsed strictly: only `{samples}` and `{oracle_ffmpeg}` placeholders are accepted, malformed `{...}` placeholders are rejected, and duplicate `env:` names in a single row are rejected before any command runs.

The current default mapping file contains `fate-runner|local-self-test`, which validates local runner wiring by invoking `cargo test -p fate-runner`; `oracle-inventory|local-oracle-unit`, which validates inventory command-list and manifest helper coverage through `cargo test -p oracle`; local `avutil-*|local-avutil-unit` rows, which validate the selected shared primitive through focused `cargo test -p avutil` filters; local `avcodec-*|local-avcodec-unit` rows, which validate the selected initial decoder through focused `cargo test -p avcodec` filters; `avformat-mov-demuxer|local-mov-unit`, which validates that the selected MOV demuxer component can drive `cargo test -p avformat mov::tests` through the FATE runner; local `avformat-rawvideo-*|local-avformat-rawvideo-unit` rows, which validate the current rawvideo demuxer and muxer through focused `cargo test -p avformat rawvideo` filters; `fftools-version|local-version-unit`; `fftools-hide-banner|local-hide-banner-unit`; `fftools-option-parser|local-option-parser-unit`; `fftools-option-parser|local-cli-logging-unit`; `fftools-basic-io|local-io-plan-unit`; shared `local-ffmpeg-unit` rows for the current `fftools-ffmpeg-*` ledger components; and shared `local-ffprobe-unit` rows for the current `fftools-ffprobe-*` ledger components. These mappings do not count as upstream FFmpeg FATE media parity.

Local `avformat-*muxer|local-avformat-*-unit` rows validate the current WAV, raw PCM, rawvideo, yuv4mpegpipe, null, hash, framecrc, framehash, and streamhash unit filters when those muxer components or their shared fuzz target change.

`tests/fate/upstream-mappings.txt` is the first non-default sample-backed mapping file. It currently maps `avformat-wav-demuxer|fate-wav-pcm-s16le-md5` to the sample-specific ignored test in `crates/fftools/tests/wav_oracle.rs`, injecting `FFMPEG_ORACLE={oracle_ffmpeg}` and `FATE_WAV_SAMPLE={samples}/audio-reference/luckynight_2ch_44kHz_s16.wav`. List or dry-run it explicitly:

```sh
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target fate-wav-pcm-s16le-md5
cargo run -p fate-runner -- run --dry-run --mappings tests/fate/upstream-mappings.txt --component avformat-wav-demuxer --target fate-wav-pcm-s16le-md5 --samples <fate-samples> --oracle-ffmpeg <ffmpeg>
```

The mapping is intentionally outside the default local smoke file so ordinary component runs do not require downloaded samples. It does not count as `fate_pass` until executed with a real pinned oracle and matching FATE sample tree.

## Differential Tests

Differential tests must compare Rust outputs to the pinned FFmpeg oracle. FFmpeg may be invoked from tests and oracle tooling only, never as runtime implementation.

`crates/avutil/tests/error_oracle.rs` is an ignored oracle harness for libavutil error strings. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a`, calls `av_strerror`, and compares `AV_ERROR_MAX_STRING_SIZE`, FFmpeg-defined `AVERROR_*` raw values, representative POSIX `AVERROR(errno)` raw values, and returned strings with the Rust `AvErrorCode` / `av_make_error_string` model. On Windows it requires the WSL oracle install created by `scripts/bootstrap_ffmpeg_oracle_wsl.sh`. It is wired into `tests/differential/mappings.txt` as `avutil-error|oracle-libavutil-error-strings`:

```sh
cargo test -p avutil --test error_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-error --target oracle-libavutil-error-strings --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/rational_oracle.rs` is an ignored oracle harness for libavutil rational helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares `av_cmp_q`, `av_q2d`, `av_reduce`, `av_d2q`, `av_nearer_q`, `av_find_nearest_q_idx`, `av_q2intfloat`, `av_gcd_q`, `av_add_q`, `av_sub_q`, `av_mul_q`, `av_div_q`, and `av_inv_q` vectors against the Rust `Rational` model. It is wired into `tests/differential/mappings.txt` as `avutil-rational|oracle-libavutil-rational`:

```sh
cargo test -p avutil --test rational_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-rational --target oracle-libavutil-rational --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/timebase_oracle.rs` is an ignored oracle harness for libavutil timebase helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares `AV_TIME_BASE`, `AV_TIME_BASE_Q`, `av_rescale`, `av_rescale_rnd`, `av_rescale_q`, `av_rescale_q_rnd`, `av_compare_ts`, `av_compare_mod`, `av_rescale_delta`, and `av_add_stable` vectors against the Rust timebase model. It is wired into `tests/differential/mappings.txt` as `avutil-timebase|oracle-libavutil-timebase`:

```sh
cargo test -p avutil --test timebase_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-timebase --target oracle-libavutil-timebase --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/packet_oracle.rs` is an ignored oracle harness for libavcodec `AVPacket` core lifecycle helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavcodec.a`, `libswresample.a`, and `libavutil.a`, then compares `AV_PKT_DATA_NB`, every public `AV_PKT_DATA_*` numeric value, every public `av_packet_side_data_name()` display string, exact public `AV_PKT_FLAG_KEY`, `AV_PKT_FLAG_CORRUPT`, `AV_PKT_FLAG_DISCARD`, `AV_PKT_FLAG_TRUSTED`, and `AV_PKT_FLAG_DISPOSABLE` numeric values, `AV_PICTURE_TYPE_*` numeric values plus `av_get_picture_type_char()` output, public C-struct/enum side-data payload layouts for `AVReplayGain`, `AVCPBProperties`, `AVProducerReferenceTime`, `AVRTCPSenderReport`, `AVDOVIDecoderConfigurationRecord`, and `enum AVAudioServiceType`, plus fixed/comment-defined/raw payload rows for H.263 MB info, quality stats, fallback track, skip samples, parameter change, JP dual mono, string metadata/update, subtitle position, Matroska BlockAdditional, WebVTT identifier/settings, MPEGTS stream ID, A53 CC, AFD, ICC opaque bytes, S12M timecode, frame cropping, and LCEVC. It also compares `av_packet_pack_dictionary` / `av_packet_unpack_dictionary` rows, `av_packet_alloc`, `av_packet_rescale_ts`, `av_packet_copy_props`, `av_packet_ref`, `av_packet_clone`, `av_packet_move_ref`, and `av_packet_unref` behavior against the Rust `Packet` model, including `opaque_ref` data, size, and `av_buffer_is_writable` state, one `AV_PKT_DATA_NEW_EXTRADATA` side-data entry, `av_packet_new_side_data` allocation/replacement plus zero-size allocation, `av_packet_get_side_data`, `av_packet_shrink_side_data` success plus missing ENOENT and oversize ENOMEM error rows, `av_packet_free_side_data`, `av_packet_add_side_data` replacement/append behavior, standalone `av_packet_side_data_new` including zero-size allocation, `av_packet_side_data_add`, `av_packet_side_data_get`, `av_packet_side_data_remove`, and `av_packet_side_data_free` array behavior, `av_packet_side_data_from_frame`, `av_packet_side_data_to_frame`, duplicate frame-side insertion behavior with and without `AV_FRAME_SIDE_DATA_FLAG_REPLACE`, unmapped `EINVAL` paths, `av_new_packet` size/padding/writability including zero-size allocation, `av_packet_from_data` including zero-size ownership, `av_grow_packet`, `av_shrink_packet`, `av_packet_make_refcounted` including empty packets, `av_packet_make_writable` including empty packets, missing side-data lookup size reset, flags, opaque address copying, payload preservation, packet `time_base` fields, and `av_container_fifo_alloc_avpacket()` move/ref write, move/ref read, peek, drain, can-read, and invalid peek behavior. Positive-size `av_new_packet` visible payload bytes are treated as unspecified. It is wired into `tests/differential/mappings.txt` as `avutil-packet|oracle-libavcodec-packet-core`:

The harness also includes a direct `av_init_packet()` row. `Packet::init_legacy()` matches the deterministic reset shape by preserving payload data/size while resetting unknown timestamps and position, zero duration, stream index 0, empty flags, cleared side data, cleared opaque metadata, cleared `opaque_ref`, and `time_base` `0/1`. The safe Rust model releases owned metadata when clearing it; it does not model C leak behavior caused by calling `av_init_packet()` on an already-owned packet.

The harness also includes `packet:side-new-zero` and `packet:array-new-zero` rows, proving FFmpeg 8.1.1 accepts zero-size `AV_PKT_DATA_NEW_EXTRADATA` entries through both packet-owned and standalone side-data allocation APIs.

The harness also includes `packet:payload-new-zero*`, `packet:payload-from-data-zero*`, `packet:payload-make-refcounted-empty*`, and `packet:payload-make-writable-empty*` rows, proving zero-size packet payload helpers keep zero visible payload bytes while retaining zeroed FFmpeg input padding and writable refcounted storage.

The harness also includes `packet:side-add-capacity-*` and `packet:side-new-capacity-overflow` rows, proving packet-owned side-data capacity behavior at `AV_PKT_DATA_NB`: replacement remains valid at capacity, append fails with `ERANGE` without changing the entry count, and `av_packet_new_side_data()` returns NULL at capacity.

The harness also includes `packet:fifo-*` rows, proving the packet-specialized container FIFO transfer semantics for move writes, ref writes, read draining, non-mutating peek, valid drain, can-read counts, and invalid offset handling.

```sh
cargo test -p avutil --test packet_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --target oracle-libavcodec-packet-core --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/byteio_oracle.rs` is an ignored oracle harness for libavutil byte-order helpers. It compiles a small test-only C helper against the pinned `third_party/ffmpeg-oracle/wsl/include/libavutil/intreadwrite.h` header and compares `AV_RB*`, `AV_RL*`, `AV_WB*`, and `AV_WL*` 8/16/24/32/48/64-bit read/write byte-order behavior plus signed interpretation vectors against the Rust `ByteReader`/`ByteWriter` model. It is wired into `tests/differential/mappings.txt` as `avutil-byteio|oracle-libavutil-byteio`:

```sh
cargo test -p avutil --test byteio_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-byteio --target oracle-libavutil-byteio --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/bitreader_oracle.rs` is an ignored oracle harness for libavcodec `GetBitContext` helpers. It compiles a small test-only C helper against the pinned FFmpeg 8.1.1 source/build cache created by `scripts/bootstrap_ffmpeg_oracle_wsl.sh`, using `libavcodec/get_bits.h`, `libavcodec/golomb.h`, and the matching generated `config.h`; it links pinned libavcodec/libavutil only inside the test for Golomb tables. The harness compares `init_get_bits8`, `show_bits`, `get_bits`, `get_bits1`, `get_bits_long`, `get_bits64`, `get_sbits`, `get_sbits64`, `skip_bits`, `skip_bits_long`, `align_get_bits`, `get_ue_golomb`, and `get_se_golomb` value/cursor vectors against the Rust `BitReader` model. On Windows the default source/build cache paths are `$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1/src` and `$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1/build` inside WSL; override them with `FFMPEGRUST_FFMPEG_SOURCE` and `FFMPEGRUST_FFMPEG_BUILD` if needed. It is wired into `tests/differential/mappings.txt` as `avutil-bitreader|oracle-libavcodec-get-bits`:

```sh
cargo test -p avutil --test bitreader_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-bitreader --target oracle-libavcodec-get-bits --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/bitwriter_oracle.rs` is an ignored oracle harness for libavcodec `PutBitContext` helpers. It uses the same pinned FFmpeg 8.1.1 source/build cache as the bitreader oracle, includes `libavcodec/put_bits.h` and `libavcodec/put_golomb.h`, and links pinned libavcodec/libavutil only inside the test for Golomb tables. The harness compares `init_put_bits`, `put_bits`, `put_bits32`, `put_bits63`, `put_bits64`, `put_sbits`, `put_sbits63`, `align_put_bits`, `set_ue_golomb`, `set_ue_golomb_long`, `set_se_golomb`, `put_bits_count`, and `put_bytes_count` flushed byte/cursor vectors against the Rust `BitWriter` model. It is wired into `tests/differential/mappings.txt` as `avutil-bitwriter|oracle-libavcodec-put-bits`:

```sh
cargo test -p avutil --test bitwriter_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-bitwriter --target oracle-libavcodec-put-bits --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/hash_oracle.rs` is an ignored oracle harness for libavutil generic hash helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares `av_hash_names`, `av_hash_alloc`, `av_hash_get_name`, `av_hash_get_size`, `av_hash_update`, `av_hash_final`, `av_hash_final_bin`, `av_hash_final_hex`, and `av_hash_final_b64` output for the full FFmpeg 8.1.1 default-native generic hash inventory: MD5, murmur3, RIPEMD128, RIPEMD160, RIPEMD256, RIPEMD320, SHA160, SHA224, SHA256, SHA512/224, SHA512/256, SHA384, SHA512, CRC32, and adler32. It is wired into `tests/differential/mappings.txt` as `avutil-hash|oracle-libavutil-hash`:

```sh
cargo test -p avutil --test hash_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-hash --target oracle-libavutil-hash --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/dict_oracle.rs` is an ignored oracle harness for libavutil AVDictionary helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares flag constants, null dictionary count, set/get/iterate/count behavior, case sensitivity, prefix scans, `AV_DICT_DONT_OVERWRITE`, `AV_DICT_APPEND`, `AV_DICT_MULTIKEY`, `AV_DICT_DEDUP`, delete-by-NULL, `av_dict_set_int`, `av_dict_get_string`, `av_dict_parse_string`, partial parse failure, and `av_dict_copy` behavior against the Rust `Dictionary` model. It is wired into `tests/differential/mappings.txt` as `avutil-dict|oracle-libavutil-dict`:

```sh
cargo test -p avutil --test dict_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-dict --target oracle-libavutil-dict --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/buffer_oracle.rs` is an ignored oracle harness for libavutil `AVBufferRef` and bounded `AVBufferPool` helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares allocation status including `av_buffer_alloc(0)` and `av_buffer_allocz(0)`, zeroed allocation, ref sharing, writable `av_buffer_create` opaque-data owners, av_buffer_create-style cloned refs preserving opaque/refcount/non-writable shared state, writable custom-owner realloc release/replacement, offset `data`/`size` subrange refs, cloned offset refs preserving visible data pointer/size/refcount shape, unique offset make-writable no-op behavior, unique/shared offset realloc detach behavior, refcount/writability, make-writable copy-on-write, readonly external opaque release behavior, realloc grow/shrink/shared/offset detach prefix preservation, zero-size realloc status for existing ordinary buffers and NULL destinations, ordinary alloc-origin realloc replacement, repeated nullable-realloc status, same-size realloc no-op behavior for shared refs, writable custom-owner refs, and readonly owner refs, replace sharing including same-buffer offset refs, nullable `av_buffer_replace`/`av_buffer_unref` rows including NULL-to-NULL replace no-op, unref nulling, default-pool allocator fallback/no-opaque reuse rows, `av_buffer_pool_init2` default-allocator fallback plus pool-free rows, custom-pool allocator bytes, `av_buffer_pool_buffer_get_opaque` rows, no-clear pool reuse, custom allocator failure return/no-release behavior, spare release on pool uninit, pool_free callback timing, and outstanding-buffer release after pool uninit against the Rust `BufferRef`/`BufferPool` model. Newly allocated/grown bytes from `av_realloc` are treated as unspecified. It is wired into `tests/differential/mappings.txt` as `avutil-buffer|oracle-libavutil-buffer`:

```sh
cargo test -p avutil --test buffer_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-buffer --target oracle-libavutil-buffer --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/fftools/tests/version_oracle.rs` is an ignored oracle harness for `ffmpeg -version`, `ffprobe -version`, and the `-hide_banner -version` variant. It checks the pinned tool-version prefix and the libav* ABI versions reported by the oracle against the Rust `ffmpeg-rs`/`ffprobe-rs` banner constants, and verifies that `-hide_banner -version` preserves the same version surface as `-version`. `FFMPEG_ORACLE` points to the pinned `ffmpeg` binary; `ffprobe` is found through `FFPROBE_ORACLE`, a sibling of `FFMPEG_ORACLE`, or the standard `third_party/ffmpeg-oracle/build/bin/ffprobe(.exe)` path. Run it with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test version_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-version --target oracle-ffmpeg-version --target oracle-ffprobe-version --target oracle-hide-banner-version --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

The same harness also checks that `--version` is not a clean successful version request for either tool, matching upstream option parsing rather than GNU-style aliases:

```sh
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-version --target oracle-double-dash-version-rejection --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

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

The same harness is wired into `tests/differential/mappings.txt`, which can be executed through `fate-runner` with `--mappings tests/differential/mappings.txt --oracle-ffmpeg <path> --component fftools-ffmpeg-rawvideo-file-output`. The current `rgb24` and `gbrp10msble` rows have passed locally through the generated WSL oracle wrapper.

`crates/avutil/tests/channel_layout_oracle.rs` is an ignored oracle harness for `ffmpeg -layouts`. It compares individual-channel names/descriptions and standard-layout decompositions against the current Rust `Channel::ALL` and `ChannelLayout::known_layouts()` inventories. It is wired into `tests/differential/mappings.txt` as `avutil-channel-layout|oracle-ffmpeg-layouts`; local execution now passes through the pinned WSL oracle wrapper after adding `3.1.2` and matching FFmpeg's `22.2` and `hexadecagonal` decomposition order.

`crates/avutil/tests/pixel_format_oracle.rs` is an ignored oracle harness for `ffmpeg -pix_fmts`. It checks that every currently modeled `PixelFormat::ALL` descriptor name appears in the oracle inventory with matching component count, integer bits-per-pixel value where the Rust descriptor is exact, and paletted flag. It intentionally does not claim full FFmpeg pixel inventory parity. It is wired into `tests/differential/mappings.txt` as `avutil-pixel-format|oracle-ffmpeg-pix-fmts-subset`. Run it with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p avutil --test pixel_format_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-pixel-format --target oracle-ffmpeg-pix-fmts-subset --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/sample_format_oracle.rs` contains ignored oracle harnesses for sample-format parity. The inventory test compares `ffmpeg -sample_fmts` name/depth output against `SampleFormat::ALL`; the libavutil test compiles a small C helper against the pinned local `libavutil.a` and validates native sample-format metadata, table strings, `av_samples_get_buffer_size`, `av_samples_fill_arrays`, `av_samples_alloc`, `av_samples_set_silence`, and `av_samples_copy` vectors. They are wired into `tests/differential/mappings.txt` as `avutil-sample-format|oracle-ffmpeg-sample-fmts` and `avutil-sample-format|oracle-libavutil-sample-format`. Run them with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p avutil --test sample_format_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-sample-format --target oracle-ffmpeg-sample-fmts --target oracle-libavutil-sample-format --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/color_oracle.rs` is an ignored oracle harness for `ffmpeg -colors`. It compares the oracle color-name/RGB table against `NamedColor::ALL` and is wired into `tests/differential/mappings.txt` as `avutil-color|oracle-ffmpeg-colors`. Run it with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p avutil --test color_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-color --target oracle-ffmpeg-colors --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/fftools/tests/wav_oracle.rs` contains ignored WAV oracle tests for the current demuxer path. `wav_pcm_s16le_generated_md5_matches_ffmpeg_oracle` creates a small PCM s16le WAV fixture through the Rust WAV muxer and compares Rust `-i <generated.wav> -f md5 -` output against pinned FFmpeg 8.1.1 `-c:a copy -f md5 -` output, so it requires only the oracle binary. It is wired into `tests/differential/mappings.txt` as `avformat-wav-demuxer|oracle-wav-generated-md5`:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test wav_oracle wav_pcm_s16le_generated_md5_matches_ffmpeg_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-wav-demuxer --target oracle-wav-generated-md5 --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample` uses the FATE PCM s16le sample and is wired through `tests/fate/upstream-mappings.txt`. Run it directly with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg FATE_WAV_SAMPLE=./third_party/fate-samples/audio-reference/luckynight_2ch_44kHz_s16.wav cargo test -p fftools --test wav_oracle wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample -- --ignored
```

On Windows PowerShell:

```powershell
$env:FFMPEG_ORACLE = ".\third_party\ffmpeg-oracle\build\bin\ffmpeg.exe"
$env:FATE_WAV_SAMPLE = ".\third_party\fate-samples\audio-reference\luckynight_2ch_44kHz_s16.wav"
cargo test -p fftools --test wav_oracle wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample -- --ignored
```

## Source-Checked Notes

The `avutil-color` inventory and parser slice was checked against FFmpeg 8.1.1 `libavutil/parseutils.c` and `fftools/opt_common.c`. The pinned `color_table` contains 140 named RGB colors from `AliceBlue` to `YellowGreen`, including the source spelling `Darkorange`; `av_get_known_color_name` returns indexed table entries; `av_parse_color` strips leading `#` or lowercase `0x` for forced hex parsing, accepts bare all-hex color strings as hex, accepts only six- or eight-digit color hex payloads, treats eight-digit colors as `RRGGBBAA`, applies alpha suffixes after named or hex parsing, parses lowercase-`0x` alpha suffixes as byte-range hex values, parses decimal alpha suffixes as normalized values multiplied by 256 and rejected when they exceed byte range, and uses case-insensitive named lookup as the fallback branch. `show_colors` prints a `name`/`#RRGGBB` table with lowercase hex. The current Rust model covers that bounded deterministic subset plus typed invalid-input rejection; FFmpeg's nondeterministic `random`/`bikeshed` seed branch, exact C buffer truncation behavior, and oracle-vector calibration for unusual `strtod`/`strtoul` edges remain pending.

The `avutil-error` string slice was checked against FFmpeg 8.1.1 `libavutil/error.h` and `libavutil/error.c`. The pinned header defines `AVERROR(e)` as the negated platform errno value on the current default-native oracle profile, tag-based `AVERROR_*` constants, and `AV_ERROR_MAX_STRING_SIZE`; the pinned C implementation maps the `AVERROR_LIST` entries to user-facing strings, delegates errno-backed descriptions through the platform strerror path, and falls back to `Error number <n> occurred` when no description is found. The ignored `error_oracle` harness now validates the FFmpeg-defined table, representative POSIX errno-backed descriptions, and unknown-code fallback against the pinned local libavutil oracle. This component is complete for the selected default-native profile; non-POSIX platform errno profiles can add their own oracle rows later if needed.

The `avutil-rational` helper slices were checked against FFmpeg 8.1.1 `libavutil/rational.h` and `libavutil/rational.c`. The pinned header documents `av_cmp_q`, `av_nearer_q`, `av_find_nearest_q_idx`, `av_q2intfloat`, and `av_gcd_q`; the pinned implementation treats positive and negative zero-denominator rationals as infinities for comparison, treats `0/0` as indeterminate, scans a `{0,0}`-terminated nearest list while preserving the first candidate on ties, converts rationals to platform-independent IEEE single-precision bit patterns with `av_q2intfloat`, and returns `av_gcd_q` results as raw `gcd(num)/lcm(den)` rationals only when `lcm < max_den`. The ignored `rational_oracle` harness now validates the modeled rational helper surface against pinned local libavutil, including arithmetic helpers, reduction, double conversion, nearest selection, int-float conversion, and rational GCD/default vectors. This component is complete for the selected default-native profile; C-level undefined-overflow cases remain outside the safe Rust API boundary and are rejected with typed errors where exposed.

The `avutil-timebase` helper slices were checked against FFmpeg 8.1.1 `libavutil/mathematics.h` and `libavutil/mathematics.c`. The pinned source defines `AV_ROUND_*`, `AV_ROUND_PASS_MINMAX`, `av_rescale*`, `av_compare_ts`, `av_compare_mod`, `av_rescale_delta`, and `av_add_stable`; the current Rust model covers checked rescale helpers, `av_compare_ts`, `av_compare_mod`, `av_rescale_delta`-style stateful duration-preserving timestamp conversion for nonnegative FFmpeg-int durations, and `av_add_stable`-style timestamp increments with exact positive tick addition, negative-increment no-op behavior through FFmpeg's `m < d` branch, sub-tick no-op behavior, and positive fractional no-drift updates. The ignored `timebase_oracle` harness now validates that modeled surface against pinned local libavutil. This component is complete for the selected default-native profile; C-level undefined-overflow or assertion-only invalid cases remain outside the safe Rust API boundary and are rejected with typed errors where exposed.

The `avutil-channel-layout` slice was checked against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c`. The pinned header defines the currently modeled native channels from front/center/back/side positions through height, wide, downmix, binaural, and 22.2-specific entries including `AV_CHAN_TOP_CENTER`, `AV_CHAN_SURROUND_DIRECT_LEFT`, `AV_CHAN_SURROUND_DIRECT_RIGHT`, `AV_CHAN_BOTTOM_FRONT_CENTER`, `AV_CHAN_BOTTOM_FRONT_LEFT`, `AV_CHAN_BOTTOM_FRONT_RIGHT`, `AV_CHAN_SIDE_SURROUND_LEFT`, `AV_CHAN_SIDE_SURROUND_RIGHT`, `AV_CHAN_TOP_SURROUND_LEFT`, and `AV_CHAN_TOP_SURROUND_RIGHT`; it maps their `AV_CH_*` macros to native mask bits including `1 << 11`, `1 << 33`, `1 << 34`, `1 << 38`, `1 << 39`, `1 << 40`, `1 << 41`, `1 << 42`, `1 << 43`, and `1 << 44` for `TC`, `SDL`, `SDR`, `BFC`, `BFL`, `BFR`, `SSL`, `SSR`, `TTL`, and `TTR`. The pinned source maps those short names to `top center`, `surround direct left`, `surround direct right`, `bottom front center`, `bottom front left`, `bottom front right`, `side surround left`, `side surround right`, `top surround left`, and `top surround right`, and `channel_layout_map` names the currently modeled layouts from `mono` through `downmix` plus `22.2`, including `3.1.2`. The same header defines `AV_CHAN_NONE = -1`, `AV_CHAN_UNUSED = 0x200`, `AV_CHAN_UNKNOWN = 0x300`, and ambisonic custom-channel IDs from `AV_CHAN_AMBISONIC_BASE = 0x400` through `AV_CHAN_AMBISONIC_END = 0x7ff`, where ACN is `id - AV_CHAN_AMBISONIC_BASE`. The pinned source formats those channel IDs as `NONE`, `UNSD`, `UNK`, `AMBI<n>`, or `USR<raw>` and descriptions as `none`, `unused`, `unknown`, `ambisonic ACN <n>`, or `user <raw>`. The pinned `AVChannelCustom` shape stores an `AVChannel id`, `char name[16]`, and opaque pointer; `av_channel_layout_custom_init` rejects nonpositive channel counts and initializes every custom entry to `AV_CHAN_UNKNOWN`; `av_channel_layout_check` rejects custom layouts with a null map or any `AV_CHAN_NONE` entry; custom channel-index lookup returns the first matching raw channel ID; string lookup supports custom names as `CH@name` or `@name`; and custom descriptions print `<n> channels (CH[@name]+...)` when a custom map cannot be reduced to a named/native description. The pinned `masked_description` helper accepts a custom map as a native mask only when every remaining entry is a native channel ID below 63 and native IDs appear in strictly increasing order; `canonical_order` refuses native reduction for custom maps with any custom names, returns `UNSPEC` for all-unknown maps, selects `NATIVE` when `masked_description` succeeds, and can select `AMBISONIC` when `av_channel_layout_ambisonic_order` accepts a complete standard-order ambisonic prefix. For custom maps, `av_channel_layout_ambisonic_order` requires all ambisonic channels to appear before any non-ambisonic channel, requires each ambisonic ACN to equal its map index, rejects missing or incomplete ambisonic orders, and describes valid maps as `ambisonic <order>` plus optional trailing-channel descriptions. `av_channel_layout_compare` first rejects different channel counts, treats one unspecified layout as unequal and two unspecified layouts as equal, compares masks directly for same-order native or ambisonic layouts, and otherwise compares `av_channel_layout_channel_from_index` results for each position, ignoring custom names. `av_channel_layout_subset` returns `mask & layout->u.mask` for native and ambisonic layouts, and for custom layouts scans native channel mask bits 0 through 63 and includes a bit when `av_channel_layout_index_from_channel` finds that raw channel ID in the custom map. `av_channel_layout_channel_from_index` returns native channels in mask-bit order, returns ambisonic channels as `AMBI0..AMBI<N>` followed by native mask extras, returns custom map entries in map order, and returns `AV_CHAN_NONE` for out-of-range or unsupported order values; `av_channel_layout_index_from_channel` returns the first map or mask-bit index for a channel and rejects absent or `AV_CHAN_NONE` channels; `av_channel_layout_index_from_string` applies `av_channel_from_string` lookup for native layouts and additionally supports `CH@name`/`@name` custom map lookup before falling back to raw channel lookup for custom layouts; and `av_channel_layout_channel_from_string` combines string lookup with index lookup. The Rust custom-map and explicit-ambisonic helpers mirror bounded native canonicalization, custom-map ambisonic prefix detection/description, current native/ambisonic/custom channel-by-index equivalence, current native/ambisonic/custom subset-mask extraction, and current native/custom index/string lookup for the currently modeled layouts while leaving lossy native/custom/ambisonic retyping, unspecified string parsing/retyping, and full `AV_CHANNEL_ORDER_AMBISONIC` layout semantics out of scope. The `22.2` macro is `AV_CH_LAYOUT_9POINT1POINT6 | AV_CH_BACK_CENTER | AV_CH_LOW_FREQUENCY_2 | AV_CH_TOP_FRONT_CENTER | AV_CH_TOP_CENTER | AV_CH_TOP_BACK_CENTER | AV_CH_BOTTOM_FRONT_CENTER | AV_CH_BOTTOM_FRONT_LEFT | AV_CH_BOTTOM_FRONT_RIGHT`, with `AV_CHANNEL_LAYOUT_22POINT2` declaring 24 channels. The Rust model exposes those source-checked channel ID, native-layout, explicit ambisonic native-extra-layout, count-only unspecified-layout, and initial custom-map shapes, and its known channel/layout inventory now passes `ffmpeg -layouts`; full `av_channel_layout_from_string()` grammar, broader custom/native/ambisonic/unspecified retyping, and full ambisonic layout order semantics remain for later oracle-backed slices.

The latest default-layout slice additionally source-checks that `av_channel_layout_default` scans `channel_layout_map` in source order and chooses the first layout whose `nb_channels` matches the requested count, otherwise producing an unspecified-order layout with that count. `av_channel_layout_check` rejects nonpositive counts and accepts unspecified layouts without a union payload. `av_channel_layout_describe` formats unspecified layouts as `<n> channels`; `av_channel_layout_compare` compares two unspecified layouts equal only when the channel counts match and treats one unspecified plus one non-unspecified layout as unequal; and `av_channel_layout_subset` returns no native bits for unspecified layouts. `ChannelLayout::default_for_count` remains a native-only helper for the modeled source-order defaults, while `ChannelLayoutSpec::default_for_count` returns those native defaults for 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, and 24 channels and returns a count-only `UnspecifiedChannelLayout` for other positive counts. AudioFrame and avformat AudioStreamParameters now store `ChannelLayoutSpec`; their legacy `channel_layout()` accessors remain native-only compatibility helpers.

The latest parser slice source-checks `av_channel_layout_from_string()` channel-list, numeric-mask, count-suffix, and ambisonic branch behavior. Pinned FFmpeg checks exact case-sensitive named layouts before the later parser branches, supports an `ambisonic <order>` branch with optional `+extra` layout, tries channel lists and `N channels (<list>)` descriptions, then parses numeric masks, `Nc`, `NC`, and `N channels`. The `parse_channel_list` path uses `av_opt_get_key_value` with `@` channel-name separators and `+` pair separators, resolves each channel token with the case-sensitive `av_channel_from_string`, stores optional `AVChannelCustom.name` values, initially creates `AV_CHANNEL_ORDER_CUSTOM`, and then canonical retyping can reduce nameless native-channel maps to native masks or all-unknown maps to unspecified order. Source checking against pinned `libavutil/opt.c` and `libavutil/avstring.c` confirms the channel-list tokenizer also skips FFmpeg ASCII whitespace, treats a missing key as an implicit channel token, accepts single-quoted token segments, backslash-escaped separators, unescaped `@` in the value after the first key separator, and a final trailing `+` after a valid token, while leading or repeated empty tokens such as `+FL` and `FL++FR` still fail after channel-token resolution. The current Rust `ChannelLayoutSpec` parser mirrors the bounded native/custom channel-list subset by resolving lists such as `FL+FR` and quoted IDs such as `'FL'+FR` to named native layouts when possible, preserving lists such as `FL`, `FL+`, `FL+FC`, and `2 channels (FL+FC)` as exact `NativeChannelMaskLayout` values, preserving named or otherwise non-canonical maps such as `FL@Left+FR@Right`, `FL@Left\+Right+FR`, `FL@Left\@Name+FR`, `FL@Left@Again`, `FL+FL`, and `UNK+UNSD+AMBI2@Height+USR2048@Vendor` as first-class custom specs, and reducing all-UNK nameless maps to `UnspecifiedChannelLayout`; it rejects lowercase or otherwise mismatched channel/layout names such as `fl+fr`, `unk+unk`, `ambi0`, `usr0`, and `STEREO` rather than falling through to the ergonomic `ChannelLayout::parse` helper. The current Rust ambisonic branch accepts bounded orders that fit the AMBI0..AMBI1023 range, stores no-extra and native-extra forms such as `ambisonic 1`, `ambisonic 1+stereo`, `ambisonic 1+0x5`, signed-zero `ambisonic -0`, and no-conversion `ambisonic +stereo` / `ambisonic -0+stereo` as explicit `AmbisonicChannelLayout` order/native-mask specs where applicable, preserves named/custom extras such as `+FL@Left+FR@Right` as custom AMBI-prefix maps, and rejects real negative orders, unspecified extras, or nested ambisonic extras. The mask branch uses `strtoull(str, &end, 0)`, requires no parse overflow, full-string consumption, no `-` anywhere in the input, and a nonzero mask, then initializes an `AV_CHANNEL_ORDER_NATIVE` layout with that exact mask. The current Rust parser resolves modeled base-0 masks such as `0x3`, `3`, `03`, leading-whitespace ` 0x3`, and `+0x3` to named native layouts, preserves arbitrary FFmpeg-valid nonzero masks such as `0x5` and `0x8000000000000000` as exact `NativeChannelMaskLayout` values, and rejects trailing-junk mask forms such as `0x3 `. The lowercase `Nc` branch calls `av_channel_layout_default` and returns success only when the resulting default order is native, so `2c`, leading-whitespace ` 2c`, and `10c` are accepted by the current Rust model while `9c` and trailing-junk `2c ` are rejected. Uppercase `NC` and `N channels` produce `AV_CHANNEL_ORDER_UNSPEC` count-only layouts for positive counts, including leading-plus forms such as `+2C` and ` +2 channels`, while trailing-junk forms such as `2C `, `2 channels `, and `2 channels (FL+FR) ` reject after full-consumption checks. The latest Rust byte-entry slice adds `CustomChannelLayout::parse_channel_list_bytes` and `ChannelLayoutSpec::parse_bytes`, routing valid UTF-8 bytes through the same parser while rejecting non-UTF-8 byte strings as typed invalid data instead of lossy conversion; byte-preserving non-UTF-8 custom-name parity, full `AV_CHANNEL_ORDER_AMBISONIC` comparison/index string parity, and broad retyping remain pending.

The latest ambisonic lookup slice uses the pinned `av_channel_layout_channel_from_index`, `av_channel_layout_index_from_channel`, `av_channel_layout_index_from_string`, and `av_channel_layout_channel_from_string` behavior for the bounded explicit ambisonic surface: ambisonic ACNs map to the leading indexes, native extra-mask channels follow in mask-bit order, canonical strings map through the same lookup, and absent/custom-name/invalid lookups fail without producing channels. Broader `AV_CHANNEL_ORDER_AMBISONIC` retyping and oracle-vector calibration remain pending.

The latest ambisonic retype slice uses the pinned `canonical_order` and `av_channel_layout_retype(..., AV_CHANNEL_ORDER_AMBISONIC, CANONICAL)` shape for the bounded lossless custom-map subset: custom names prevent lossless retyping, a complete standard-order `AMBI0..AMBI<N>` prefix is required, and trailing extras must be native raw channel IDs in strictly increasing mask-bit order. The Rust model now maps matching nameless channel-list parses to explicit `AmbisonicChannelLayout` specs and keeps named, unknown-extra, incomplete, or out-of-order forms as custom maps.

The latest custom-target retype slice uses the pinned `av_channel_layout_retype(..., AV_CHANNEL_ORDER_CUSTOM, ...)` shape for the current bounded variants. FFmpeg initializes a custom map with UNKNOWN entries for count-only UNSPEC layouts, and otherwise fills each custom entry from `av_channel_layout_channel_from_index` before replacing the layout. The Rust `ChannelLayoutSpec::to_custom_layout` mirrors that bounded behavior for native, arbitrary native-mask, explicit ambisonic, custom, and count-only unspecified specs without claiming the remaining lossy retype flags or complete order coverage.

The latest native/unspecified-target retype slice uses the pinned `av_channel_layout_retype(..., AV_CHANNEL_ORDER_NATIVE, flags)` and `av_channel_layout_retype(..., AV_CHANNEL_ORDER_UNSPEC, flags)` shape for the current bounded variants. FFmpeg treats already-target-order layouts as no-ops; accepts target-NATIVE from custom order only when `masked_description` can form a strictly ordered native mask, returning a lossy result only when custom names are dropped; and accepts target-UNSPEC from any non-target order when lossy conversion is allowed, with custom all-UNKNOWN/no-name maps remaining lossless. The Rust helpers `ChannelLayoutSpec::retype_to_native_order` and `ChannelLayoutSpec::retype_to_unspecified_order` mirror those bounded branches and expose the returned lossy bit through `ChannelLayoutRetypeResult`, while the lossless wrapper methods keep rejecting the name- or identity-dropping cases.

The latest ambisonic/canonical-target retype slice uses the pinned `av_channel_layout_retype(..., AV_CHANNEL_ORDER_AMBISONIC, flags)` and `AV_CHANNEL_LAYOUT_RETYPE_FLAG_CANONICAL` branches. FFmpeg accepts target-AMBISONIC from custom order only when `av_channel_layout_ambisonic_order` finds a complete standard-order AMBI prefix and `masked_description` can form a native extra-channel mask after that prefix; custom names only make that conversion lossy. With CANONICAL, FFmpeg first computes `canonical_order`, which refuses to drop custom names, turns all-UNKNOWN nameless maps into UNSPEC, turns strict native maps into NATIVE, turns strict ambisonic maps into AMBISONIC, and leaves unreducible custom maps as CUSTOM no-ops. The Rust helpers `ChannelLayoutSpec::retype_to_ambisonic_order` and `retype_to_canonical_order` mirror those bounded current-model branches; oracle-vector coverage is still pending.

The latest raw-channel-mask retype slice source-checks the pinned `masked_description` condition `ch >= 0 && ch < 63` and `mask < (1ULL << ch)`: custom channel IDs do not have to be named `AV_CHAN_*` entries to reduce to native-mask order. The Rust model now lets nameless custom lists such as `USR45+USR46` retype to exact `NativeMask` values and lets complete AMBI prefixes with `USR45`-style trailing extras retype to explicit ambisonic native-extra masks, while `USR63`, out-of-range `USR2048`, duplicates, and descending raw-ID order still reject native/ambisonic retyping.

The latest explicit-ambisonic raw-extra lookup slice applies the same raw-ID native-mask principle to explicit `AmbisonicChannelLayout` lookup for the bounded Rust model. Layouts such as `ambisonic 1+0x200000000000` now expose the extra channel as canonical `USR45` through index lookup, string lookup, channel-by-index, subset-mask, and custom-equivalence helpers, while noncanonical direct user IDs such as `User(0)`, absent raw extras, special unknown/unused IDs, and custom-name strings still reject.

The latest channel-string parser slice source-checks pinned `av_channel_from_string`. Native short names and `UNK`/`UNSD` are exact case-sensitive matches; `AMBI` names use `strtol(..., base 0)` after the uppercase prefix with a null end pointer and only range-check the parsed ACN, so trailing text and no-conversion suffixes such as `AMBIx`, `AMBI+`, and signed-zero `AMBI-0tail` resolve as ACN 0 while real negative ACNs reject; `USR` names use the same base-0 parser after the uppercase prefix, require full-string consumption and a nonnegative raw ID, and accept the bare `USR` no-conversion case plus signed-zero `USR-0` as raw ID 0. The Rust model keeps canonical generated names strict while routing layout parsing and channel string lookup through this parser-facing form, covering inputs such as `USR0x2d+USR056`, `USR`, `USR+`, `USR-0`, `USR0`, `USR055`, `USR+45`, `AMBIx`, and `AMBI0x1tail`, and rejecting lowercase forms such as `fl`, `ambi0`, and `usr0`.

The `avutil-sample-format` table-string, buffer-layout, fill-array, allocation, silence-fill, and sample-copy slices were checked against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` and its implementations of `av_get_sample_fmt_name`, `av_get_sample_fmt`, `av_get_alt_sample_fmt`, `av_get_packed_sample_fmt`, `av_get_planar_sample_fmt`, `av_get_sample_fmt_string`, `av_get_bytes_per_sample`, `av_sample_fmt_is_planar`, `av_samples_get_buffer_size`, `av_samples_fill_arrays`, `av_samples_alloc`, `av_samples_alloc_array_and_samples`, `av_samples_set_silence`, and `av_samples_copy`. The upstream table-string helper prints the fixed header `name   depth` for negative format values and formats valid native sample formats as a 6-column left-aligned name, three spaces, a 2-column depth, and a trailing space. The upstream buffer-size helper rejects missing sample size, nonpositive sample count, and nonpositive channel count; treats `align=0` as automatic 32-sample padding with byte alignment 1; aligns packed line size as `samples * bytes_per_sample * channels`; aligns planar line size as `samples * bytes_per_sample`; and returns total size as one line for packed data or one line per channel for planar data. The upstream fill-array helper writes a single packed plane or one line-size-spaced pointer per planar channel, returns the computed buffer size, and supports a null input buffer as size-only calculation. The upstream allocation helper allocates one contiguous buffer, fills the plane pointer array, then applies silence for the originally requested sample count; `av_samples_alloc_array_and_samples` first allocates the pointer array and delegates to the same sample allocation path. The upstream silence helper computes packed byte spans with all channels, planar byte spans per channel plane, fills `u8`/`u8p` samples with `0x80`, and fills all other native formats with `0x00`. The upstream copy helper computes packed byte spans with all channels, planar byte spans per channel plane, multiplies source and destination sample offsets by that block alignment, then uses overlap-aware movement when source and destination ranges overlap. The Rust model exposes the table-string shape through `SampleFormat::sample_fmt_string_header` and `SampleFormat::sample_fmt_string`, exposes those storage values through `SampleBufferLayout`, exposes contiguous-buffer plane ranges and safe split helpers through `SampleArrayLayout`, exposes owned contiguous allocation through `SampleAllocation`, exposes silence byte/range/fill helpers through `SampleSilenceRange`, exposes copy byte spans and safe copy helpers through `SampleCopyRange`, and rejects invalid or overflowing bounded Rust inputs before mutation instead of exposing implementation-defined C pointer or integer-overflow behavior. Unlike FFmpeg's raw `av_malloc` tail storage, Rust allocation keeps alignment padding and auto-aligned tail bytes deterministically zeroed.

The latest ambisonic lookup slice uses the pinned `av_channel_layout_channel_from_index`, `av_channel_layout_index_from_channel`, `av_channel_layout_index_from_string`, and `av_channel_layout_channel_from_string` behavior for the bounded explicit ambisonic surface: ambisonic ACNs map to the leading indexes, native extra-mask channels follow in mask-bit order, canonical strings map through the same lookup, and absent/custom-name/invalid lookups fail without producing channels. Broader `AV_CHANNEL_ORDER_AMBISONIC` retyping and oracle-vector calibration remain pending.

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

The packet side-data name boundary slice was checked against pinned FFmpeg 8.1.1 `libavcodec/packet.h` and `av_packet_side_data_name()`. The oracle row records that invalid enum value `-1`, sentinel `AV_PKT_DATA_NB`, `AV_PKT_DATA_NB + 1`, and `INT_MAX` return NULL; the Rust model exposes the same bounded surface through `PacketSideDataKind::from_ffmpeg_value()` and `ffmpeg_side_data_name_for_value()`.
