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

FATE samples are expected to be obtained using upstream FFmpeg's documented `make fate-rsync` flow against a local samples directory. Samples are intentionally ignored by git under `third_party/`; install them locally before claiming sample-backed FATE rows.

The current workspace has the first required sample-backed WAV seed installed at:

```text
third_party/fate-samples/audio-reference/luckynight_2ch_44kHz_s16.wav
```

That one-file seed can be refreshed without downloading the full suite:

```sh
mkdir -p third_party/fate-samples/audio-reference
rsync -vrltLW --timeout=60 rsync://fate-suite.ffmpeg.org/fate-suite/audio-reference/luckynight_2ch_44kHz_s16.wav third_party/fate-samples/audio-reference/
```

The full suite is still needed as more upstream FATE mappings are added.

## Fuzz Execution

Local fuzz execution currently works through WSL Ubuntu with Rust nightly and `cargo-fuzz` installed:

```sh
wsl -d Ubuntu --exec bash -lc "cd /mnt/c/Users/trevo/code/ffmpegrust && . /home/trevo/.cargo/env && cargo fuzz run avutil_core_models -- -runs=1"
```

The same form is used for `avutil_bitreader`, `avutil_byteio`, and `avutil_metadata_options`. Windows-side `cargo-fuzz` is installed, but MSVC ASan runtime/path behavior has been unreliable in this workspace, so WSL is the current recorded fuzz-evidence path.

The latest `avutil_core_models` deterministic logging fixtures include the UTC+05:30 default-callback timestamp row, the POSIX Pacific DST spring-forward/fall-back timestamp rows, the wrapped AEST/AEDT southern-hemisphere DST timestamp rows, the context-sensitive default-callback repeat row where identical text from distinct AVClass instances is not coalesced, the default-callback carriage-return prefix reset row, low-level `av_log_format_line()` / `av_log_format_line2()` carriage-return prefix reset rows, low-level `AV_LOG_QUIET` prefix-suppression rows including the no-prefix `av_log_format_line2(NULL, AV_LOG_QUIET, PRINT_LEVEL)` row, forced-color default-callback `AV_LOG_QUIET` message coloring, forced-color default-callback carriage-return prefix reset, presence-only empty/zero force-color and force-no-color environment rows, no-force pseudo-terminal TERM palette rows including `TERM=dumb` BASIC quiet-message coloring and `TERM=xterm-256color` severity colors, and custom-callback raw delivered levels `-1`, `23`, and `57` with quiet/context delivery. The latest recorded WSL smoke evidence includes 1024 inputs after the UTC+05:30 timestamp addition, a warmed one-input run after adding the context-sensitive repeat fixture, a warmed one-input run after adding the default-callback carriage-return reset fixture, a dedicated one-input run after adding the low-level format-line carriage-return reset fixture, a dedicated one-input run after adding the low-level quiet prefix-suppression fixture, a dedicated one-input run after adding the raw custom-callback delivered-level fixture, a warmed one-input run after adding forced-color quiet rows, a warmed one-input run after adding forced-color carriage-return rows, a warmed one-input run after adding empty/zero force-env rows, a warmed one-input run after adding no-force TTY TERM palette rows, a warmed one-input run after adding no-force `TERM=xterm-256color` severity rows, a warmed one-input run after adding custom-callback raw negative/high-level plus quiet-context rows, a warmed one-input run after adding Pacific POSIX DST rows, and a warmed one-input run after adding wrapped AEST/AEDT DST rows; those sanitizer-backed smokes are useful, but not a sustained fuzz campaign.

The latest packet display-rotation boundary fixture extends
`avutil_core_models` with fractional, near-boundary, and wraparound
`PacketDisplayMatrix::from_clockwise_rotation_degrees` cases. The pinned
libavutil packet oracle emits the matching `packet:display-rotation-boundaries`
row, proving fixed-point matrix construction and rounded counterclockwise
rotation extraction for the selected boundary angles. A bounded one-input WSL
cargo-fuzz smoke for this slice timed out before libFuzzer completion output,
so the deterministic fixture is recorded with build/clippy coverage until a
later warmed smoke can pass.

The latest packet empty zero-growth fixture extends `avutil_core_models` with
`Packet::default().grow_data(0)`. The pinned libavcodec packet oracle emits the
matching `packet:payload-grow-empty-zero*` rows, proving
`av_grow_packet(pkt, 0)` succeeds for a default empty packet by installing
writable refcounted storage with zero visible bytes and zeroed input padding.

The latest packet raw no-buffer release fixture extends `avutil_core_models`
with `Packet::unref()` and `Option<Packet>::take()` over external read-only
caller storage at a nonzero visible offset. The pinned libavcodec packet oracle
emits matching `packet:payload-unref-unrefcounted-offset*` and
`packet:payload-free-unrefcounted-offset` rows, proving FFmpeg resets/nulls the
packet while leaving caller storage unchanged when `pkt->buf == NULL`.

The latest buffer zero-size custom-pool fixture extends `avutil_core_models`
with `BufferPool::with_callbacks(0, 0, ...)` over an exact-shape custom
allocation. The pinned libavutil buffer oracle emits matching
`pool-zero-custom:*` rows, proving first checkout and spare reuse preserve
per-buffer pool opaque data, only one custom allocation is made, ordinary unref
runs no callbacks, and final pool uninit releases the zero-size allocation
before pool_free. A bounded one-input WSL cargo-fuzz smoke for this slice timed
out before libFuzzer completion output, so the deterministic fixture is recorded
with build/clippy coverage until a later warmed smoke can pass.

The latest buffer zero-size default `init2` fixture adds
`pool-init2-zero-default:*` rows. The pinned libavutil oracle proves
`av_buffer_pool_init2(0, opaque, NULL, pool_free)` uses the default allocator
for empty buffers, exposes NULL per-buffer pool opaque data, reuses the empty
spare, and runs the owner pool_free callback once at pool uninit.

The latest buffer non-null zero-size owner fixture adds
`buffer:create-zero-ref-*`, `buffer:replace-zero-null-dst-*`, and READONLY
variants. The pinned libavutil oracle proves `av_buffer_create(data, 0, free,
opaque, flags)` preserves a non-NULL `AVBufferRef.data` pointer through
`av_buffer_ref()` and `av_buffer_replace()` into a NULL destination, while
sharing storage, preserving opaque lookup, and delaying the release callback
until the final source unref.

The latest buffer offset-pool same-size realloc fixture extends
`avutil_core_models` with a first checkout whose visible range starts inside a
custom-pool backing allocation. The pinned libavutil buffer oracle emits
matching `pool-offset:realloc-same*` rows, proving same-size
`av_buffer_realloc()` returns 0, preserves the visible data pointer, size,
writability, and pool opaque lookup, then returns the original backing
allocation for normalized spare reuse after ordinary unref. A 64-run WSL
`avutil_core_models` smoke passed with local leak detection disabled.

The latest buffer make-writable API slice keeps the existing pinned
`buffer:make-writable-*` rows but generates the Rust expected values through
`BufferRef::make_writable`, the fallible helper that mirrors
`av_buffer_make_writable()` success and allocation-error return shape. The
deterministic `avutil_core_models` fixture now covers unique no-op and shared
copy-on-write behavior through that helper.

The latest packet raw-flag fixture extends `avutil_core_models` with the
deterministic `PacketFlags::from_bits_retain` lifecycle: copy-props, ref, clone,
and move preserve unknown raw `AVPacket.flags` bits, while move resets the
source packet flags. The existing pinned libavcodec packet oracle emits the
matching `packet:flags-unknown-*` rows, so this fixture is evidence
strengthening for the Rust model rather than new oracle behavior.

The latest packet refcounted-unpadded fixture extends `avutil_core_models` with
the deterministic split between raw no-buffer payload storage and an existing
refcounted `AVBufferRef` with no logical trailing input-padding capacity.
`Packet::with_buffer` now preserves the pointer and zero padding length through
`make_refcounted()` and unique-writable `make_writable()`, while
`Packet::with_raw_buffer` continues to copy raw no-buffer offset payloads into
zero-offset padded storage. The pinned libavcodec oracle emits matching
`packet:payload-make-refcounted-refcounted-unpadded-*` and
`packet:payload-make-writable-refcounted-unpadded-*` rows. The WSL
`avutil_core_models -- -runs=1` smoke for this slice rebuilt and then hung at
the cargo-fuzz sandbox/tooling layer without usable completion output; unit,
oracle, FATE-runner, and clippy gates cover the slice until a later warmed fuzz
smoke can be recorded.

The latest packet signed-size fixture extends `avutil_core_models` with
negative FFmpeg `int` payload-size boundaries. The pinned libavcodec packet
oracle emits matching `packet:payload-new-packet-negative-*` and
`packet:payload-grow-negative-*` rows: `av_new_packet(pkt, -1)` returns EINVAL
without mutating a populated packet, while `av_grow_packet(pkt, -1)` on a
nonempty packet returns ENOMEM without mutating packet fields, payload bytes,
input padding, or writability. `packet:payload-grow-empty-negative-*` proves
the same ENOMEM return without mutating an empty packet. A 64-run WSL
`avutil_core_models` smoke passed with local leak detection disabled.

The latest packet signed-stream-index fixture extends `avutil_core_models` with
negative `AVPacket.stream_index` behavior. `Packet::stream_index_raw` preserves
the signed field for oracle parity, while `Packet::stream_index()` remains a
nonnegative helper for safe stream-array indexing. `Packet::rescale_ts` leaves
the raw stream index untouched, and copy/ref/clone/move lifecycles carry the
signed field. The pinned libavcodec oracle emits matching
`packet:rescale-negative-stream-index`,
`packet:copy-props-negative-stream-index`,
`packet:ref-negative-stream-index`, `packet:clone-negative-stream-index`, and
`packet:move-negative-stream-index-*` rows. A WSL one-input cargo-fuzz smoke
compiled the fuzz package and then left the tool session stuck without a
reported target result, so check/clippy coverage is recorded for the
deterministic fixture until a later clean smoke can run.

The latest packet rescale-boundary fixture extends `avutil_core_models` with
`Packet::rescale_ts_ffmpeg`, a separate C-shaped helper for the void
`av_packet_rescale_ts()` behavior. The pinned libavcodec oracle emits
`packet:rescale-overflow`, `packet:rescale-invalid-time-base`, and
`packet:rescale-invalid-dst-time-base`, proving present PTS/DTS and positive
duration fields become `AV_NOPTS_VALUE` when `av_rescale_q()` overflows or
receives an invalid source time base, while a `{1,0}` destination time base
zeroes those present fields. The
fallible `Packet::rescale_ts` API remains a typed no-mutation error surface.
The corpus now includes `packet_rescale_sentinel`, and a bounded WSL 64-run
`avutil_core_models` smoke passed with local leak detection disabled against a
temporary corpus copy after the default LeakSanitizer pass reported the local
ptrace incompatibility.

The latest duration-only rescale row extends the ordinary `Packet::rescale_ts`
fixture with `packet:rescale-duration-only`: FFmpeg rescales positive duration
even when PTS and DTS are both `AV_NOPTS_VALUE`, while leaving payload,
position, flags, stream index, and raw packet time base unchanged.

The latest packet signed-position fixture extends `avutil_core_models` with
negative `AVPacket.pos` behavior. `Packet::set_pos` accepts signed values such
as `-2` while preserving `-1` as the public unknown-position sentinel,
`Packet::rescale_ts` leaves `pos` untouched, and copy/ref/clone/move lifecycles
carry the signed position field. The pinned libavcodec oracle emits matching
`packet:rescale-negative-pos`, `packet:copy-props-negative-pos`,
`packet:ref-negative-pos`, `packet:clone-negative-pos`, and
`packet:move-negative-pos-*` rows. A WSL one-input cargo-fuzz smoke executed
the corpus but failed after completion in LeakSanitizer under ptrace, so
clippy/build coverage is recorded for the deterministic fixture until a later
LSAN-compatible smoke can run.

The latest packet signed-duration fixture extends `avutil_core_models` with
negative `AVPacket.duration` behavior. `Packet::set_duration` accepts signed
values, `Packet::rescale_ts` preserves nonpositive duration while rescaling
valid PTS/DTS, and copy/ref/clone/move lifecycles carry the negative duration
field. The pinned libavcodec oracle emits matching
`packet:rescale-negative-duration`,
`packet:copy-props-negative-duration`, `packet:ref-negative-duration`,
`packet:clone-negative-duration`, and `packet:move-negative-duration-*` rows.
A bounded WSL one-input cargo-fuzz smoke for this slice timed out during
cargo-fuzz build/link before target execution, so clippy/build coverage is
recorded for the deterministic fixture until a later warmed smoke can run.

The latest packet custom-padding fixture extends `avutil_core_models` with the
deterministic `Packet::grow_data(0)` and no-op `Packet::shrink_data()` boundary:
zero growth preserves visible payload and the backing pointer while zeroing
dirty FFmpeg input padding, but exact-size and oversize shrink calls preserve
that dirty padding unchanged. A one-input WSL `avutil_core_models` smoke passed
with local leak detection disabled after the sanitizer rebuild.

The latest packet offset-payload fixture extends `avutil_core_models` with the
deterministic `Packet::grow_data(0)` path where packet data begins at a nonzero
offset inside the backing buffer. It preserves the visible data pointer and
offset while zeroing dirty FFmpeg input padding. A one-input WSL
`avutil_core_models` smoke passed with local leak detection disabled after the
sanitizer rebuild.

The latest packet positive offset-grow fixture extends `avutil_core_models`
with the deterministic `Packet::grow_data(2)` path where enough backing-buffer
capacity already exists after a nonzero packet data offset. It preserves the
visible data pointer and offset, exposes the existing tail bytes as newly
visible payload, and zeroes the new FFmpeg input-padding window. A one-input
WSL `avutil_core_models` smoke passed with local leak detection disabled after
the sanitizer rebuild.

The latest packet offset-shrink fixture extends `avutil_core_models` with the
deterministic `Packet::shrink_data(2)` path where packet data begins at a
nonzero offset inside the backing buffer. It preserves the visible data pointer
and offset, keeps the visible prefix bytes, and zeroes the new FFmpeg
input-padding window over the truncated tail bytes. A one-input WSL
`avutil_core_models` smoke passed with local leak detection disabled after the
sanitizer rebuild.

The latest packet offset-helper fixture extends `avutil_core_models` with
`Packet::make_refcounted()` and `Packet::make_writable()` no-op behavior for a
unique writable offset-backed payload. It preserves the visible data pointer,
offset, payload bytes, writability, and dirty FFmpeg input-padding bytes. A WSL
`avutil_core_models` `-runs=1` smoke passed with local leak detection disabled
after the sanitizer rebuild.

The latest packet offset-clone fixture extends `avutil_core_models` with
`Packet::clone()` behavior for an offset-backed payload. It preserves the
visible data pointer, offset, payload bytes, and dirty FFmpeg input-padding
bytes in both packet references, while both references report non-writable
because they share the backing buffer.

The latest packet offset-ref fixture extends `avutil_core_models` with
`Packet::try_ref_from()` behavior for an offset-backed payload. It preserves the
visible data pointer, offset, payload bytes, and dirty FFmpeg input-padding
bytes in both packet references, while both references report non-writable
because they share the backing buffer.

The latest packet offset-move fixture extends `avutil_core_models` with
`Packet::move_ref_from()` behavior for an offset-backed payload. It moves the
visible data pointer, offset, payload bytes, and dirty FFmpeg input-padding
bytes into the destination, preserves writability because ownership is moved
rather than shared, and resets the source packet to defaults.

The latest packet offset-copy-props fixture extends `avutil_core_models` with
`Packet::copy_props_from()` behavior for an offset-backed destination payload.
It preserves the destination visible data pointer, offset, payload bytes, dirty
FFmpeg input-padding bytes, and writability while copying timestamps, flags,
side data, opaque metadata, `opaque_ref`, stream index, and packet time base
from the source.

The latest packet shared offset make-writable fixture extends
`avutil_core_models` with `Packet::make_writable()` behavior after
`Packet::try_ref_from()` creates a shared ref to an offset-backed payload. It
detaches the destination to a writable zero-offset padded buffer, preserves the
visible bytes and copied packet properties, zeroes destination input padding,
and leaves the source offset plus dirty padding intact after the refcount drops
back to one.

The latest packet offset-unref fixture extends `avutil_core_models` with
`Packet::unref()` behavior for an offset-backed payload. It resets the packet
to default fields and releases the complete backing storage, including prefix
bytes before the visible payload and dirty FFmpeg input-padding bytes after it.

The latest packet raw-offset ref/clone fixture extends `avutil_core_models`
with raw no-buffer `Packet::ref_from()` and `Packet::clone()` copy behavior plus
the safe explicit-owner unpadded offset source path. Ref and clone destinations
preserve visible bytes while detaching into zero-offset padded writable storage.

The latest packet raw-offset resize fixture extends `avutil_core_models` with
raw no-buffer grow/shrink models plus the safe explicit-owner unpadded offset
resize path. Raw no-buffer grow copies visible prefix bytes into zero-offset
padded writable storage; raw no-buffer shrink keeps the caller pointer shape
and zeroes the new padding window, while the safe explicit-owner fixtures
preserve the visible offset and do not require backing allocation address
stability when padding is installed.

The latest packet zero `opaque_ref` fixture extends `avutil_core_models` with
copy-props/ref/clone sharing and move-transfer behavior for an empty
`BufferRef`. The mapped packet oracle emits matching
`packet:opaque-ref-zero-*` rows from pinned libavcodec, proving that FFmpeg
keeps zero-size `AVBufferRef` storage refcounted and non-writable while shared,
then restores writability when move-ref transfers the single owner.

The follow-up packet zero `opaque_ref` helper fixture covers the payload helper
split: `packet:payload-make-refcounted-zero-opaque-*` rows keep the empty user
buffer singly owned while payload storage becomes refcounted, and
`packet:payload-make-writable-zero-opaque-*` rows detach only shared payload
bytes while the empty user buffer remains shared.

The latest packet zero `opaque_ref` resize fixture covers
`av_grow_packet()`/`av_shrink_packet()` preservation:
`packet:payload-grow-zero-opaque-*` and
`packet:payload-shrink-zero-opaque` rows keep the empty user buffer singly
owned while payload bytes are grown, caller-initialized, and then shrunk.

The latest packet zero `opaque_ref` reset fixture covers scoped clearing:
`packet:opaque-ref-zero-free-side` proves `av_packet_free_side_data()` clears
only side data while preserving the empty user buffer, and
`packet:opaque-ref-zero-unref` proves `av_packet_unref()` clears that user
buffer with the rest of the packet reset state.

The latest packet payload reset fixture extends `avutil_core_models` with the deterministic `Packet::alloc_new_packet_payload(0)` reset path for `av_new_packet(pkt, 0)` parity: a pre-populated packet becomes an empty writable padded packet with default metadata, no side data, no opaque pointer metadata, no `opaque_ref`, empty flags, stream index zero, and packet time base zero.

The latest `avutil_core_models` pixel-format fuzz evidence includes a saved-crash replay and warmed 4096-run WSL sanitizer execution after fixing a stale no-byte-stride invariant. The harness now treats absent fixed byte stride as valid for planar formats or the modeled bit-packed single-plane formats (`monow`, `monob`, `rgb4`, `bgr4`, and `uyyvyy411`), and separately recognizes planar GBRA formats as alpha-bearing. This narrows bounded pixel-format fuzz coverage but does not replace full `AVPixFmtDescriptor`, FATE media, conversion, or hardware-device parity.

The latest orchestrated WSL fuzz evidence for the avutil evidence batch includes `avutil_metadata_options -- -runs=1` after adding the `PI` expression-constant option fixture and another fresh-target `avutil_metadata_options -- -runs=1` after adding the `metadata=` empty-assignment invariant, plus a warmed `avutil_core_models -- -runs=1` smoke after the packet/frame/pixel/channel-layout evidence updates. A fresh `avutil_core_models` target directory exceeded the local timeout during sanitizer rebuild, then the warmed rerun completed without a crash artifact.

The latest orchestrated avutil lifecycle smoke first timed out while WSL was still compiling/linking `avutil_core_models`, then the background job finished and a warmed `avutil_core_models -- -runs=1` sanitizer run completed. That warmed run covers the new packet FIFO invalid-operation/no-mutation fixture and the logging callback replacement/default-restore/custom-restore fixture.

The latest direct packet-free WSL fuzz smoke used a fresh `avutil_core_models` target directory and completed after the sanitizer rebuild. That one-input run covers the deterministic `Option<Packet>::take()` / Drop fixture for direct `av_packet_free()` parity: nullable frees are no-ops, freeing one shared owner preserves the other owner, and payload plus `opaque_ref` release callbacks fire after the final owner is dropped.

The latest packet FIFO allocation-flags fixture extends `avutil_core_models`
with `PacketFifo::with_flags(PacketFifoFlags::REF | PacketFifoFlags::USER |
unknown-high-bits)`. The pinned libavcodec rows prove
`av_container_fifo_alloc_avpacket(flags)` ignores those allocation flags:
the FIFO starts empty, and zero operation flags still move a packet through
the queue. A warmed 64-run WSL `avutil_core_models` cargo-fuzz smoke passed
with leak detection disabled after rebuilding the release fuzz target.

The latest packet FIFO nullable-free fixture extends `avutil_core_models` with the safe Rust equivalent of `av_container_fifo_free(&fifo)` for `fifo == NULL` and an empty FIFO: `Option<PacketFifo>::take()` keeps `None` as `None`, drops an empty FIFO owner without queued entries, and leaves no release side effects to observe. A fresh-target WSL one-input sanitizer smoke passed after rebuilding `avutil_core_models`.

The latest packet FIFO partial-drain fixture extends `avutil_core_models` with
mixed move/ref `PacketFifo::drain(1)` release ordering: the drained move entry
releases its payload and `opaque_ref`, the ref-written packet remains queued,
and the ref-source payload releases only after the queued ref is drained and
the original source drops. A default WSL `cargo fuzz run avutil_core_models --
-runs=1` completed corpus execution but failed in LeakSanitizer with the local
`does not work under ptrace` fatal condition; the same warmed target passed
with `LSAN_OPTIONS=detect_leaks=0 ASAN_OPTIONS=detect_leaks=0`, so that rerun
is recorded as target-logic smoke rather than full leak-sanitizer evidence.

The latest packet side-data clear fixture extends `avutil_core_models` with
`Packet::clear_side_data()` empty and repeated-call behavior: clearing an empty
packet remains empty, and repeated clearing after a populated packet preserves
payload, timestamps, flags, opaque pointer metadata, `opaque_ref`, stream
index, and packet time base. A warmed WSL one-input run with local leak
detection disabled passed after rebuilding the changed target.

The latest packet zero-size side-data replacement fixture extends
`avutil_core_models` with packet-owned and standalone `AV_PKT_DATA_NEW_EXTRADATA`
replacement by an empty payload. It mirrors pinned rows where
`av_packet_add_side_data()` and `av_packet_side_data_add()` replace existing
two-byte entries with a zero-size caller-owned entry and keep lookup present
with size zero. A warmed WSL one-input run with local leak detection disabled
passed after rebuilding `avutil_core_models`.

The latest packet nullable-zero side-data fixture extends `avutil_core_models`
with packet-owned and standalone `AV_PKT_DATA_NEW_EXTRADATA` replacement by
NULL caller data at size zero. It mirrors pinned rows where packet-owned
`av_packet_add_side_data(pkt, type, NULL, 0)` stores the side-data entry but
`av_packet_get_side_data()` returns NULL, while standalone
`av_packet_side_data_add(..., NULL, 0, flags)` stores an entry that
`av_packet_side_data_get()` still returns.

The latest packet nullable-zero side-data lifecycle fixture extends
`avutil_core_models` with the copy/ref/clone reallocation split for packet-owned
NULL/0 side data. It mirrors pinned rows where `av_packet_copy_props()`,
`av_packet_ref()`, and `av_packet_clone()` allocate an ordinary zero-size
padded side-data entry that packet-owned lookup returns, while
`av_packet_move_ref()` transfers the original NULL/0 entry and lookup remains
NULL.

The latest packet zero-size side-data lifecycle fixture extends
`avutil_core_models` with `Packet::copy_props_from`, `Packet::ref_from`,
`Packet` clone, and `Packet::move_ref_from` preservation of an empty
`AV_PKT_DATA_NEW_EXTRADATA` record. It mirrors pinned rows where
`av_packet_copy_props()`, `av_packet_ref()`, `av_packet_clone()`, and
`av_packet_move_ref()` keep lookup present with size zero, with move-ref
resetting the source side-data list. A warmed WSL one-input run with local leak
detection disabled passed after rebuilding `avutil_core_models`.

The latest packet zero-size side-data shrink fixture extends
`avutil_core_models` with the no-mutation `av_packet_shrink_side_data()` edge
for a zero-size `AV_PKT_DATA_NEW_EXTRADATA` entry: oversize shrink to one byte
returns ENOMEM and keeps lookup present at size zero. A warmed WSL one-input
run with local leak detection disabled passed after rebuilding
`avutil_core_models`.

The latest packet zero-size side-data payload-helper fixture extends
`avutil_core_models` with `Packet::make_refcounted()` and
`Packet::make_writable()` preservation of an empty
`AV_PKT_DATA_NEW_EXTRADATA` record. It mirrors pinned rows where
`av_packet_make_refcounted()` on a raw-data packet and
`av_packet_make_writable()` on a shared refcounted packet keep lookup present
with size zero. A warmed WSL one-input run with local leak detection disabled
passed after rebuilding `avutil_core_models`.

The latest packet zero-size side-data resize-helper fixture extends
`avutil_core_models` with `Packet::grow_data()` and `Packet::shrink_data()`
preservation of an empty `AV_PKT_DATA_NEW_EXTRADATA` record plus packet
metadata. It mirrors pinned rows where `av_grow_packet()` followed by
deterministic writes to the newly visible bytes and `av_shrink_packet()` keep
lookup present with size zero. A warmed WSL one-input run with local leak
detection disabled passed after rebuilding `avutil_core_models`.

The latest PAL8/buffer lifecycle fixture extends `avutil_core_models` with full PAL8 frame palette side-plane construction, make-writable detachment, copy/crop preservation, and threaded `BufferRef` clone/drop release timing. WSL `cargo fuzz run avutil_core_models -- -runs=1` passed after a long sanitizer rebuild in `target-wsl-pal8-buffer-fuzz-o1`.

The latest orchestrated WSL fuzz evidence for the packet/options/WAV/image2
edge batch includes one-run sanitizer smokes for `avutil_metadata_options`,
`avformat_wav`, and `avformat_image2` after adding the whitespace shorthand,
missing odd-`fmt ` padding, and image2 start-number range seeds. A combined
fresh-target run that also included `avutil_core_models` exceeded the local
timeout while rebuilding sanitizer artifacts; the process was stopped after
confirming no crash artifact, and the Windows fuzz build/clippy checks for
`avutil_core_models` passed.

The latest orchestrated media-edge fuzz fixtures add deterministic seeds for
packet-owned raw numeric side data in `avutil_core_models`, raw `pcm_s16le`
9-channel packetization and packet-size overflow guarding in
`avformat_pcm_s16le`, WAV duplicate-`fmt ` with an unsupported second format in
`avformat_wav`, and YUV4MPEG2 leading whitespace before the first frame marker
in `avformat_yuv4mpegpipe`. Windows-side fuzz build/clippy checks are the
baseline for these targets on this turn; sanitizer-backed WSL `cargo fuzz run`
smokes remain desirable before promoting any affected component to complete.

The latest yuv4mpegpipe parser fuzz smoke ran through WSL with `cargo fuzz run avformat_yuv4mpegpipe -- -runs=1` after adding malformed, duplicate, signed, and zero sample-aspect field fixtures. This records only bounded sanitizer-backed smoke evidence for the current corpus, not a sustained fuzz campaign.

`cargo run -p fate-runner -- list` reads `PORTING_LEDGER.toml` and lists known components. `cargo run -p fate-runner -- status --next 5` reports the current strict completion count/percentage, status counts, and the next priority-sorted incomplete components. `cargo run -p fate-runner -- mappings` reads `tests/fate/mappings.txt` and lists configured component-target commands; add repeated `--component <id>` and/or `--target <name>` filters to narrow the listing to exact component IDs or target names, or add `--check-prereqs` with `--samples <path>` and `--oracle-ffmpeg <path>` to resolve placeholders and validate all listed mapping prerequisites without executing commands. `cargo run -p fate-runner -- run --changed` inspects git changed paths, maps currently covered Rust modules, dependency manifests and lockfile, cargo-fuzz target files, selected fuzz corpus directories, `tests/differential/` files, and the version/loglevel, rawvideo/PCM/dict/options/pixel-format/sample-format/channel-layout/color/WAV oracle integration harnesses to ledger component IDs, and runs explicit command mappings from the selected mapping file for selected components. Explicit runs may pass `--component <id>` more than once to select multiple components in one invocation; duplicate component IDs are coalesced before execution. Repeated `--target <name>` filters narrow run selection to exact target names, and a selected component whose mappings are all filtered out still fails as unmapped rather than silently passing. Add `--dry-run` to resolve and print selected mappings, including prerequisite validation, without executing them. The mapping format is documented in `tests/fate/README.md` as `component_id|target|workdir|program|arg1|arg2|...`.

Mappings may reference `{samples}` and `{oracle_ffmpeg}` in the workdir, program, or args fields. When a selected mapping uses those placeholders, pass `--samples <path>` and/or `--oracle-ffmpeg <path>` to `fate-runner`; the runner validates that the samples path is an existing directory and the oracle path is an existing file before executing the mapped command. If explicit flags are omitted, `fate-runner` tries `FATE_SAMPLES`, `SAMPLES`, and standard local sample directories (`third_party/fate-samples`, `third_party/fate-suite`, `fate-suite`) for `{samples}`, and `FFMPEG_ORACLE` plus the standard local oracle paths under `third_party/ffmpeg-oracle/build/bin/` for `{oracle_ffmpeg}`. Invalid explicit or environment paths fail before execution rather than falling through silently.

Mapping args may also include `env:NAME=value`; `fate-runner` treats those as child-process environment variables rather than command arguments. Placeholder resolution applies to the environment value, so differential mappings can use `env:FFMPEG_ORACLE={oracle_ffmpeg}` to validate and inject the pinned oracle path for ignored Rust oracle tests. Resolved `{samples}` and `{oracle_ffmpeg}` values are expanded to absolute paths before command execution so child tools whose working directory changes, including Cargo test binaries, can still find the oracle and sample tree. Mapping rows are parsed strictly: only `{samples}` and `{oracle_ffmpeg}` placeholders are accepted, malformed `{...}` placeholders are rejected, and duplicate `env:` names in a single row are rejected before any command runs.

The current default mapping file contains `fate-runner|local-self-test`, which validates local runner wiring by invoking `cargo test -p fate-runner`; `oracle-inventory|local-oracle-unit`, which validates inventory command-list and manifest helper coverage through `cargo test -p oracle`; local `avutil-*|local-avutil-unit` rows, which validate the selected shared primitive through focused `cargo test -p avutil` filters; local `avcodec-*|local-avcodec-unit` rows, which validate the selected initial decoder through focused `cargo test -p avcodec` filters; `avformat-mov-demuxer|local-mov-unit`, which validates that the selected MOV demuxer component can drive `cargo test -p avformat mov::tests` through the FATE runner; local `avformat-rawvideo-*|local-avformat-rawvideo-unit` rows, which validate the current rawvideo demuxer and muxer through focused `cargo test -p avformat rawvideo` filters; `fftools-version|local-version-unit`; `fftools-version|local-buildconf-unit`; `fftools-hide-banner|local-hide-banner-unit`; `fftools-option-parser|local-option-parser-unit`; `fftools-option-parser|local-cli-logging-unit`; `fftools-basic-io|local-io-plan-unit`; shared `local-ffmpeg-unit` rows for the current `fftools-ffmpeg-*` ledger components; and shared `local-ffprobe-unit` rows for the current `fftools-ffprobe-*` ledger components. These mappings do not count as upstream FFmpeg FATE media parity.

Local `avformat-*|local-avformat-*-unit` rows validate the current WAV, raw PCM, rawvideo, AVI, yuv4mpegpipe, null, hash, framecrc, framehash, and streamhash unit filters when those format components or their shared fuzz target change.

`tests/fate/upstream-mappings.txt` is the non-default upstream FFmpeg/FATE-style mapping file. API-level rows can run against the pinned FFmpeg source/build cache without media samples; sample-backed rows additionally require `{samples}`. It currently maps `avutil-channel-layout|fate-channel_layout` to an ignored wrapper around upstream FFmpeg's `make fate-channel_layout`, maps `avutil-packet|fate-avpacket` to an ignored wrapper around upstream FFmpeg's `make fate-avpacket`, maps `avutil-frame|fate-side_data_array` to an ignored wrapper around upstream FFmpeg's `make fate-side_data_array`, maps `avutil-options|fate-opt` to an ignored wrapper around upstream FFmpeg's `make fate-opt`, maps `avutil-pixel-format|fate-pixfmt_best` to an ignored wrapper around upstream FFmpeg's `make fate-pixfmt_best`, maps `avutil-logging|upstream-libavutil-log-test` to an ignored wrapper that proves there is no `fate-log` row in pinned `tests/fate/libavutil.mak` while building and running upstream `libavutil/tests/log`, and maps `fate-wav-pcm-s16le-md5` to the sample-specific ignored test in `crates/fftools/tests/wav_oracle.rs` for both `avformat-wav-demuxer` and `avutil-packet`, injecting `FFMPEG_ORACLE={oracle_ffmpeg}` and `FATE_WAV_SAMPLE={samples}/audio-reference/luckynight_2ch_44kHz_s16.wav`. List or dry-run it explicitly:

```sh
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target fate-wav-pcm-s16le-md5
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target fate-channel_layout
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target fate-avpacket
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target fate-side_data_array
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target fate-opt
cargo run -p fate-runner -- mappings --mappings tests/fate/upstream-mappings.txt --target upstream-libavutil-log-test
cargo run -p fate-runner -- run --dry-run --mappings tests/fate/upstream-mappings.txt --component avformat-wav-demuxer --target fate-wav-pcm-s16le-md5 --samples <fate-samples> --oracle-ffmpeg <ffmpeg>
cargo run -p fate-runner -- run --dry-run --mappings tests/fate/upstream-mappings.txt --component avutil-packet --target fate-wav-pcm-s16le-md5 --samples <fate-samples> --oracle-ffmpeg <ffmpeg>
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-channel-layout --target fate-channel_layout
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-packet --target fate-avpacket
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-frame --target fate-side_data_array
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-options --target fate-opt
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-logging --target upstream-libavutil-log-test
```

These mappings are intentionally outside the default local smoke file so ordinary component runs do not require downloaded samples or an upstream build cache. The channel-layout row has passed locally with the pinned WSL source/build cache created by `scripts/bootstrap_ffmpeg_oracle_wsl.sh`, satisfying the upstream FATE target for that API surface while leaving broader parser/retype/ambisonic and sustained fuzz evidence incomplete. The packet API row has passed locally with the same pinned source/build cache, satisfying FFmpeg's upstream `fate-avpacket` API target while leaving remaining AVPacket ABI/media-integration vectors incomplete. The frame side-data array row has passed locally with the same pinned source/build cache, satisfying FFmpeg's upstream `fate-side_data_array` target for the public `AVFrameSideData` array helper surface and advancing `avutil-frame` to `fate_pass` while broader AVFrame behavior remains incomplete. The AVOption row has passed locally with the same pinned source/build cache, satisfying FFmpeg's upstream `fate-opt` target while leaving broader AVOption API vectors and CLI option-ordering integration incomplete. The pixel-format row has passed locally with the same pinned source/build cache, satisfying FFmpeg's upstream `fate-pixfmt_best` target for the bounded best-pixel-format helper surface while leaving full descriptor inventory, conversion, hardware, and sustained fuzz closure incomplete. The logging row has passed locally with the same pinned source/build cache; FFmpeg 8.1.1 ships `libavutil/tests/log` as an upstream test program but does not wire it into `tests/fate/libavutil.mak`, so the row records FATE inapplicability plus upstream test-program execution while leaving callback/stderr and fuzz closure incomplete. The WAV row has passed locally with the pinned WSL oracle wrapper and the one-file WAV sample seed, advancing `avformat-wav-demuxer` to `fate_pass` and adding sample-backed media evidence for `avutil-packet` while leaving broader WAV coverage, actual fuzz execution, and full upstream FATE coverage incomplete.

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

`crates/avutil/tests/logging_oracle.rs` is an ignored oracle harness for the bounded libavutil logging surface. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares the `AV_LOG_*` numeric levels, `AV_LOG_MAX_OFFSET`, `AV_LOG_SKIP_REPEATED`, `AV_LOG_PRINT_LEVEL`, `AV_LOG_PRINT_TIME`, `AV_LOG_PRINT_DATETIME`, default `av_log_get_level()`, known `av_log_set_level()` round trips, known `av_log_set_flags()` round trips, bounded `av_log_format_line2(NULL, ...)` prefix/truncation rows, normalized `av_log_format_line2(non-NULL, ...)` AVClass context-prefix rows, bounded `av_log_format_line(NULL/non-NULL, ...)` no-return buffer rows, ignored `AV_LOG_PRINT_TIME`/`AV_LOG_PRINT_DATETIME` low-level formatter flag rows, normalized `av_log_default_callback(NULL/non-NULL, ...)` timestamp and AVClass context-prefix rows, byte-identical fixed and POSIX-DST `av_gettime()`/`TZ` default-callback local-time rows, bounded default-callback `AV_LOG_SKIP_REPEATED` summary rows including normalized AVClass-context and forced-color repeat rows, bounded `AV_LOG_FORCE_COLOR` default-callback rows including quiet-message color behavior and carriage-return prefix reset, no-force `TERM=dumb` BASIC quiet rows, no-force `TERM=xterm-256color` severity rows, first-use default-callback color-cache rows, fresh-process no-force redirected-stderr no-color rows, fresh-process no-force pseudo-terminal color rows, fresh-process no-color precedence rows including empty-string and `0` force-env values, plain and forced-color partial-line default-callback `print_prefix` continuation rows, bounded custom `av_log_set_callback` dispatch rows including raw non-enum delivered levels `-1`, `23`, and `57`, multi-argument `va_list` formatted text, NULL fallback, AVClass item-name capture, and `AV_LOG_QUIET` context delivery, plus `av_log_once()` initial/subsequent-level state behavior against the Rust `LogLevel`, `LogFlags`, `AvLogFormatLine`, `AvLogFormatLine2`, `AvLogContextPrefix`, `LogTimestamp`, `LogColorMode`, `DefaultCallbackColorState`, `DefaultCallbackPrefixState`, `LogFormatOptions`, `LogDefaultCallbackTimeZone`, `PosixDstTransition`, `LogRecord::raw_level`, `LogOnceState`, `Logger::log_custom_callback`, and `Logger::log_once` model. The same file also has an ignored upstream-disposition wrapper that audits pinned `tests/fate/libavutil.mak` for the absence of a `fate-log` target while building and running upstream `libavutil/tests/log`. The `av_log_format_line2()` rows pin `PRINT_LEVEL` prefix insertion, quiet-level suppression of the optional level prefix, context prefix order before optional level prefixes, no-prefix suppression, persistent `print_prefix` transitions, newline and carriage-return reset behavior, full would-write return lengths for the stable NULL-context rows, and truncation for `line_size` 8, 1, and 0 with a null output pointer. The `av_log_format_line()` rows pin the same supported copied buffer and persistent `print_prefix` transitions without relying on a returned full length, including `AV_LOG_QUIET` rows and carriage-return reset rows for NULL and normalized AVClass contexts. The low-level time/datetime flag rows prove those flags do not add prefixes in the format-line helpers; the default-callback rows separately normalize the dynamic current timestamp and process-specific `0x...` address and prove unbracketed millisecond-precision `PRINT_TIME`/`PRINT_DATETIME` prefixes, with output ordered as timestamp, AVClass context prefix, optional level prefix, then message. The fixed-time rows override FFmpeg's `av_gettime()` inside the oracle helper and set deterministic `TZ` values, proving byte-identical local-time output for UTC+2, UTC+5:30, UTC-8 date-crossing, `TZ=PST8PDT,M3.2.0,M11.1.0` Pacific DST boundary rows, and `TZ=AEST-10AEDT,M10.1.0,M4.1.0/3` wrapped southern-hemisphere boundary rows. The default-callback repeat rows prove duplicate suppression prints the first line once, flushes `    Last message repeated N times\n` without a level prefix before the next different line, prints every duplicate when `AV_LOG_SKIP_REPEATED` is not set, and preserves that same plain summary shape for normalized AVClass contexts and forced-color output where adjacent warning/error segments are colored. The partial-line rows prove default callback suppresses timestamp/context/level prefixes after a message without trailing newline, then restores prefixing after the newline tail for NULL and normalized AVClass contexts; the carriage-return rows prove both the low-level format-line helpers and default callback treat copied messages ending in `\r` as a prefix reset. The forced-color rows run the same oracle binary in a fresh `--color` process because FFmpeg caches default-callback color mode; those rows prove context prefixes use `38;5;250` on black, warning level/message segments use `38;5;226` on black, error message segments use `38;5;196` on black, info output stays uncolored, quiet messages still suppress time/level prefixes but use `38;5;196` on `48;5;52`, forced-color partial-line continuation keeps warning message fragments colored while suppressing context/level prefixes until newline reset, and forced-color carriage-return reset restores colored context/level prefixes for the next NULL or normalized AVClass record. The cache/no-color rows prove a first no-color default-callback decision stays cached even after `AV_LOG_FORCE_COLOR` is set later in the same process, a fresh process with no force environment and redirected stderr stays uncolored, fresh processes with no force environment and pseudo-terminal stderr use basic ANSI rows for `TERM=dumb`/empty `TERM`, including BASIC fatal-color output for quiet messages under `TERM=dumb`, and the 256-color warning/error/fatal/panic palette for `TERM=xterm-256color`, a fresh process with both `AV_LOG_FORCE_NOCOLOR` and `AV_LOG_FORCE_COLOR` starts uncolored, and empty-string or `0` values still count as set for both force-color and no-color precedence. The custom-callback rows prove a custom callback is invoked even when the configured av_log level would hide a message from the default callback, bypasses repeat suppression for the bounded rows, receives raw delivered integer levels below and above named thresholds, preserves `va_list` formatting, and reports NULL or AVClass item context; the newest rows prove raw levels `-1` and `57` plus quiet-context delivery. The `av_log_once()` rows pin zero-state first-call initial-level logging, subsequent-call subsequent-level logging, and preseeded nonzero state resetting to raw state `1`. It is wired into `tests/differential/mappings.txt` as `avutil-logging|oracle-libavutil-logging` and into `tests/fate/upstream-mappings.txt` as `avutil-logging|upstream-libavutil-log-test`:

The default-callback context-switch repeat row logs identical warning text through two distinct normalized AVClass instances, then a different error, proving FFmpeg compares the emitted context-bearing line rather than message text alone.

The newest local-time rows add `TZ=AEST-10AEDT,M10.1.0,M4.1.0/3` `PRINT_DATETIME|PRINT_LEVEL` default-callback cases around southern-hemisphere spring-forward and fall-back boundaries, proving the bounded POSIX DST formatter handles wrapped daylight intervals in addition to the fixed UTC+2, UTC+5:30, UTC-8, and Pacific DST rows.

The newest custom-callback rows specifically include duplicate NULL-context and AVClass-context messages under `AV_LOG_SKIP_REPEATED`, raw non-enum delivered levels `-1`, `23`, and `57`, a multi-argument formatted callback payload, and an `AV_LOG_QUIET` AVClass-context row, proving both duplicate invocations are delivered to the custom callback and unknown levels plus quiet/context metadata are preserved for the bounded surface.

The newest callback lifecycle rows prove `av_log_set_callback()` replacement by observing dispatch counts and messages: the first callback receives the first row, the replacement callback receives the second row, restoring `av_log_default_callback` stops custom callback delivery, and installing a custom callback again resumes delivery. Rust mirrors this for instance loggers and the global logger, including record clear, callback clear, and reset behavior.

The newest default-callback threshold rows prove `AV_LOG_INFO` is suppressed at an `AV_LOG_WARNING` threshold, `AV_LOG_WARNING` is emitted at that threshold, and `AV_LOG_QUIET` records are emitted at an `AV_LOG_QUIET` threshold while suppressing optional time/level prefixes and preserving AVClass context prefixes.

The newest low-level quiet row proves `av_log_format_line2(NULL, AV_LOG_QUIET, AV_LOG_PRINT_LEVEL)` with `print_prefix=0` emits the plain message, returns the full message length, and keeps `print_prefix` false instead of manufacturing a level prefix.

The newest raw-flag rows prove `av_log_set_flags()` stores raw flag integers without truncating unknown bits: isolated unknown bits, mixed known/unknown bits, and raw `-1` round-trip through `av_log_get_flags()`.

The newest raw-level rows prove `av_log_set_level()` stores arbitrary integers, including `-1`, `23`, and `57`, and that default-callback threshold filtering uses raw integer comparison for levels between named constants.

The newest CLI loglevel rows prove pinned `ffmpeg` and `ffprobe` accept raw numeric `-loglevel` thresholds on version requests, including `23`, `57`, `-1`, `999`, `repeat+23`, `level+23`, `+23`, `time+23`, `repeat+level+23`, and `-repeat+23`.

Local `fftools` diagnostic stderr unit coverage now applies the pinned default-callback repeat-summary indentation shape to process-level repeated diagnostics: compressed rows emit `    Last message repeated N times` while non-repeating paths preserve duplicate records. The newest pinned `ffmpeg` CLI row uses a two-file invalid MJPEG image2 sequence to prove real default CLI stderr emits the indented repeat summary, and that `-loglevel repeat+error` prints duplicate diagnostics without a summary.

The newest default-callback forced-color severity rows add fatal and panic `PRINT_LEVEL` cases. Pinned FFmpeg 8.1.1 uses warning `38;5;226` on black, error `38;5;196` on black, fatal `38;5;208` on black, panic `38;5;196` on `48;5;52`, and leaves info uncolored.

The default-callback repeat/color rows also add normalized AVClass-context repeated diagnostics, context-switch diagnostics, forced-color repeated diagnostics, and forced-color quiet diagnostics, proving distinct AVClass instances with identical warning text are not coalesced, warning/error segments colorize while the repeat summary stays the plain four-space line, and quiet messages use panic color while still suppressing optional time/level prefixes.

The newest no-force pseudo-terminal TERM rows run as fresh oracle processes because FFmpeg caches the first default-callback color decision. They prove unset `TERM` produces plain warning output, `TERM=dumb` produces basic ANSI context (`0;39m`), warning (`0;33m`), error (`1;31m`), and fatal/panic (`4;31m`) segments while info stays plain, empty `TERM` and `TERM=xterm-color` produce the same basic warning row, and `TERM=xterm-256color` produces 256-color context, warning, error, fatal, and panic segments while info stays plain.

```sh
cargo test -p avutil --test logging_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-logging --target oracle-libavutil-logging --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-logging --target upstream-libavutil-log-test
```

`crates/avutil/tests/options_oracle.rs` is an ignored oracle harness for the bounded libavutil AVOption surface. It compiles a test-only C `AVClass` against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares public `AV_OPT_FLAG_*` bit values, public `AV_OPT_TYPE_*` numeric values, public `AV_OPT_SEARCH_*` bit values, public `AV_OPT_SERIALIZE_*` bit values, `av_opt_next` source-order iteration, bounded `av_opt_find`/`av_opt_find2` exact name/unit/flag matching for root definitions, unit-scoped constants, and direct child targets, `av_opt_set_defaults`, `av_opt_get` string retrieval for default, post-set, and `AV_OPT_SEARCH_CHILDREN` bool/int64/double/rational/string/int64-constant values, typed `av_opt_set_int`, `av_opt_set_double`, `av_opt_set_q`, `av_opt_get_int`, `av_opt_get_double`, and `av_opt_get_q` rows for success, range, read-only, string-error, child-search, and `AV_OPT_SEARCH_FAKE_OBJ` behavior, `av_opt_query_ranges(..., flags=0)` default range rows for bool/int64/double/rational/string/int64-constant-backed/read-only options and the default missing-option `ENOMEM` row, `av_opt_set` string parsing for root and direct-child values plus bounded numeric expression rows covering whitespace-stripped arithmetic, SI suffixes, unit constants inside expressions, exact rational literals, invalid-expression `EINVAL`, and out-of-range `ERANGE`, exact option-name rejection, exact constant-name rejection, missing-option `AVERROR_OPTION_NOT_FOUND` codes, invalid bool rejection, `AV_OPT_FLAG_READONLY` rejection, `AV_OPT_SEARCH_FAKE_OBJ` get/set not-found behavior, bounded `av_opt_set_dict2` root, child-search, duplicate-unknown, and hard-error dictionary-preservation rows, bounded `av_opt_set_from_string` named, shorthand, after-named parse-error, set-error, option-not-found, no-shorthand, escaped-token, protected-whitespace, and quoted-token rows, bounded `av_opt_serialize` default, exact-filter, child-search, skip-default, escaping, invalid-separator, and read-only default-state rows, and bounded `av_opt_copy` same-class root, child-object, non-recursive child-state, deep-clone, and class-mismatch rows against the Rust `OptionSet` model. The compared root state intentionally covers writable fields only because pinned FFmpeg does not initialize READONLY storage through `av_opt_set_defaults`; serialization rows separately pin that read-only current-storage/default distinction. The same file also wraps upstream FFmpeg's `fate-opt` target from `tests/fate/libavutil.mak`, which runs `libavutil/tests/opt` through the pinned source/build cache. It is wired into `tests/differential/mappings.txt` as `avutil-options|oracle-libavutil-options` and into `tests/fate/upstream-mappings.txt` as `avutil-options|fate-opt`:

The latest set-from-string rows add empty-key, unclosed-quote, quote-escape, protected-whitespace, and empty-pair-separator multi-token coverage: `=7` returns `AVERROR_OPTION_NOT_FOUND` without mutation, `metadata='title` succeeds with `title`, `metadata='\\''x'` stores `\\x`, `metadata=title\ :threads=16` preserves `title ` while applying `threads=16`, and `pairs_sep=""` with `threads=7:quality=0.25` returns `EINVAL` without mutating the option state.

The same harness now has separate bounded `AV_OPT_TYPE_DURATION`, `AV_OPT_TYPE_IMAGE_SIZE`, `AV_OPT_TYPE_PIXEL_FMT`, `AV_OPT_TYPE_SAMPLE_FMT`, `AV_OPT_TYPE_CHLAYOUT`, `AV_OPT_TYPE_BINARY`, `AV_OPT_TYPE_DICT`, `AV_OPT_TYPE_FLAG_ARRAY`, `AV_OPT_TYPE_VIDEO_RATE`, and `AV_OPT_TYPE_COLOR` classes. It also pins `AV_OPT_ALLOW_NULL` and `AV_OPT_ARRAY_REPLACE` search flag bits plus nullable string, binary, and dictionary `av_opt_get` rows for NULL storage with and without `AV_OPT_ALLOW_NULL`, non-null storage with the flag, and `AV_OPT_SEARCH_FAKE_OBJ | AV_OPT_ALLOW_NULL`. The duration class validates duration string parsing for seconds, `MM:SS`, `HH:MM:SS`, fractional microseconds, `ms`/`us` suffixes, parse `EINVAL`, range `ERANGE`, `av_opt_get` string formatting, typed int/double/q getters, and default `av_opt_query_ranges` rows against pinned FFmpeg 8.1.1. The image-size class validates default parsing, numeric pair strings, named video-size abbreviations, `none` as `0x0`, typed `av_opt_set_image_size` / `av_opt_get_image_size`, direct-child typed get/set targeting under `AV_OPT_SEARCH_CHILDREN`, child-only not-found behavior without child search, `AV_OPT_SEARCH_FAKE_OBJ` not-found/no-mutation behavior, negative typed `EINVAL`, wrong-type `EINVAL`, generic numeric ERANGE/EINVAL behavior, `av_opt_get` string formatting, and default image-size range rows. The pixel-format class validates default storage, named format strings, `none`, C base-0 numeric strings, invalid name `EINVAL` including the string form of `AV_PIX_FMT_NB`, typed numeric range `ERANGE`, typed `av_opt_set_pixel_fmt` / `av_opt_get_pixel_fmt`, generic numeric getters/setters, `av_opt_get` name formatting, and declared enum range rows for the bounded `AV_PIX_FMT_NONE` through `AV_PIX_FMT_NB - 1` surface with high modeled software rows such as `gbrap32le` and `yuv444p10msble`. The sample-format class validates default storage, named format strings, `none`, C base-0 numeric strings, invalid name `EINVAL`, string numeric `AV_SAMPLE_FMT_NB` `EINVAL`, typed `av_opt_set_sample_fmt` / `av_opt_get_sample_fmt`, generic numeric getters/setters, `av_opt_get` name formatting, and declared enum range rows for `AV_SAMPLE_FMT_NONE` through `AV_SAMPLE_FMT_NB - 1`. The channel-layout class validates default storage, native name strings, `2C` count-only strings, invalid string `EINVAL` destination reset to `0 channels`, typed `av_opt_set_chlayout` / `av_opt_get_chlayout`, generic numeric ERANGE/EINVAL behavior, typed numeric getter `EINVAL` rows, `av_opt_get` channel-layout descriptions, and `av_opt_query_ranges` `ENOSYS` behavior. The binary class validates default and null hex-string storage, even-length hex string parsing, empty-string clearing, invalid odd-length and non-hex string `EINVAL` reset to empty payloads, typed `av_opt_set_bin` copy/clear behavior, generic numeric ERANGE/EINVAL behavior, numeric getter `EINVAL`, `av_opt_get` uppercase-hex formatting, `AV_OPT_ALLOW_NULL` NULL-output behavior, and `av_opt_query_ranges` `ENOSYS` behavior. The dictionary class validates default and null storage, escaped key/value formatting, string parse/clear behavior, quoted and escaped tokens, case-insensitive duplicate-key replacement with later key spelling such as `artist=rust:ARTIST=override`, invalid parse preservation, typed `av_opt_set_dict_val` / `av_opt_get_dict_val`, generic numeric ERANGE/EINVAL behavior, numeric getter `EINVAL`, `AV_OPT_ALLOW_NULL` NULL-output behavior, and `av_opt_query_ranges` `ENOSYS` behavior. The array class validates default string parsing, escaped separator/backslash formatting, `av_opt_get_array_size`, int64, string, double, and rational `av_opt_get_array` rows, string/double/rational typed int array conversion rows, `av_opt_set_array` insert/replace/remove behavior for int64 and string arrays, zero-count set/remove no-ops at the array end, zero-count get `EINVAL` boundaries at and beyond the array end, wrong-type/range errors, string-set max preservation, minimum-length clearing, numeric getter `EINVAL`, and default `av_opt_query_ranges` `ENOSYS` behavior. The video-rate class validates named rate abbreviations, ratio/expression strings, nonpositive `EINVAL`, range `ERANGE`, typed `av_opt_set_video_rate`, generic numeric setters, FFmpeg's `av_opt_get_video_rate` delegation behavior, `av_opt_get` string formatting, and default video-rate range rows. The color class validates default parsing, deterministic named/hex/alpha strings, FFmpeg's decimal-alpha truncation, bad-name and bad-alpha `EINVAL` rows with partial destination mutation, generic numeric ERANGE/EINVAL behavior, typed getter `EINVAL` rows, `av_opt_get` `0xRRGGBBAA` formatting, and default color range rows.

```sh
cargo test -p avutil --test options_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-options --target oracle-libavutil-options --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-options --target fate-opt
```

`crates/avutil/tests/packet_oracle.rs` is an ignored oracle harness for libavcodec `AVPacket` core lifecycle helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavcodec.a`, `libswresample.a`, and `libavutil.a`, then compares `AV_PKT_DATA_NB`, every public `AV_PKT_DATA_*` numeric value, every public `av_packet_side_data_name()` display string plus out-of-range NULL rows including `INT_MIN`, exact public `AV_PKT_FLAG_KEY`, `AV_PKT_FLAG_CORRUPT`, `AV_PKT_FLAG_DISCARD`, `AV_PKT_FLAG_TRUSTED`, and `AV_PKT_FLAG_DISPOSABLE` numeric values, `AV_PICTURE_TYPE_*` numeric values plus `av_get_picture_type_char()` output, default-native public ABI layout rows for `AVPacketSideData`, `AVPacket`, and deprecated `AVPacketList` using C `sizeof`, `_Alignof`, and `offsetof`, public C-struct/enum side-data payload layouts for `AVReplayGain`, `AVContentLightMetadata`, `AVMasteringDisplayMetadata`, `AVAmbientViewingEnvironment`, `AV3DReferenceDisplaysInfo` plus `AV3DReferenceDisplay`, `AVSphericalMapping`, `AVStereo3D`, `AVCPBProperties`, `AVProducerReferenceTime`, `AVRTCPSenderReport`, `AVDOVIDecoderConfigurationRecord`, `AVDynamicHDRPlus`, and `enum AVAudioServiceType`, helper-produced `AV_PKT_DATA_ENCRYPTION_INFO` bytes from `av_encryption_info_add_side_data()`, helper-produced `AV_PKT_DATA_ENCRYPTION_INIT_INFO` bytes from `av_encryption_init_info_add_side_data()`, allocator-produced native `AVIAMFParamDefinition` payload layouts from `av_iamf_param_definition_alloc()` for `AV_PKT_DATA_IAMF_MIX_GAIN_PARAM`, `AV_PKT_DATA_IAMF_DEMIXING_INFO_PARAM`, and `AV_PKT_DATA_IAMF_RECON_GAIN_INFO_PARAM`, the native int32 displaymatrix payload layout, plus fixed/comment-defined/raw payload rows for AV_PKT_DATA_PALETTE's AVPALETTE_SIZE payload, H.263 MB info, quality stats, fallback track, skip samples, parameter change, JP dual mono, string metadata/update, subtitle position, Matroska BlockAdditional, WebVTT identifier/settings, MPEGTS stream ID, A53 CC, AFD, ICC opaque bytes, S12M timecode, frame cropping, and LCEVC. It also compares `av_packet_pack_dictionary` / `av_packet_unpack_dictionary` rows including empty input, `AV_DICT_MULTIKEY` packing, duplicate-key unpack collapse, and malformed string-metadata return codes, `av_packet_alloc`, `av_packet_rescale_ts`, `av_packet_copy_props`, `av_packet_ref`, `av_packet_clone`, `av_packet_move_ref`, and `av_packet_unref` behavior against the Rust `Packet` model, including `opaque_ref` data, size, and `av_buffer_is_writable` state, one `AV_PKT_DATA_NEW_EXTRADATA` side-data entry, `av_packet_new_side_data` allocation/replacement plus zero-size allocation, `av_packet_get_side_data` including duplicate first-match lookup, size-NULL present/missing rows, and NULL/0 add rows where lookup returns NULL, `av_packet_shrink_side_data` success plus duplicate first-match shrink and missing ENOENT/oversize ENOMEM error rows, `av_packet_free_side_data`, `av_packet_add_side_data` replacement/append behavior including duplicate first-match replacement and nullable-zero caller data, standalone empty/null-array `av_packet_side_data_get`/`remove`/`free` no-op rows, standalone `av_packet_side_data_new` including zero-size allocation and same raw-type replacement after an out-of-range append, `av_packet_side_data_add` including nullable-zero caller data and out-of-range raw-type append past `AV_PKT_DATA_NB`, `av_packet_side_data_get`, `av_packet_side_data_remove` including duplicate-type first-match lookup, first-match replacement, and last-match swap removal, and `av_packet_side_data_free` array behavior, `av_packet_side_data_from_frame`, `av_packet_side_data_to_frame`, duplicate frame-side insertion behavior with and without `AV_FRAME_SIDE_DATA_FLAG_REPLACE`, unmapped `EINVAL` paths, `av_new_packet` size/padding/writability including zero-size allocation, `av_packet_from_data` including zero-size ownership, documented property-preserving data/size/buf replacement, ref/clone shared data-pointer behavior, and make-writable detachment, `av_grow_packet`, `av_shrink_packet`, `av_packet_make_refcounted` including empty packets, `av_packet_make_writable` including empty packets, missing side-data lookup size reset, flags, opaque address copying, payload preservation, raw packet `time_base` storage including noncanonical and zero-denominator lifecycle rows, and `av_container_fifo_alloc_avpacket()` move/ref write, move/ref read, USER-only and REF|USER raw flag transfer, peek, drain including partial mixed move/ref release ordering, can-read, invalid peek, and empty-read behavior. Positive-size `av_new_packet` visible payload bytes are treated as unspecified. It is wired into `tests/differential/mappings.txt` as `avutil-packet|oracle-libavcodec-packet-core`:

The packet oracle also includes nullable-zero side-data lifecycle rows for
copy-props, ref, clone, and move. These rows prove packet-owned NULL/0 side
data is reallocated into ordinary zero-size padded storage by copy/ref/clone but
is preserved as NULL/0 by move-ref.

Additional `avutil-packet` differential rows now reuse generated media oracles
whose packet metadata is already compared against pinned FFmpeg 8.1.1:
generated MOV `ffprobe -show_packets` fields, YUV4MPEG2 sample-aspect and
color-range remux transfer, flexible YUV4MPEG2 frame headers, empty rawvideo
and WAV framecrc streams, odd PCM-to-WAV packet output, duplicate WAV data with
an empty final data chunk, first duplicate WAV fmt preservation, and image2
gap/start-probe packet rows.

The newest `packet:side-free-preserves-fields`, `packet:side-free-empty`, and
`packet:side-free-repeat-*` rows prove `av_packet_free_side_data()` clears
packet-owned side data without changing payload bytes,
PTS/DTS/duration/position, stream index, flags, opaque pointer metadata,
`opaque_ref` contents/writability, or packet `time_base`; empty packets and
repeated calls remain no-ops.

The newest packet-owned raw shrink rows prove `av_packet_shrink_side_data()` accepts raw side-data type `-1` and shrinks it to zero length, while an oversize shrink for raw `INT_MIN` returns ENOMEM and leaves the entry bytes unchanged.

The newest zero-size side-data replacement rows prove packet-owned
`av_packet_add_side_data()` and standalone `av_packet_side_data_add()` replace
an existing `AV_PKT_DATA_NEW_EXTRADATA` entry with a zero-size caller-owned
entry, keep the side-data count at one, and keep lookup present with size zero.

The newest nullable-zero side-data rows prove packet-owned
`av_packet_add_side_data(pkt, type, NULL, 0)` stores a zero-size entry while
`av_packet_get_side_data()` returns NULL, and standalone
`av_packet_side_data_add(..., NULL, 0, flags)` stores a zero-size entry while
`av_packet_side_data_get()` still returns that entry pointer.

The newest zero-size side-data lifecycle rows prove `av_packet_copy_props()`,
`av_packet_ref()`, `av_packet_clone()`, and `av_packet_move_ref()` preserve a
zero-size `AV_PKT_DATA_NEW_EXTRADATA` record with lookup present and size zero;
the move-ref row also proves the source side-data list resets.

The newest custom-padding payload rows prove `av_grow_packet(pkt, 0)` on
ordinary writable padded storage preserves the packet data pointer and visible
bytes while zeroing dirty input padding, while exact-size and oversize
`av_shrink_packet()` preserve the same dirty padding as true no-ops. Rust
mirrors this through `Packet::grow_data(0)`, no-op `Packet::shrink_data()`,
focused unit coverage, and the `avutil_core_models` deterministic fixture.

The newest offset-payload resize rows prove `av_grow_packet(pkt, 0)` preserves
`pkt->data` when it points at an offset inside `pkt->buf->data` and the backing
buffer already has enough FFmpeg input-padding capacity, while zeroing dirty
padding. Rust mirrors this by preserving unique owned `BufferRef` offsets
through `resize_with_padding`, focused packet unit coverage, and the
`avutil_core_models` deterministic fixture.

The newest positive offset-grow rows prove `av_grow_packet(pkt, 2)` also
preserves that offset `pkt->data` pointer when the backing buffer already has
enough capacity, exposes the existing two tail bytes as newly visible payload,
and zeroes the new padding window. Rust mirrors this through the same
offset-preserving resize path plus focused packet unit coverage and the
`avutil_core_models` deterministic fixture.

The newest offset-shrink rows prove `av_shrink_packet(pkt, 2)` preserves the
same offset `pkt->data` pointer, keeps the visible prefix bytes, and zeroes the
new padding window over the truncated tail. Rust mirrors this through the same
offset-preserving resize path plus focused packet unit coverage and the
`avutil_core_models` deterministic fixture.

The newest offset-helper rows prove `av_packet_make_refcounted()` and
`av_packet_make_writable()` are no-ops for a unique writable packet whose
`pkt->data` points inside `pkt->buf->data`: FFmpeg preserves the offset pointer,
payload, writability, and dirty padding bytes. Rust mirrors this through
focused packet unit coverage and the `avutil_core_models` deterministic
fixture.

The newest offset-clone rows prove `av_packet_clone()` preserves an offset
`pkt->data` pointer inside `pkt->buf->data`, keeps both packet refs at the same
offset with the same visible bytes and dirty padding, and makes both refs
non-writable because they share the same `AVBuffer`. Rust mirrors this through
focused packet unit coverage and the `avutil_core_models` deterministic
fixture.

The newest offset-ref rows prove `av_packet_ref()` preserves an offset
`pkt->data` pointer inside `pkt->buf->data`, keeps both packet refs at the same
offset with the same visible bytes and dirty padding, and makes both refs
non-writable because they share the same `AVBuffer`. Rust mirrors this through
focused packet unit coverage and the `avutil_core_models` deterministic
fixture.

The newest offset-move rows prove `av_packet_move_ref()` transfers an offset
`pkt->data` pointer inside `pkt->buf->data` into the destination, keeps the
visible bytes and dirty padding, preserves writability because the buffer is not
shared, and resets the source packet to defaults. Rust mirrors this through
focused packet unit coverage and the `avutil_core_models` deterministic
fixture.

The newest offset-copy-props rows prove `av_packet_copy_props()` preserves a
destination offset `pkt->data` pointer inside `pkt->buf->data`, keeps visible
bytes, dirty padding, and writability unchanged, and copies timestamps, flags,
side data, opaque pointer metadata, `opaque_ref`, stream index, and
`time_base` from the source. Rust mirrors this through focused packet unit
coverage and the `avutil_core_models` deterministic fixture.

The newest shared offset make-writable rows prove `av_packet_make_writable()`
on a shared packet ref detaches an offset `pkt->data` pointer inside
`pkt->buf->data` into a writable zero-offset destination buffer, preserves
visible bytes plus packet properties, zeroes destination input padding, and
leaves the source offset-backed dirty-padding buffer intact once the refcount
drops back to one. Rust mirrors this through focused packet unit coverage and
the `avutil_core_models` deterministic fixture.

The newest offset-unref rows prove `av_packet_unref()` on an offset
`pkt->data` pointer inside `pkt->buf->data` resets packet fields to defaults
and calls the AVBuffer release callback with the original backing allocation
base. The release row captures prefix, visible payload, and dirty padding bytes
at callback time. Rust mirrors this through focused packet unit coverage and
the `avutil_core_models` deterministic fixture.

The newest packet FIFO partial-drain rows prove `av_container_fifo_drain(fifo, 1)` on a mixed move/ref packet FIFO releases the drained move-written packet's payload buffer and `opaque_ref` buffer immediately, keeps the ref-written packet queued, then delays ref-source payload release until the queued ref is drained and the original source drops. Rust `PacketFifo::drain` plus `avutil_core_models` mirror that release ordering.

The newest packet side-data free rows prove `av_packet_free_side_data()` is a no-op on an empty packet and remains idempotent after clearing a populated packet, while preserving payload, timestamps, flags, opaque pointer metadata, `opaque_ref`, stream index, and packet `time_base`. Rust `Packet::clear_side_data()` plus `avutil_core_models` mirror the empty and repeated-clear shape.

The newest packet FIFO free rows prove `av_container_fifo_free()` on a packet FIFO with a queued move-written packet releases the queued packet's payload buffer and `opaque_ref` buffer exactly once while leaving the moved-from source packet reset. The Rust `PacketFifo` model covers the same queued-drop lifecycle through `PacketFifo` drop/clear behavior and the `avutil_core_models` deterministic fuzz fixture.

The newest packet FIFO nullable-free rows prove `av_container_fifo_free(&fifo)` with `fifo == NULL` is a no-op and freeing an empty packet FIFO nulls the pointer. The oracle deliberately avoids `av_container_fifo_free(NULL)`, because pinned FFmpeg requires the pointer-to-pointer argument itself to be valid. Rust mirrors the supported nullable-pointee lifecycle with `Option<PacketFifo>::take()` unit coverage and the deterministic `avutil_core_models` fixture.

The newest packet FIFO clear-equivalence rows use `av_container_fifo_drain(fifo, av_container_fifo_can_read(fifo))` because FFmpeg exposes no direct `av_container_fifo_clear()` API. They prove all-draining a mixed move/ref packet queue releases move-written payload and `opaque_ref` storage once, preserves a ref-written source until that source is freed, leaves the FIFO empty, and keeps an empty all-drain no-op. Rust covers the direct `PacketFifo::clear()` helper with unit and deterministic fuzz invariants against the same lifecycle.

The newest packet FIFO invalid-operation rows prove an out-of-range `av_container_fifo_peek()` on a mixed move/ref queue returns EINVAL without changing readable count or queued packet order. Follow-on rows read the moved packet first, then drain the ref packet while preserving the ref-written source packet, and the Rust `PacketFifo` plus `avutil_core_models` fixture mirror that no-mutation lifecycle.

The newest direct packet-free rows prove `av_packet_free(NULL)` and `av_packet_free(&NULL)` are no-ops, and that `av_packet_free(&pkt)` on one side of a shared refcounted packet pair nulls only that pointer while retaining the other owner. Payload and `opaque_ref` custom release callbacks fire exactly once after the final owner is freed. Rust mirrors that C nullable-pointer lifecycle through `Option<Packet>::take()`, Drop, unit coverage, and the deterministic `avutil_core_models` fuzz fixture.

The packet bridge rows also include `packet:frame-to-packet-map-inventory` and `packet:packet-to-frame-map-inventory`, proving all 11 pinned FFmpeg 8.1.1 global packet/frame side-data mappings in both directions while preserving expected kind order and payload bytes. They also cover `AV_FRAME_SIDE_DATA_FLAG_REPLACE`, `AV_FRAME_SIDE_DATA_FLAG_UNIQUE`, `AV_FRAME_SIDE_DATA_FLAG_NEW_REF`, and the combined `UNIQUE | REPLACE` flag shape. For `av_packet_side_data_from_frame()` on a duplicate-rich packet side-data array, the pinned FFmpeg 8.1.1 REPLACE, UNIQUE, and combined UNIQUE|REPLACE behavior is packet-specific: the first matching mapped packet side-data entry is replaced and later duplicates are preserved. For `av_packet_side_data_to_frame()`, REPLACE updates the matching frame-side entry in place while preserving nonmatching entry order; UNIQUE and combined UNIQUE|REPLACE follow the frame-owned insertion path by removing matching frame side data, preserving nonmatching entries, and appending the mapped replacement. For NEW_REF, the bounded raw packet/frame side-data conversion surface returns success and produces the same mapped side-data payload shape as the ordinary copied insertion path.

The harness also includes `packet:rescale-duration-only`, `packet:rescale-mixed`, `packet:rescale-mixed-dts`, `packet:rescale-zero-duration`, `packet:rescale-negative-ts`, `packet:rescale-near-inf-rounding`, and `packet:rescale-negative-near-inf-rounding`, proving `av_packet_rescale_ts()` rescales positive duration even when PTS/DTS remain `AV_NOPTS_VALUE`, preserves `AV_NOPTS_VALUE` independently for PTS or DTS, rescales valid timestamps while preserving zero duration, rescales negative PTS/DTS with positive duration, uses nearest-away rounding for positive fractional timestamp/duration rescale plus the negative half-tick PTS row, and avoids changing payload bytes, byte position, flags, stream index, or the packet `time_base` field. The `packet:rescale-overflow` and `packet:rescale-invalid-time-base` rows cover FFmpeg's sentinel-writing edge when the underlying rational rescale returns `INT64_MIN`; `packet:rescale-invalid-dst-time-base` covers the distinct zero-result edge for a `{1,0}` destination time base.

The harness also includes `packet:copy-props-raw-time-base`,
`packet:ref-raw-time-base`, `packet:clone-raw-time-base`, and
`packet:move-raw-time-base-*` rows. These prove `AVPacket.time_base` is raw
field storage: FFmpeg preserves noncanonical, `0/0`, and negative-denominator
`AVRational` pairs through copy-props, ref, clone, and move-ref lifecycles
without rational normalization, while move-ref resets the source packet to the
default zero time base.

The harness also includes pre-populated `av_new_packet()` rows. Successful positive-size and zero-size calls reset packet metadata, side data, opaque pointer metadata, `opaque_ref`, stream index, flags, and time base before installing writable padded payload storage; the positive-size row initializes bytes before comparison because fresh allocation contents are unspecified, and the zero-size row proves an empty visible payload with FFmpeg input padding. The `INT_MAX - AV_INPUT_BUFFER_PADDING_SIZE` invalid-size boundary returns `EINVAL` without mutating the existing packet.

The harness also includes negative signed packet payload-size rows:
`packet:payload-new-packet-negative-*` proves `av_new_packet(pkt, -1)` returns
EINVAL without mutating a populated packet, and
`packet:payload-grow-negative-*` proves `av_grow_packet(pkt, -1)` on a nonempty
packet returns ENOMEM without mutating packet fields, payload bytes, input
padding, or writability. `packet:payload-grow-empty-negative-*` proves the same
ENOMEM return without mutating an empty packet.

The harness also includes `packet:payload-from-data-invalid-*` rows. These prove `av_packet_from_data()` returns `AVERROR(EINVAL)` before mutation when `size >= INT_MAX - AV_INPUT_BUFFER_PADDING_SIZE`; the Rust packet constructors and replacement helper use `Packet::validate_payload_len` to reject the same boundary before allocation.

The harness also includes `packet:payload-from-data-ref-*`, `packet:payload-from-data-clone*`, and `packet:payload-from-data-make-writable-*` rows. These prove `av_packet_from_data()` installs refcounted payload ownership: `av_packet_ref()` and `av_packet_clone()` share the adopted data pointer and report non-writable shared buffers, while `av_packet_make_writable()` detaches the destination before caller mutation and leaves the source payload unchanged.

The harness also includes direct `av_init_packet()` rows. `Packet::init_legacy()` matches the deterministic reset shape by preserving payload data/size while clearing `AVPacket.buf`/refcounted ownership, resetting unknown timestamps and position, zero duration, stream index 0, empty flags, cleared side data, cleared opaque metadata, cleared `opaque_ref`, and `time_base` `0/1`. A follow-on `av_packet_make_refcounted()` row proves FFmpeg copies that preserved raw payload into fresh padded storage with a different data pointer; `Packet::make_refcounted()` now mirrors that post-init helper shape. The safe Rust model releases owned metadata when clearing it; it does not model C leak behavior caused by calling `av_init_packet()` on an already-owned packet.

The harness also includes `packet:unref-empty` and `packet:unref-repeat` rows. These prove `av_packet_unref()` resets an already-empty allocated packet to the same default state and remains idempotent after releasing a populated packet.

The harness also includes `packet:ref-replace`, `packet:ref-replace-side`, and `packet:ref-replace-payload` rows for `av_packet_ref()`. These rows prove that a destination packet with existing payload, side data, opaque pointer metadata, and `opaque_ref` is unreferenced and then replaced with source packet state.

The harness also includes `packet:move-replace-dst`, `packet:move-replace-dst-side`, `packet:move-replace-dst-payload`, and `packet:move-replace-src` rows for `av_packet_move_ref()`. These rows prove that a destination packet with existing payload, side data, opaque pointer metadata, and `opaque_ref` is unreferenced before FFmpeg transfers the complete source packet state and resets the source packet to defaults.

The harness also includes `packet:copy-props-replace`, `packet:copy-props-replace-side`, and `packet:copy-props-replace-payload` rows for `av_packet_copy_props()`. These rows prove that a destination packet with existing payload, side data, opaque pointer metadata, and `opaque_ref` keeps its payload bytes while FFmpeg replaces the old metadata, side data, and `opaque_ref` with source properties.

The harness also includes `packet:*duplicate-side` lifecycle rows, proving `av_packet_ref()`, `av_packet_clone()`, and `av_packet_copy_props()` collapse duplicate packet-owned side-data kinds by later-entry replacement while `av_packet_move_ref()` transfers the raw duplicate side-data array and resets the source. `avutil_core_models` now has a deterministic fixture for the same copy/ref/clone collapse versus move-transfer split.

The harness also includes `packet:*empty*` lifecycle rows for an empty source packet. These prove `av_packet_copy_props()` preserves destination payload bytes while clearing packet properties, side data, opaque pointer metadata, and `opaque_ref`; `av_packet_ref()` and `av_packet_clone()` from an empty raw packet produce a zero-size writable refcounted payload with zeroed input padding; and `av_packet_move_ref()` transfers the unpadded default empty state while resetting the source.

The harness also includes a `packet:payload-layout-palette` row, proving FFmpeg's packet palette side-data payload uses `AVPALETTE_SIZE` bytes. `PacketPalette` derives its modeled length from the same shared `AVPALETTE_COUNT` and `AVPALETTE_SIZE` constants used by the pixel-format model.

The harness also includes a `packet:payload-layout-content-light` row, proving FFmpeg's packet content-light side-data payload uses the native 8-byte `AVContentLightMetadata` shape with `MaxCLL` at offset 0 and `MaxFALL` at offset 4.

The harness also includes a `packet:payload-layout-mastering-display` row, proving FFmpeg's packet mastering-display side-data payload uses the native 88-byte `AVMasteringDisplayMetadata` shape with display primaries at offset 0, white point at offset 48, min luminance at offset 64, max luminance at offset 72, `has_primaries` at offset 80, and `has_luminance` at offset 84.

The harness also includes a `packet:payload-layout-ambient-viewing-environment` row, proving FFmpeg's packet ambient viewing environment side-data payload uses the native 24-byte `AVAmbientViewingEnvironment` shape with ambient illuminance at offset 0, ambient light x at offset 8, and ambient light y at offset 16.

The harness also includes a `packet:payload-layout-3d-reference-displays` row, proving FFmpeg's packet 3D reference displays side-data payload uses the native `AV3DReferenceDisplaysInfo` allocation envelope from `av_tdrdi_alloc()`, including precision/count offsets, `entries_offset` and `entry_size` metadata, and `AV3DReferenceDisplay` entry field offsets.

The harness also includes a `packet:payload-layout-spherical` row, proving FFmpeg's packet spherical side-data payload uses the native 36-byte `AVSphericalMapping` shape with projection at offset 0, yaw/pitch/roll at offsets 4/8/12, bounds at offsets 16/20/24/28, and padding at offset 32.

The harness also includes a `packet:payload-layout-displaymatrix` row, proving FFmpeg's packet displaymatrix side-data payload uses the native 36-byte `int32_t[9]` shape with elements at offsets 0, 4, 8, 12, 16, 20, 24, 28, and 32. The `packet:display-rotation-set-get`, `packet:display-rotation-boundaries`, `packet:display-rotation-singular`, `packet:display-rotation-get-affine`, and `packet:display-flip` rows prove the bounded `av_display_rotation_set()` / `av_display_rotation_get()` and `av_display_matrix_flip()` behavior for current `PacketDisplayMatrix` helpers, including fractional/near-boundary/wraparound angle behavior, axis-normalized rotation extraction from raw affine matrices, and NaN behavior for zero matrix axes.

The harness also includes a `packet:payload-layout-stereo3d` row, proving FFmpeg's packet stereo3d side-data payload uses the native 36-byte `AVStereo3D` shape with type at offset 0, flags at offset 4, view at offset 8, primary eye at offset 12, baseline at offset 16, horizontal disparity adjustment at offset 20, and horizontal field of view at offset 28.

The harness also includes a `packet:payload-layout-dynamic-hdr10-plus` row, proving FFmpeg's packet Dynamic HDR10+ side-data payload uses the pinned native 11304-byte `AVDynamicHDRPlus` envelope with selected top-level offsets for the country/application/window header, `params` array stride, targeted-system display maximum luminance and actual-peak grid, and mastering-display actual-peak grid.

The harness also includes a `packet:payload-layout-encryption-info` row, proving FFmpeg's `av_encryption_info_add_side_data()` helper emits the modeled big-endian `AV_PKT_DATA_ENCRYPTION_INFO` byte envelope for scheme, pattern block fields, key ID, IV, subsample count, and clear/protected subsample byte counts.

The harness also includes a `packet:payload-layout-encryption-init-info` row, proving FFmpeg's `av_encryption_init_info_add_side_data()` helper emits the modeled big-endian `AV_PKT_DATA_ENCRYPTION_INIT_INFO` byte envelope for linked init-info record count, system ID, key ID count/size/data, init data size/data, and a zero-key second record.

The harness also includes `packet:payload-layout-iamf-mix-gain-param`, `packet:payload-layout-iamf-demixing-info-param`, and `packet:payload-layout-iamf-recon-gain-info-param` rows. They allocate the native `AVIAMFParamDefinition` envelopes through FFmpeg's `av_iamf_param_definition_alloc()`, zero copied `AVClass *` pointer fields before byte comparison, and prove the pinned header offsets, subblock offsets/sizes/counts, mix-gain animation/rational fields, demixing `dmixp_mode`, and recon-gain 6x12 table layout.

The harness also includes `packet:side-new-zero`, `packet:array-new-zero`, `packet:side-add-zero*`, and `packet:array-add-zero*` rows, proving FFmpeg 8.1.1 accepts and retains zero-size `AV_PKT_DATA_NEW_EXTRADATA` entries through both packet-owned/standalone allocation APIs and packet-owned/standalone ownership-add APIs.

The harness also includes `packet:payload-new-zero*`, `packet:payload-from-data-zero*`, `packet:payload-make-refcounted-empty*`, and `packet:payload-make-writable-empty*` rows, proving zero-size packet payload helpers keep zero visible payload bytes while retaining zeroed FFmpeg input padding and writable refcounted storage.

The harness also includes `packet:payload-from-data-replace-*` rows, proving
replacement-style `av_packet_from_data()` on a populated/property-bearing
packet installs refcounted payload ownership while preserving packet
properties. Follow-on `av_packet_ref()` shares the adopted data pointer,
`av_packet_make_refcounted()` keeps the same pointer, and
`av_packet_make_writable()` on a shared ref detaches before mutation.

The harness also includes `packet:payload-from-data-null-zero*` rows, proving
`av_packet_from_data(pkt, NULL, 0)` creates a nullable zero-size refcounted
payload with `pkt->data == NULL`, `pkt->buf != NULL`, `pkt->buf->data == NULL`,
an `AV_INPUT_BUFFER_PADDING_SIZE` backing size, and writable storage. Ref/clone
and make-refcounted preserve that nullable pointer shape, unique make-writable
is a same-shape no-op, and make-writable on a shared nullable ref detaches to
ordinary writable padded zero-length storage. Move-ref transfers the nullable
refcounted storage to the destination and resets the source, while unref resets
the packet to the default visible state. The Rust model exposes this bounded C
shape through `Packet::from_null_data_zero`. Source inspection shows the pinned
`av_grow_packet()` path would dereference NULL for this exact nullable-zero
state, so the harness does not add a crashing grow row.

The harness also includes already-refcounted payload no-op rows for `packet:payload-make-writable-unique*`, `packet:payload-make-refcounted-unique*`, and `packet:payload-make-refcounted-shared*`. These prove unique refcounted packets keep their visible data pointer and writable padded storage, while shared refcounted packets keep shared storage and remain non-writable after `av_packet_make_refcounted()`.

The harness also includes `packet:payload-make-refcounted-readonly-*` and `packet:payload-make-writable-readonly-*` rows. These prove an existing read-only `AVBufferRef` is considered refcounted and left attached/non-writable by `av_packet_make_refcounted()`, then detached to writable padded storage by `av_packet_make_writable()`.

The harness also includes `packet:payload-make-refcounted-offset-padding*` and `packet:payload-make-writable-offset-padding*` rows. These prove refcounted and writable helper calls preserve a unique writable offset-backed payload pointer, offset, visible bytes, dirty padding, and writability.

The harness also includes `packet:payload-clone-offset-padding*` rows. These prove `av_packet_clone()` preserves an offset-backed payload pointer, offset, visible bytes, and dirty padding in both packet references while shared ownership makes both refs non-writable.

The harness also includes `packet:payload-ref-offset-padding*` rows. These prove `av_packet_ref()` preserves an offset-backed payload pointer, offset, visible bytes, and dirty padding in both packet references while shared ownership makes both refs non-writable.

The harness also includes `packet:payload-move-offset-padding*` rows. These prove `av_packet_move_ref()` transfers an offset-backed payload pointer, offset, visible bytes, and dirty padding into the destination while preserving writability and resetting the source packet.

The harness also includes `packet:payload-copy-props-offset-padding*` rows.
These prove `av_packet_copy_props()` preserves an offset-backed destination
payload pointer, offset, visible bytes, dirty padding, and writability while
copying source packet properties.

The harness also includes
`packet:payload-make-writable-shared-offset-padding*` rows. These prove
`av_packet_make_writable()` on a shared ref to an offset-backed payload detaches
the destination to writable zero-offset padded storage, preserves source and
destination visible bytes plus packet properties, zeroes destination input
padding, and leaves the source offset buffer and dirty padding intact.

The harness also includes `packet:payload-unref-offset-padding*` rows. These
prove `av_packet_unref()` resets an offset-backed packet to default fields and
releases the original backing allocation base with prefix, payload, and dirty
padding bytes still observable by the release callback.

The harness also includes `packet:payload-grow-empty-zero*`,
`packet:payload-grow-empty*`, `packet:payload-shrink-oversize`, and
`packet:payload-shrink-zero` rows. These prove empty-packet zero growth and
positive growth both return success with zeroed input padding and writable
refcounted storage; oversize `av_shrink_packet()` is a no-op; and
shrink-to-zero keeps a writable padded buffer while zeroing the exposed padding
window. FFmpeg's newly visible bytes after positive `av_grow_packet()` are
allocator-dependent, so growth rows compare stable prefix bytes where present,
size, padding, and writability rather than all grown payload bytes. The Rust
model intentionally zeroes newly grown bytes for deterministic safe ownership.

The harness also includes `packet:payload-grow-zero-offset-padding*` rows. These prove zero-growth on a packet whose visible payload starts at an offset inside its refcounted buffer preserves the offset data pointer and visible bytes while zeroing the FFmpeg input-padding window.

The harness also includes `packet:payload-grow-offset-padding*` rows. These prove positive growth on the same offset-payload shape preserves the data pointer and offset, exposes existing tail bytes as newly visible payload, and zeroes the new FFmpeg input-padding window.

The harness also includes `packet:payload-shrink-offset-padding*` rows. These prove shrinking the same offset-payload shape preserves the data pointer and offset, keeps the prefix bytes, and zeroes the new FFmpeg input-padding window over the truncated tail.

The harness also includes `packet:payload-grow-invalid-*` rows. These prove `av_grow_packet()` returns `AVERROR(ENOMEM)` before mutation when `grow_by` exceeds `INT_MAX - (pkt->size + AV_INPUT_BUFFER_PADDING_SIZE)`, preserving packet fields, side data, opaque metadata, `opaque_ref`, time base, payload bytes, input padding, and writability.

The harness also includes `packet:payload-grow-shared*` rows, proving shared refcounted `av_grow_packet()` behavior. When the shared backing storage must grow, FFmpeg detaches the destination packet from the source, preserves the source payload, preserves stable prefix bytes in the grown packet, and exposes writable padded destination storage; newly visible grown bytes remain allocator-dependent.

A pinned diagnostic expansion for shared `av_shrink_packet()` is not kept in the
passing mapped row set yet. It showed that shrinking one reference of a shared
refcounted packet preserves the destination data pointer, keeps both refs
non-writable, and zeroes the truncated tail through bytes still visible in the
source packet. The current safe Rust `BufferRef` resize path copy-on-write
detaches instead, so the ignored
`packet_shrink_shared_refcounted_matches_ffmpeg_tail_zeroing` regression records
this as a pending alias-safe storage-design gap.

The harness also includes `packet:payload-grow-unrefcounted*`,
`packet:payload-grow-unrefcounted-offset*`, and
`packet:payload-make-writable-unrefcounted*` rows, proving raw
`AVPacket.data`/`size` helpers with no `buf` preserve prefix bytes, add zeroed
FFmpeg input padding, and return writable storage after grow or make-writable.
The offset grow rows prove `av_grow_packet()` copies from the current data
pointer into zero-offset padded storage when raw caller storage starts before
`pkt->data`. FFmpeg leaves the newly visible bytes from no-buffer
`av_grow_packet()` allocator-unspecified, so those rows compare payload length,
preserved prefix, padding, and writability rather than full grown payload
bytes.

The harness also includes `packet:payload-make-refcounted-unrefcounted-offset-*` and `packet:payload-make-writable-unrefcounted-offset-*` rows, proving raw `AVPacket.data`/`size` helpers with no `buf` still copy from the current data pointer when that pointer starts at a nonzero offset inside caller storage. FFmpeg installs zero-offset padded `AVBufferRef` storage for both helper calls, returns success, zeroes input padding, and leaves the copied buffer writable.

The harness also includes `packet:payload-shrink-unrefcounted` and
`packet:payload-shrink-unrefcounted-offset*` rows, proving raw
`AVPacket.data`/`size` shrink behavior with caller-provided input padding.
FFmpeg truncates visible size and zeroes padding without allocating an
`AVBufferRef`; the offset rows additionally prove the raw data pointer stays at
the same caller-storage offset. The rows compare payload length, visible bytes,
and padding rather than writability.

The harness also includes `packet:payload-ref-unrefcounted-*`, `packet:payload-clone-unrefcounted`, `packet:payload-ref-unrefcounted-offset-*`, and `packet:payload-clone-unrefcounted-offset-*` rows, proving raw `AVPacket.data`/`size` reference behavior when `pkt->buf` is NULL. FFmpeg copies the visible bytes at the current data pointer into new zero-offset padded refcounted destination storage for both `av_packet_ref()` and `av_packet_clone()`, including when the source data pointer starts at a nonzero offset inside caller storage, while the raw source packet remains a no-buffer packet.

The harness also includes `packet:dict-pack-multikey`, `packet:dict-unpack-multikey-ret`, and `packet:dict-unpack-multikey` rows. These pin the `AV_DICT_MULTIKEY` pack shape and the subsequent case-insensitive duplicate-key unpack collapse. The `packet:dict-unpack-empty-ret`, `packet:dict-unpack-missing-final-nul-ret`, `packet:dict-unpack-key-without-value-ret`, `packet:dict-unpack-empty-key-ret`, and `packet:dict-unpack-trailing-empty-key-ret` rows pin `av_packet_unpack_dictionary()` return-code behavior for empty input and malformed string metadata, with the malformed rows returning `AVERROR_INVALIDDATA`.

The harness also includes `packet:side-add-capacity-*`, `packet:side-add-capacity-overflow-owned`, and `packet:side-new-capacity-overflow` rows, proving packet-owned side-data capacity behavior at `AV_PKT_DATA_NB`: replacement remains valid at capacity, append fails with `ERANGE` without changing the entry count, failed `av_packet_add_side_data()` append preserves the caller-owned data pointer for the caller to free, and `av_packet_new_side_data()` returns NULL at capacity. The packet-owned raw lookup rows prove `av_packet_get_side_data()` finds raw inserted side-data kinds at `AV_PKT_DATA_NB`, `AV_PKT_DATA_NB + 1`, `-1`, and `INT_MIN` after the corresponding `av_packet_add_side_data()` or `av_packet_new_side_data()` calls.

The harness also includes `packet:array-add-capacity-*` and `packet:array-new-capacity-overflow` rows, proving the standalone `AVPacketSideData` array helpers do not apply the same packet-owned `AV_PKT_DATA_NB` ceiling: `av_packet_side_data_add()` accepts an out-of-range raw type and grows the array from 41 to 42 entries, and `av_packet_side_data_new()` for that same raw type replaces the entry while keeping the count at 42. The newest standalone raw-type rows add `packet:array-add-raw-negative-type-*` and `packet:array-new/get/remove-raw-min-type`, proving the same standalone array API accepts negative raw values `-1` and `INT_MIN`, lookup finds them, and removing the `INT_MIN` entry preserves the `-1` entry.

The harness also includes `packet:array-new-flags-nonzero-*` and `packet:array-add-flags-nonzero-*` rows, proving pinned FFmpeg 8.1.1 currently ignores nonzero flags for standalone `av_packet_side_data_new()` and `av_packet_side_data_add()`. The `new` row initializes the returned bytes before comparison because positive-size FFmpeg allocation contents are not stable before caller writes, and the `add` row proves first-match replacement plus caller-buffer ownership transfer.

The harness also includes `packet:array-empty-*` rows, proving standalone `AVPacketSideData` empty/null-array behavior: `av_packet_side_data_get(NULL, 0, type)` returns missing, `av_packet_side_data_remove(NULL, &count, type)` leaves the count at zero, and `av_packet_side_data_free(&ptr, &count)` leaves pointer and count empty.

The harness also includes `packet:side-duplicate-*` rows, proving packet-owned duplicate-type side-data behavior: `av_packet_get_side_data()` returns the first matching duplicate entry, `av_packet_new_side_data()` replaces the first matching duplicate entry, `av_packet_add_side_data()` replaces the first matching duplicate entry, `av_packet_shrink_side_data()` shrinks the first matching duplicate entry while leaving later duplicates in place, and `av_packet_free_side_data()` clears all duplicate-rich side data so subsequent lookup returns missing.

The harness also includes `packet:side-get-*-size-null` rows, proving `av_packet_get_side_data()` may be called with a NULL size pointer for present and missing packet-owned side data.

The harness also includes `packet:array-new-duplicate-*` rows, proving standalone `av_packet_side_data_new()` duplicate-type behavior: the helper replaces the first matching duplicate entry while leaving later duplicates in place. The positive-size duplicate-new rows write deterministic bytes before comparison because FFmpeg allocator contents are not a stable oracle value until the caller initializes the returned buffer.

The harness also includes `packet:array-free-duplicate-*` rows, proving standalone `av_packet_side_data_free()` resets a duplicate-rich side-data array to an empty/null state.

The harness also includes `packet:fifo-*` rows, proving the packet-specialized container FIFO transfer semantics for move writes, ref writes, USER-only raw-flag move behavior, REF|USER raw-flag reference behavior, read draining, ref reads and move reads into pre-populated destinations, failed empty move/ref reads preserving pre-populated destinations, non-mutating peek, zero-count drain no-op behavior, valid positive drain including partial mixed move/ref release ordering, mixed move/ref all-drain clear equivalence, can-read counts, invalid offset handling, and empty-read `EINVAL` handling.

The same Rust test file includes an ignored `upstream_fate_avpacket_passes` wrapper for upstream FFmpeg's `fate-avpacket` target from `tests/fate/libavcodec.mak`, backed by `libavcodec/tests/avpacket.c`. It runs `make fate-avpacket` from `FFMPEG_FATE_BUILD_DIR` or the default WSL build cache created by `scripts/bootstrap_ffmpeg_oracle_wsl.sh`, and is wired through `tests/fate/upstream-mappings.txt` as `avutil-packet|fate-avpacket`.

```sh
cargo test -p avutil --test packet_oracle libavcodec_packet_core_lifecycle_matches_packet_model -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --target oracle-libavcodec-packet-core --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-packet --target fate-avpacket
```

`crates/avutil/tests/frame_oracle.rs` is an ignored oracle harness for libavutil
`AVFrame` core lifecycle helpers. It compiles a small test-only C helper against
`third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares `av_frame_alloc`,
`av_frame_unref`, `av_frame_ref`, `av_frame_clone`, `av_frame_replace`,
`av_frame_make_writable`, including empty-frame `EINVAL` behavior and rich
data-plane/side-data/opaque_ref behavior,
`av_frame_move_ref`, `av_frame_copy`,
`av_frame_copy_props`, PTS/packet-DTS/duration/time-base/sample-rate/channel-layout/sample-aspect-ratio,
crop-offset, picture-type, quality, repeat_pict, public frame-flag,
color_range, color_primaries, color_trc, colorspace, chroma_location,
best-effort timestamp, public decode-error flags, nullable non-dereferenceable
opaque pointer metadata, BufferRef-backed opaque_ref user storage, alpha_mode,
and top-level frame metadata propagation through ref/move/copy-props rows,
`av_frame_get_buffer` for a gray8 video frame, packed s16 stereo audio,
planar s16p stereo audio proving that only `linesize[0]` is set for planar
audio, planar 10-channel s16p direct-slot versus extended-buffer topology with
direct and extended `av_buffer_is_writable()` counts, and packed 10-channel s16
audio showing that high channel count alone does not allocate extended buffers,
`av_frame_get_plane_buffer()` lookup presence, visible plane bytes, refcount,
and writability for video, packed audio, direct planar audio, extended planar
audio, and out-of-range indexes,
`av_frame_apply_cropping()` for gray8 aligned, gray8 unaligned, byte-packed
RGB/BGR/Bayer8 default and unaligned cases, packed high-bit grayscale and
gray-alpha default and unaligned cases, pal8 default and unaligned cases,
RGB24 aligned, RGB24 unaligned,
BGR24 aligned, BGR24 unaligned, selected packed RGB/RGBA and Bayer16 default
and unaligned cases, selected bitstream right/bottom-only cases, and invalid crop rectangles,
`AV_FRAME_DATA_*` numeric
values/names/descriptors/properties, `av_frame_side_data_name()` invalid
raw-value boundaries, `AV_FRAME_SIDE_DATA_FLAG_*`,
`AV_SIDE_DATA_PROP_*`, `av_frame_new_side_data` / `av_frame_get_side_data`
/ `av_frame_remove_side_data` displaymatrix rows, duplicate
`av_frame_get_side_data()` / `av_frame_remove_side_data()` rows, and
frame-level `av_frame_new_side_data_from_buf()` rows against the Rust `Frame`
model. The `av_frame_copy` rows prove payload-only copy for gray8 video and
packed s16 audio, that destination frame properties remain unchanged, that a
larger video destination preserves bytes outside the source rectangle, and that
a too-small video destination returns `EINVAL` without mutating destination
payload bytes. The `av_frame_copy_props` rows prove destination payload buffers are
preserved, PTS, packet DTS, duration, time base, sample rate, sample aspect
ratio, crop offsets, picture type, quality, repeat_pict, public frame flags,
best-effort timestamp, decode-error flags, nullable opaque pointer metadata,
opaque_ref, alpha mode, and top-level color/chroma metadata are copied, while
the destination channel layout remains unchanged,
source metadata keys overwrite matching
destination metadata while destination-only metadata keys remain, existing destination side data is retained,
source side data is appended as a deep copy, and destination `hw_frames_ctx`
remains unchanged.
The `frame:unref-rich` row proves `av_frame_unref()` resets a populated frame
that owns payload buffers, metadata, displaymatrix side data, `hw_frames_ctx`,
and `opaque_ref`, matching the Rust `Frame::unref()` reset surface.
The `frame:ref-rich-*` rows prove valid `av_frame_ref()` behavior on a newly
allocated destination: payload buffers, displaymatrix side data,
`hw_frames_ctx`, and `opaque_ref` are shared through new references, source
properties are copied, and the source frame remains populated. A pre-populated
destination is intentionally not claimed as an oracle row because FFmpeg
documents that call shape as invalid caller input; Rust `Frame::ref_from()`
safely releases existing destination owners before cloning as a wrapper
boundary.
The `frame:empty-make-writable-ret` and `frame:empty-after-make-writable`
rows prove `av_frame_make_writable()` returns `EINVAL` for an empty frame and
leaves that frame in the default unallocated state. Rust exposes the fallible
equivalent through `FrameData::try_make_writable()` and
`Frame::try_make_writable()`.
The `frame:rich-after-make-writable-*` rows prove that a rich refcounted
software frame detaches data-plane buffers, deep-copies frame side data, keeps
`opaque_ref` shared, and leaves the source frame populated after
`av_frame_make_writable()`. The oracle row clears the fake `hw_frames_ctx`
before calling the C helper because an arbitrary `AVBufferRef` is not a valid
hardware frames context and FFmpeg may dereference it on this path; hardware
context make-writable parity remains outside this bounded row.
The `av_frame_side_data_name` boundary row proves that known 0-based
`AV_FRAME_DATA_*` values map to descriptor names, while -1, the first raw value
after `AV_FRAME_DATA_EXIF`, the next raw value, and INT_MAX return NULL. FFmpeg
8.1.1 does not expose a public `AV_FRAME_DATA_NB` sentinel, so the Rust model
uses the current pinned inventory length as its bounded sentinel.
The adjacent `av_frame_side_data_desc` boundary row proves the same raw-value
range for descriptor lookup, including descriptor names and `AV_SIDE_DATA_PROP_*`
property bits for representative valid values and NULL for the invalid
boundaries.
The `av_frame_clone` and `av_frame_replace` rows prove source properties and
refcounted plane, side-data, hardware-context, and `opaque_ref` storage are
shared for cloned/replaced frames, pre-populated destination state is dropped
and replaced, and replacing from an empty source unreferences the destination.
The `frame:move-replace-*` rows prove `av_frame_move_ref()` first unreferences
a pre-populated destination carrying payload, side data, `hw_frames_ctx`, and
`opaque_ref`, then transfers the source references and resets the source frame
to defaults.
The `frame:fifo-*` rows prove `av_container_fifo_alloc_avframe()` move/ref
write/read behavior, non-mutating peek, readable counts, drain ordering,
invalid peek `EINVAL`, and the oracle-observed empty-read `EINVAL` result
against the Rust `FrameFifo` model.
The `av_frame_apply_cropping` rows prove FFmpeg's default left-crop rounding to
keep the data pointer at least 32-byte aligned for gray8, byte-packed RGB/BGR
(`rgb8`, `bgr8`, `rgb4_byte`, and `bgr4_byte`), Bayer8 (`bayer_bggr8`,
`bayer_rggb8`, `bayer_gbrg8`, and `bayer_grbg8`), paletted `pal8`, RGB24, and
BGR24, exact-left behavior under `AV_FRAME_CROP_UNALIGNED`, crop-field reset on success, and
`ERANGE` no-mutation behavior for invalid crop rectangles. They also prove the
packed high-bit grayscale/gray-alpha family (`ya8`, `ya16le`, `ya16be`,
`yaf16le`, `yaf16be`, `yaf32le`, `yaf32be`, `gray9le`, `gray9be`, `gray10le`,
`gray10be`, `gray12le`, `gray12be`, `gray14le`, `gray14be`, `gray16le`,
`gray16be`, `gray32le`, `gray32be`, `grayf16le`, `grayf16be`, `grayf32le`,
and `grayf32be`), 16-bit packed RGB/BGR family (`rgb565be`, `rgb565le`,
`rgb555be`, `rgb555le`, `bgr565be`, `bgr565le`, `bgr555be`, `bgr555le`,
`rgb444le`, `rgb444be`, `bgr444le`, and `bgr444be`), 32-bit packed RGB/RGBA
family (`rgba`, `bgra`, `argb`, `abgr`, `0rgb`, `rgb0`, `0bgr`, and `bgr0`),
high-depth packed RGB/RGBA family (`rgb48le`, `rgb48be`, `bgr48le`,
`bgr48be`, `rgba64le`, `rgba64be`, `bgra64le`, and `bgra64be`), and Bayer16
CFA family (`bayer_bggr16le`, `bayer_bggr16be`, `bayer_rggb16le`,
`bayer_rggb16be`, `bayer_gbrg16le`, `bayer_gbrg16be`, `bayer_grbg16le`, and
`bayer_grbg16be`) return `AVERROR_BUG` without mutation for default
nonzero-left crop while succeeding with exact-left behavior under
`AV_FRAME_CROP_UNALIGNED`.
The `frame:apply-crop-yuv420p-*`, `frame:apply-crop-yuv422p-*`, and
`frame:apply-crop-yuv444p-*` rows prove the bounded 8-bit planar software-frame
crop path for 4:2:0, 4:2:2, and 4:4:4 YUV. Default mode rounds the luma left
crop down for FFmpeg's 32-byte data-pointer alignment rule, derives chroma plane
starts from each format's rounded chroma offsets, keeps crop-visible chroma
dimensions floor-shifted after cropping, and `AV_FRAME_CROP_UNALIGNED` applies
exact luma crop with subsampled chroma offsets.
The standalone side-data array rows exercise `av_frame_side_data_new()`,
`av_frame_side_data_get()`, `av_frame_side_data_remove_by_props()`, and
`av_frame_side_data_free()`: duplicate insertion without flags fails without
mutation, `AV_FRAME_SIDE_DATA_FLAG_REPLACE` replaces non-MULTI entries,
`AV_FRAME_SIDE_DATA_FLAG_UNIQUE` removes matching entries before appending,
MULTI side data appends even with REPLACE, lookup returns the first matching
MULTI entry, missing lookup returns NULL, property removal clears all MULTI
entries, and free resets the array.
The buffer-backed side-data rows exercise `av_frame_side_data_add()` ownership:
successful non-`NEW_REF` insert and replace consume the caller `AVBufferRef`,
duplicate failure leaves the caller buffer alive, and
`AV_FRAME_SIDE_DATA_FLAG_NEW_REF` creates a second shared buffer reference
without taking ownership.
The frame-level from-buffer rows exercise `av_frame_new_side_data_from_buf()`:
successful insertion consumes caller ownership, duplicate non-MULTI ReplayGain
entries append instead of replacing or failing, MULTI SEI_UNREGISTERED entries
append, and the Rust safe wrapper rejects a missing caller buffer with `EINVAL`.
The duplicate frame-owned side-data rows exercise `av_frame_get_side_data()` and
`av_frame_remove_side_data()`: lookup returns the first matching duplicate, and
removal clears all records of the requested type while preserving nonmatching
MULTI side data.
The side-data clone rows exercise `av_frame_side_data_clone()`: cloned entries
copy source metadata, share the source `AVBufferRef` through a new reference,
return `EEXIST` for duplicate non-MULTI insertion without flags, replace
non-MULTI entries under `AV_FRAME_SIDE_DATA_FLAG_REPLACE`, remove matching
entries under `AV_FRAME_SIDE_DATA_FLAG_UNIQUE`, and append MULTI entries even
when `REPLACE` is set.
The make-writable rows caught and now verify the pinned default 64-byte
`av_frame_make_writable()` realignment path plus side-data deep-copy behavior.
The packed YUV 4:2:2 crop rows exercise `av_frame_apply_cropping()` for
`yuyv422`, `uyvy422`, `yvyu422`, `y210le`, `y210be`, `y212le`, `y212be`,
`y216le`, and `y216be`. The pinned rows prove default nonzero-left crop returns
`AVERROR_BUG` without mutation for these multi-byte packed formats. With
`AV_FRAME_CROP_UNALIGNED`, FFmpeg applies exact left offsets using two stored
bytes per pixel for the 8-bit packed rows and four stored bytes per pixel for
the high-bit rows, then resets crop fields on success.
The semi-planar crop rows exercise `av_frame_apply_cropping()` for 8-bit
`nv12`, `nv21`, `nv16`, `nv24`, and `nv42`, plus high-bit `nv20*` and the
`p010*`/`p012*`/`p016*`/`p210*`/`p212*`/`p216*`/`p410*`/`p412*`/`p416*`
families. The 8-bit rows cover default luma-left alignment rounding, while the
high-bit rows prove default nonzero-left crop returns `AVERROR_BUG` without
mutation. `AV_FRAME_CROP_UNALIGNED` applies exact luma offsets and
geometry-specific interleaved chroma offsets: two-byte chroma pairs for the
8-bit rows and four-byte chroma pairs for high-bit rows. The `nv24`/`nv42` and
`p410*`/`p412*`/`p416*` rows also record the oracle-observed full-resolution
chroma line size expansion under FFmpeg's default 64-byte allocation
alignment.
The UYYVYY411 crop rows prove default nonzero-left
`av_frame_apply_cropping()` returns `AVERROR_BUG` without mutation, while
`AV_FRAME_CROP_UNALIGNED` advances the data pointer by the descriptor
first-component step of four bytes for one left pixel. Visible rows are sized as
`ceil(width / 4) * 6`, and the current `av_frame_get_buffer(..., 64)` row for
an 8x4 UYYVYY411 frame has a 128-byte line size.
The pal8 crop rows prove FFmpeg treats the pixel index plane as a one-byte
packed crop surface: default nonzero-left crop rounds the left offset down for
the 32-byte data-pointer alignment rule, and `AV_FRAME_CROP_UNALIGNED` applies
the exact one-byte left offset. The pinned `calc_cropping_offsets()` path keeps
the palette plane offset at zero, so this row proves crop pointer math without
claiming full `AVFrame.data[1]` palette side-plane/context propagation.
The same Rust test file includes an ignored `upstream_fate_side_data_array_passes`
wrapper for upstream FFmpeg's `fate-side_data_array` target from
`tests/fate/libavutil.mak`, backed by `libavutil/tests/side_data_array.c`. It is
wired through `tests/fate/upstream-mappings.txt` as
`avutil-frame|fate-side_data_array`.
The harness is wired into
`tests/differential/mappings.txt` as `avutil-frame|oracle-libavutil-frame-core`:

```sh
cargo test -p avutil --test frame_oracle libavutil_frame_core_lifecycle_matches_current_model -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-frame --target oracle-libavutil-frame-core --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-frame --target fate-side_data_array
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

`crates/avutil/tests/buffer_oracle.rs` is an ignored oracle harness for libavutil `AVBufferRef` and bounded `AVBufferPool` helpers. It compiles a small test-only C helper against `third_party/ffmpeg-oracle/wsl/lib/libavutil.a` and compares public `AV_BUFFER_FLAG_READONLY`, public `AVBufferRef` size, alignment, and `buffer`/`data`/`size` field offsets and field sizes, allocation status including `av_buffer_alloc(0)`, `av_buffer_allocz(0)`, and huge `av_buffer_alloc(SIZE_MAX)` / `av_buffer_allocz(SIZE_MAX)` failure, zeroed allocation, ref sharing, writable `av_buffer_create` opaque-data owners including default-free opaque owners, zero-length custom-owner refs, and nullable `av_buffer_create(NULL, 0, ...)` custom-owner refs, `av_buffer_create` unknown-flag rows proving only `AV_BUFFER_FLAG_READONLY` affects writability, readonly `av_buffer_create` custom-owner make-writable detach and original-byte release rows, shared readonly custom-owner make-writable detach/release timing, av_buffer_create-style cloned refs preserving opaque/refcount/non-writable shared state, unique writable custom-owner grow/shrink realloc immediate-release rows, unique readonly custom-owner grow/shrink realloc immediate-release rows, shared writable custom-owner grow/shrink realloc detach/release timing, shared readonly custom-owner grow/shrink realloc detach/release timing, offset `data`/`size` subrange refs, cloned offset refs preserving visible data pointer/size/refcount shape, unique offset make-writable no-op behavior, unique pool-owned make-writable no-op rows, unique/shared offset realloc detach behavior, refcount/writability, make-writable copy-on-write including shared zero-length refs detaching to separate writable empty refs, readonly external opaque release behavior, shared readonly realloc detach/release timing, realloc grow/shrink/shared/offset detach prefix preservation, zero-size realloc status for existing ordinary buffers and NULL destinations, ordinary alloc-origin realloc replacement, repeated nullable-realloc status, oversized realloc ENOMEM/no-mutation rows for existing and NULL destinations, same-size realloc no-op behavior for shared refs, writable custom-owner refs, and readonly owner refs, replace sharing including same-buffer offset refs, pool-owned destination replacement by ordinary refs, pool-owned destination replacement by NULL, ordinary destination replacement by pool-owned refs, and pool-owned destination replacement by a distinct pool-owned source, nullable `av_buffer_replace`/`av_buffer_unref` rows including NULL-to-NULL replace no-op, unref nulling, and shared-pair unref where one reference becomes NULL while the remaining reference stays live with refcount 1, zero-size default-pool get/reuse rows, legacy `av_buffer_pool_init(size, alloc)` custom-allocator reuse rows with NULL pool opaque data, default-pool allocator fallback/no-opaque reuse rows, `av_buffer_pool_init2` default-allocator fallback plus pool-free rows including the zero-size default-allocator path, huge default-pool get failure plus delayed pool-free rows, custom-pool allocator bytes, shared custom-pool copy-on-write detach/reuse rows, unique custom-pool make-writable/reuse rows, custom-pool replace/realloc detach and reuse rows, readonly custom-pool allocator first-checkout/reuse rows, `av_buffer_pool_buffer_get_opaque` rows, no-clear pool reuse, custom allocator failure return/no-release behavior, nullable `av_buffer_pool_uninit` NULL no-op behavior, LIFO spare reuse/release on pool uninit, pool_free callback timing, and outstanding-buffer release after pool uninit against the Rust `BufferRef`/`BufferPool` model. Newly allocated/grown bytes from `av_realloc` are treated as unspecified. Upstream FFmpeg 8.1.1 FATE has no standalone AVBufferRef/AVBufferPool target in the pinned source tree, so buffer FATE coverage is currently documented as inapplicable until a broader component-level FATE row exercises it through another subsystem. It is wired into `tests/differential/mappings.txt` as `avutil-buffer|oracle-libavutil-buffer`:

```sh
cargo test -p avutil --test buffer_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-buffer --target oracle-libavutil-buffer --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

The ledger also records `tests/fate/mappings.txt:avutil-buffer|local-avutil-unit`.
That local FATE smoke row passes, and upstream FFmpeg 8.1.1 has no standalone
AVBufferRef/AVBufferPool FATE target, so the documented disposition supports
`avutil-buffer` as `fate_pass` without claiming strict completion.

The latest buffer oracle rows add nullable-zero create coverage:
`av_buffer_create(NULL, 0, free, opaque, 0)` returns a valid zero-size ref with
`ref->data == NULL`. `av_buffer_ref()` and `av_buffer_replace()` into a NULL
or already-populated destination preserve the NULL data pointer, opaque owner,
shared storage, refcount 2, and shared non-writability while both refs are live;
replacing an existing destination first releases that destination's prior owner.
Self-replace succeeds with refcount 1 and no early release. Unreffing the
destination restores source writability without release, and the custom release
runs only after the final source unref. Same-size `av_buffer_realloc()`
preserves the NULL data pointer; unique
`av_buffer_make_writable()` is a same-pointer no-op; shared make-writable
detaches only the destination to ordinary non-NULL empty storage; and grow
realloc releases the original custom-owner buffer while clearing destination
opaque lookup.

The latest buffer oracle rows add nullable-zero readonly create coverage:
`av_buffer_create(NULL, 0, free, opaque, AV_BUFFER_FLAG_READONLY)` returns a
valid zero-size ref with `ref->data == NULL`, opaque lookup, and non-writable
state. `av_buffer_ref()` and `av_buffer_replace()` into a NULL destination
preserve the NULL data pointer, opaque lookup, shared storage, refcount 2, and
READONLY non-writability for both refs, and the custom release callback remains
delayed until the final source unref. Replacing an already-populated destination
first releases that destination's prior owner while preserving the same nullable
readonly source-sharing shape. Self-replace succeeds with refcount 1 and no
early release.
`av_buffer_make_writable()` detaches it to ordinary non-NULL writable empty
storage while releasing the original owner. Same-size realloc preserves the NULL
data pointer and readonly owner until unref, while grow realloc releases the
owner immediately and clears destination opaque lookup.

The latest buffer oracle rows add shared nullable-zero grow-realloc coverage:
`av_buffer_realloc()` on one reference of shared
`av_buffer_create(NULL, 0, free, opaque, flags)` storage detaches only the
destination to ordinary writable non-NULL storage. The source remains live with
`ref->data == NULL`, refcount 1, opaque lookup, and writable or readonly state
according to the READONLY flag; the custom release callback runs only after the
final source unref.

The latest buffer oracle rows add shared nullable-zero no-growth coverage:
same-size `av_buffer_realloc()` on writable and READONLY shared nullable-zero
refs returns success without detaching either reference, preserving shared
storage, refcount 2, NULL data pointers, opaque lookup, and no release until the
final source unref. A companion READONLY shared `av_buffer_make_writable()` row
proves destination-only detach to ordinary writable non-NULL empty storage while
the source owner remains live until final unref.

The latest buffer oracle rows add outstanding two-reference pool-uninit coverage: `av_buffer_pool_uninit()` with two refs sharing one custom-pool allocation does not run release or pool-free callbacks at uninit time or after the first unref, then the final unref releases the buffer bytes exactly once before running the pool-free callback.

The latest Rust-only `BufferPool::recycle` rejection lifecycle rows have no direct public FFmpeg C API equivalent; they are therefore covered by unit and deterministic fuzz invariants, while the pinned buffer oracle and mapped `avutil-buffer|oracle-libavutil-buffer` row are rerun as broader regressions.

The latest buffer oracle rows add offset-pool same-size realloc coverage:
`av_buffer_realloc(&ref, ref->size)` on the first offset-visible custom-pool
checkout returns 0 without moving the visible data pointer, changing size or
writability, or dropping pool opaque lookup. Ordinary unref returns the original
backing allocation to the pool, spare reuse rebuilds a normalized zero-offset
view, and final pool uninit releases the original backing bytes.

The latest buffer oracle rows add unique pool make-writable coverage: `av_buffer_make_writable()` on a unique custom-pool ref preserves the same visible data pointer and bytes, keeps pool opaque lookup available, does not run pool release/free callbacks during make-writable or ordinary unref, returns the mutated allocation for spare reuse, and releases the original allocation plus pool owner on final pool uninit. The newest offset-pool row covers the same unique writable make-writable shape when the allocator's visible `data` pointer starts inside the backing allocation: FFmpeg preserves the visible offset for mutation, later returns the full backing allocation to the pool, reuses it as a normalized zero-offset spare, and releases the full mutated backing bytes at pool uninit.

The latest buffer oracle rows add pool-to-pool replace coverage: `av_buffer_replace()` with a pool-owned custom destination and a source from a distinct custom pool returns the destination's original storage to its own pool, shares the source pool storage and opaque data into the destination, waits until both source refs are unreffed before the source pool can reuse storage, and releases each pool's original storage plus owner independently on final pool uninit.

The latest buffer oracle rows add pool-owned source replace coverage: `av_buffer_replace()` with an ordinary destination and pool-owned custom source shares source storage into the destination, preserves pool opaque lookup, does not return storage to the pool when only the original source is unreffed, returns the allocation for reuse when the replacement destination is unreffed, and releases only the original source allocation plus pool owner on final pool uninit.

Recent buffer oracle rows add custom-pool destination replace coverage: `av_buffer_replace()` with a pool-owned custom destination and ordinary source returns the destination's original pool storage immediately for reuse with preserved bytes and pool opaque data, makes the destination an ordinary shared ref to the source with NULL ordinary opaque lookup, and releases only the original pool allocation plus pool owner on final pool uninit.

The latest buffer oracle rows add pool-owned NULL replace coverage: `av_buffer_replace(&dst, NULL)` with a custom-pool destination returns success, nulls the destination, returns the original storage to the pool spare list without release/free callbacks, reuses it with preserved bytes and pool opaque data, and releases the allocation plus pool owner on final pool uninit.

Recent buffer oracle rows add custom-pool realloc coverage: `av_buffer_realloc()` on a pool-owned custom allocation detaches the destination to ordinary writable storage, preserves only the stable copied prefix for comparison, returns the original pool storage for immediate reuse with its pool opaque data, keeps detached destination mutation out of the pool, and releases only the original pool allocation plus pool owner on final pool uninit.

The latest buffer oracle rows add readonly offset custom-pool allocation coverage: a custom `av_buffer_pool_init2()` allocator may return a readonly ref whose visible `data`/`size` point inside a larger backing allocation. FFmpeg preserves the readonly offset range for the first checkout, but after unref/reuse creates a writable spare ref from the original backing pointer and pool size while preserving pool opaque lookup and delaying release of the full backing bytes until pool uninit.

The latest buffer oracle rows add multi-spare pool ordering coverage: after two custom pool buffers are returned, the next two checkouts reuse the second returned buffer first and the first returned buffer second, and final pool uninit releases those spare allocations in LIFO order.

Prior buffer oracle rows pin offset custom-pool allocation, readonly custom-pool allocation, default-free opaque owners, shared-readonly custom-owner make-writable, unique readonly make-writable, unique writable/readonly shrink-realloc, and shared writable/read-only shrink-realloc edges. The separate offset and readonly custom-pool rows prove the same base-pointer reuse and readonly-to-writable reuse behaviors independently.

`crates/fftools/tests/version_oracle.rs` is an ignored oracle harness for `ffmpeg -version`, `ffprobe -version`, the `-hide_banner -version` variant, and the bounded `-buildconf` surface. It checks the pinned tool-version prefix and the libav* ABI versions reported by the oracle against the Rust `ffmpeg-rs`/`ffprobe-rs` banner constants, verifies that `-hide_banner -version` preserves the same version surface as `-version`, verifies that `-buildconf` emits a configuration block without library-version rows and preempts unknown trailing options, verifies that `-version` ignores later unknown options while earlier unknown options fail after printing the banner to stderr, compares accepted/rejected `-loglevel` directive forms on version requests for both tools, and probes real `ffmpeg` repeated diagnostic stderr with an invalid MJPEG image sequence. The buildconf row compares output shape rather than exact configure flags because the Rust implementation reports the selected default-native compatibility profile while the local WSL oracle wrapper includes bootstrap-local flags such as `--prefix` and `--disable-x86asm`. The loglevel rows pin case-sensitive names, standalone `repeat`/`level`/`time`/`datetime` flag directives, `repeat`/`+repeat` versus `-repeat` CLI semantics, compound flag+level values, `+error`, known numeric levels, raw numeric thresholds including compound raw-threshold forms, and representative rejected aliases such as `warn` and `ERROR`; the repeated-diagnostic row pins default indented summary emission plus `repeat+error` duplicate preservation. `FFMPEG_ORACLE` points to the pinned `ffmpeg` binary; `ffprobe` is found through `FFPROBE_ORACLE`, a sibling of `FFMPEG_ORACLE`, or the standard `third_party/ffmpeg-oracle/build/bin/ffprobe(.exe)` path. Run it with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test version_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-version --target oracle-ffmpeg-version --target oracle-ffprobe-version --target oracle-hide-banner-version --target oracle-buildconf-surface --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-version --target oracle-version-unknown-order --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-option-parser --component avutil-logging --target oracle-cli-loglevel-directives --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-option-parser --component avutil-logging --target oracle-cli-repeat-diagnostics --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

The same harness also checks that `--version` is not a clean successful version request for either tool, matching upstream option parsing rather than GNU-style aliases:

```sh
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-version --target oracle-double-dash-version-rejection --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/fftools/tests/ffprobe_mov_oracle.rs` is an ignored oracle harness for a bounded generated MOV `ffprobe` path. It uses pinned FFmpeg 8.1.1 to create a two-frame rawvideo MOV fixture with edit lists disabled, then compares Rust `ffprobe-rs` default output against pinned `ffprobe` for selected shared fields: format counts/name/duration/size/probe score, stream codec/tag/dimensions/timing/count fields, and packet stream index, PTS/DTS, duration, size, and packet flags. It is wired into `tests/differential/mappings.txt` as the shared `oracle-ffprobe-mov-core-fields` target for the mapped `fftools-ffprobe-*` rows. The target is bounded evidence, not full ffprobe JSON or field-complete parity:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test ffprobe_mov_oracle mov_rgb24_ffprobe_core_fields_match_ffmpeg_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component fftools-ffprobe-mov-show-format --target oracle-ffprobe-mov-core-fields --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
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

The harness currently compares Rust constrained `-f rawvideo ... -f rawvideo <file>` output bytes against pinned FFmpeg `-c:v copy -f rawvideo` output for `rgb24` and `gbrp10msble`. It also compares demuxed raw `bgr24` to `-f avi` output against pinned FFmpeg `-c:v rawvideo -f avi` output, proving the current AVI path's zero stream handler, `00dc` chunks, top-down-height demuxing, and four-byte DIB scanline padding. A generated RGB24 AVI input is also compared through normalized `-f framecrc -` streamcopy packet records against pinned FFmpeg `-c:v copy -f framecrc`, proving the bounded AVI demuxer -> Packet -> framecrc muxer path including DIB-padded packet sizes and checksums. It also compares normalized `-f framecrc -`, `-f hash -`, `-f md5 -`, `-f framehash -`, `-f framemd5 -`, and `-f streamhash -` rows for `rgb24`, proving the bounded Rust rawvideo path matches FFmpeg packet stream index, DTS, PTS, duration, payload size, framecrc checksum, whole-output SHA-256 hash digest, whole-output MD5 digest, SHA-256 framehash digest, MD5 framemd5 digest, and accumulated SHA-256 streamhash digest/type label while ignoring FFmpeg/Rust header comments. If `FFMPEG_ORACLE` is unset and `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)` is absent, the ignored tests fail before comparison instead of silently passing.

The same harness is wired into `tests/differential/mappings.txt`, which can be executed through `fate-runner` with `--mappings tests/differential/mappings.txt --oracle-ffmpeg <path> --component fftools-ffmpeg-rawvideo-file-output` for rawvideo file-copy rows, `--component avutil-packet --target oracle-rawvideo-file-output` for the same byte-copy row as packet media-path evidence, `--component fftools-ffmpeg-rawvideo-avi-file-output --target oracle-rawvideo-avi-file-output` for the bgr24 AVI row, `--component avformat-avi-muxer --target oracle-rawvideo-avi-file-output` or `--component avformat-avi-demuxer --target oracle-rawvideo-avi-file-output` for AVI file-output evidence, `--component fftools-ffmpeg-avi-framecrc-null --target oracle-avi-framecrc-records` or `--component avformat-avi-demuxer --component avformat-framecrc-muxer --component avutil-packet --target oracle-avi-framecrc-records` for generated AVI framecrc evidence, `--component avformat-framecrc-muxer --target oracle-rawvideo-framecrc-records` for the rawvideo framecrc record row, `--component avformat-hash-muxer --target oracle-rawvideo-hash-output` for hash, `--component avformat-hash-muxer --target oracle-rawvideo-md5-output` for md5, `--component avformat-framehash-muxer --target oracle-rawvideo-framehash-records` for framehash, `--component avformat-framehash-muxer --target oracle-rawvideo-framemd5-records` for framemd5, or `--component avformat-streamhash-muxer --target oracle-rawvideo-streamhash-records` for streamhash. The current `rgb24`, `gbrp10msble`, bgr24 AVI, generated AVI framecrc, rawvideo framecrc, hash, md5, framehash, framemd5, and streamhash rows have passed locally through the generated WSL oracle wrapper.

`crates/fftools/tests/pcm_oracle.rs` is an ignored oracle harness for the constrained raw `pcm_s16le` CLI path. It compares normalized Rust `-f s16le -ar 48000 -ac 2 ... -f framecrc -` rows against pinned FFmpeg 8.1.1 `-c:a copy -f framecrc -` output, including empty-input, partial-packet, and odd-byte/non-sample-frame-aligned packet rows, compares a bounded 3-channel partial packet row against FFmpeg's default `2.1` channel-layout shape, compares raw `-f s16le` file-output bytes including odd payload bytes against pinned FFmpeg `-c:a copy -f s16le` output, and compares local `-f wav` file-output bytes against pinned FFmpeg `-c:a copy -f wav` output including the default RIFF `LIST/INFO/ISFT` encoder chunk. It is wired into `tests/differential/mappings.txt` as `oracle-pcm-s16le-framecrc-records`, `oracle-pcm-s16le-empty-framecrc-records`, `oracle-pcm-s16le-partial-framecrc-records`, `oracle-pcm-s16le-odd-framecrc-records`, `oracle-pcm-s16le-three-channel-partial-framecrc-records`, `oracle-pcm-s16le-file-output`, `oracle-pcm-s16le-odd-file-output`, and `oracle-pcm-s16le-wav-file-output`:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test pcm_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-pcm-s16le-demuxer --target oracle-pcm-s16le-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-pcm-s16le-demuxer --target oracle-pcm-s16le-empty-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-pcm-s16le-demuxer --target oracle-pcm-s16le-odd-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-pcm-s16le-demuxer --target oracle-pcm-s16le-three-channel-partial-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-pcm-s16le-muxer --target oracle-pcm-s16le-file-output --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-pcm-s16le-muxer --target oracle-pcm-s16le-odd-file-output --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-wav-muxer --target oracle-pcm-s16le-wav-file-output --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/fftools/tests/yuv4mpegpipe_oracle.rs` is an ignored oracle harness for the constrained raw `yuv420p` to local yuv4mpegpipe file-output path and YUV4MPEG2 input to framecrc path. It compares Rust `-f rawvideo -pix_fmt yuv420p -s 2x2 -r 25 ... -f yuv4mpegpipe <file>` bytes against pinned FFmpeg 8.1.1 `-c:v copy -f yuv4mpegpipe` bytes, then parses the oracle output through the Rust `Yuv4MpegDemuxer` to verify the FFmpeg-shaped `A0:0` unspecified sample-aspect and `XYSCSS=420JPEG` header extension path. It also compares normalized Rust `-f yuv4mpegpipe -i <input.y4m> -f framecrc -` streamcopy packet rows against pinned FFmpeg 8.1.1 `-c:v copy -f framecrc`, proving bounded packet count, byte count, DTS, PTS, duration, size, and checksum. The latest metadata row runs pinned ffprobe against empty `X`, `XCOLORRANGE=FULL`, `XCOLORRANGE=LIMITED`, and bogus `XCOLORRANGE` headers, proving the Rust demuxer color-range model for those bounded extension values:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test yuv4mpegpipe_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --target oracle-rawvideo-yuv4mpegpipe-file-output --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-yuv4mpegpipe-demuxer --component avformat-framecrc-muxer --component fftools-ffmpeg-yuv4mpegpipe-framecrc-null --target oracle-yuv4mpegpipe-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-yuv4mpegpipe-demuxer --target oracle-yuv4mpegpipe-xcolorrange-metadata --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/fftools/tests/image2_oracle.rs` is an ignored oracle harness for the constrained image2 file-output and framecrc paths. It compares a valid one-frame PPM input and a two-frame numbered PPM sequence through Rust `-f image2 -framerate 1 ... -f image2 <file-or-pattern>` against pinned FFmpeg 8.1.1 `-c:v copy -f image2`, proving the current bounded Image2 demuxer, Packet path, and Image2 muxer preserve the input bytes for those cases. It also compares normalized single-image, two-frame PPM sequence, start-number padded-width growth, and `%020d` wide-zero-padded sequence `framecrc` rows against pinned FFmpeg `-c:v copy -f framecrc`, proving the current Image2 demuxer, Packet path, and FrameCrc muxer packet records for those bounded inputs. It is wired into `tests/differential/mappings.txt` as `oracle-image2-file-output`, `oracle-image2-sequence-file-output`, `oracle-image2-single-framecrc-records`, `oracle-image2-sequence-framecrc-records`, `oracle-image2-padded-width-growth-framecrc-records`, and `oracle-image2-wide-zero-padded-sequence-framecrc-records` for the relevant image2, framecrc, CLI, and packet rows:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test image2_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-image2-demuxer --component avformat-image2-muxer --component fftools-ffmpeg-image2-file-output --target oracle-image2-file-output --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-image2-demuxer --component avformat-image2-muxer --component fftools-ffmpeg-image2-file-output --target oracle-image2-sequence-file-output --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-image2-demuxer --component avformat-framecrc-muxer --component fftools-ffmpeg-image2-single-framecrc-null --target oracle-image2-single-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-image2-demuxer --component avformat-framecrc-muxer --component fftools-ffmpeg-image2-sequence-framecrc-null --target oracle-image2-sequence-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-image2-demuxer --component avformat-framecrc-muxer --component fftools-ffmpeg-image2-sequence-framecrc-null --target oracle-image2-wide-zero-padded-sequence-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/channel_layout_oracle.rs` is an ignored oracle harness for `ffmpeg -layouts` plus pinned libavutil parser, retype, compare, and default/check vectors. The inventory test compares individual-channel names/descriptions and standard-layout decompositions against the current Rust `Channel::ALL` and `ChannelLayout::known_layouts()` inventories. The parser test compiles a small C helper against the pinned `libavutil.a`, calls `av_channel_layout_from_string()`, `av_channel_layout_describe()`, `av_channel_layout_channel_from_index()`, `av_channel_layout_subset()`, `av_channel_layout_index_from_string()`, and `av_channel_layout_channel_from_string()`, then compares those rows with the current bounded `ChannelLayoutSpec::parse` model. Current parser rows include quoted channel IDs, quoted/custom names, spaced custom keys, escaped `@` names, raw-byte custom names, empty/all-space invalid inputs, leading-plus and octal masks, leading-plus counts, described sparse layouts and count mismatches, base-0 raw `USR` forms including uppercase-`0X` forms such as `USR0X2D`, signed-zero raw IDs, no-conversion `AMBIx` forms, zeroth/signed-zero/hex-order ambisonic forms, plus-hex ambisonic order, zero-order native-mask/list ambisonic extras, sparse and named ambisonic extras, invalid leading/empty separators, invalid ambisonic suffix/nested/octal-junk rows, and broadened lookup strings with both index and raw channel-ID results. The retype test calls `av_channel_layout_retype()` for the current target-CUSTOM, target-NATIVE, target-UNSPEC, target-AMBISONIC, and CANONICAL subset, including raw-user native masks, uppercase-hex raw-user lossy unspecified rows, raw ambisonic extra masks, unspecified/native rejection, `UNK+UNSD` to unspecified rows, ambisonic-to-unspecified rows, named-ambisonic canonical no-op, raw-ambisonic-extra canonical reduction, unspecified canonical no-op, lossy returns, LOSSLESS-flag rejections, no-mutation ENOSYS failures, and post-retype lookup index/channel-ID results. The compare test calls `av_channel_layout_compare()` for native equality/difference, sparse masks, order-sensitive custom maps, name-insensitive custom maps, unknown/unused custom maps, unspecified counts, plus-hex and signed-zero ambisonic equivalence, zero-order ambisonic native-extra equivalence/difference, ambisonic extras, raw native-mask IDs, and uppercase/lowercase raw-user spelling equivalence. The default/check test calls `av_channel_layout_default()` and `av_channel_layout_check()` for nonpositive invalid counts, native defaults selected from `channel_layout_map`, and unspecified positive fallback counts, then compares description, channel order, subset-mask, and string lookup fields with the Rust model. The same Rust test file also contains an ignored `upstream_fate_channel_layout_passes` wrapper for upstream FFmpeg's `fate-channel_layout` target. It runs `make fate-channel_layout` from `FFMPEG_FATE_BUILD_DIR` or the default WSL build cache created by `scripts/bootstrap_ffmpeg_oracle_wsl.sh`, and is wired through `tests/fate/upstream-mappings.txt` as `avutil-channel-layout|fate-channel_layout`. The differential harness is wired into `tests/differential/mappings.txt` as `avutil-channel-layout|oracle-ffmpeg-layouts`; local execution now passes through the pinned WSL oracle wrapper after adding `3.1.2`, matching FFmpeg's `22.2` and `hexadecagonal` decomposition order, validating the current parser/retype/compare/default/check plus lookup vector subset, and supporting the ledger's `fate_pass` status for this still-incomplete component.

`crates/avutil/tests/pixel_format_oracle.rs` is an ignored oracle harness for `ffmpeg -pix_fmts` plus bounded libavutil pixel-format lookup and best-format selection. It checks that every currently modeled `PixelFormat::ALL` descriptor name appears in the oracle inventory with matching component count, integer bits-per-pixel value where the Rust descriptor is exact, `BIT_DEPTHS` component-depth values, paletted flag, and bitstream flag. It also checks hardware identity rows through `PixelFormat::HARDWARE`. The current bitstream rows pin `monow`, `monob`, `rgb4`, `bgr4`, `xv30be`, and `v30xbe` as FFmpeg `....B` formats. The libavutil helper compiles a small C program against the pinned local `libavutil.a` and validates bounded `av_get_pix_fmt()` lookup rows, including accepted `gray8a`, `y32le`, `yf32le`, and `vaapi` names plus rejected `y400a`, uppercase `RGB24`, and unknown names. The same helper validates `av_find_best_pix_fmt_of_2()` rows for the upstream `fate-pixfmt_best` candidate families: base software/hardware formats, semi-planar YUV, packed YUV, subsampled YUV, and depth/chroma tradeoffs. It intentionally does not claim full FFmpeg pixel inventory parity. It is wired into `tests/differential/mappings.txt` as `avutil-pixel-format|oracle-ffmpeg-pix-fmts-subset`. Run it with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p avutil --test pixel_format_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-pixel-format --target oracle-ffmpeg-pix-fmts-subset --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

The upstream FATE wrapper for this slice lives in
`crates/avutil/tests/pixel_format_fate.rs` and runs `make fate-pixfmt_best`
from `FFMPEG_FATE_BUILD_DIR` or the default WSL build cache created by
`scripts/bootstrap_ffmpeg_oracle_wsl.sh`. It is wired through
`tests/fate/upstream-mappings.txt` as `avutil-pixel-format|fate-pixfmt_best`:

```sh
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-pixel-format --target fate-pixfmt_best
```

`crates/avutil/tests/sample_format_oracle.rs` contains ignored oracle harnesses for sample-format parity. The inventory test compares `ffmpeg -sample_fmts` name/depth output against `SampleFormat::ALL`; the libavutil test compiles a small C helper against the pinned local `libavutil.a` and validates native sample-format metadata, table strings, `av_samples_get_buffer_size`, `av_samples_fill_arrays`, `av_samples_alloc`, `av_samples_set_silence`, and `av_samples_copy` vectors. They are wired into `tests/differential/mappings.txt` as `avutil-sample-format|oracle-ffmpeg-sample-fmts` and `avutil-sample-format|oracle-libavutil-sample-format`. Run them with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p avutil --test sample_format_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-sample-format --target oracle-ffmpeg-sample-fmts --target oracle-libavutil-sample-format --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/avutil/tests/color_oracle.rs` is an ignored oracle harness for `ffmpeg -colors` and bounded libavutil `av_parse_color()` vectors. It compares the oracle color-name/RGB table against `NamedColor::ALL`, compiles a test-only C helper against pinned `third_party/ffmpeg-oracle/wsl/lib/libavutil.a`, and compares deterministic named, hex, alpha, and invalid-input parser rows against the Rust `parse_color` model. It is wired into `tests/differential/mappings.txt` as `avutil-color|oracle-ffmpeg-colors`. Run it with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p avutil --test color_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-color --target oracle-ffmpeg-colors --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`crates/fftools/tests/wav_oracle.rs` contains ignored WAV oracle tests for the current demuxer path. `wav_pcm_s16le_generated_md5_matches_ffmpeg_oracle` creates a small PCM s16le WAV fixture through the Rust WAV muxer and compares Rust `-i <generated.wav> -f md5 -` output against pinned FFmpeg 8.1.1 `-c:a copy -f md5 -` output, so it requires only the oracle binary. `wav_pcm_s16le_generated_framecrc_matches_ffmpeg_oracle` uses the same style of generated fixture and compares normalized Rust `-i <generated.wav> -f framecrc -` packet rows against pinned FFmpeg 8.1.1 `-c:a copy -f framecrc`, proving packet count, byte count, DTS, PTS, duration, size, and checksum. The duplicate-data rows generate two `data` chunks and prove pinned FFmpeg 8.1.1 uses the last chunk; when that final chunk is empty, framecrc succeeds with no packet rows. The generated rows are wired into `tests/differential/mappings.txt` as `oracle-wav-generated-md5`, `oracle-wav-generated-framecrc-records`, `oracle-wav-duplicate-data-last-framecrc-records`, and `oracle-wav-duplicate-data-last-empty-framecrc-records` for the relevant WAV, packet, framecrc, MD5, and CLI rows:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test wav_oracle wav_pcm_s16le_generated_md5_matches_ffmpeg_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avformat-wav-demuxer --target oracle-wav-generated-md5 --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --target oracle-wav-generated-md5 --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg cargo test -p fftools --test wav_oracle wav_pcm_s16le_generated_framecrc_matches_ffmpeg_oracle -- --ignored
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-wav-demuxer --component avformat-framecrc-muxer --component fftools-ffmpeg-wav-framecrc-null --target oracle-wav-generated-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-wav-demuxer --component avformat-framecrc-muxer --component fftools-ffmpeg-wav-framecrc-null --target oracle-wav-duplicate-data-last-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/differential/mappings.txt --component avutil-packet --component avformat-wav-demuxer --component avformat-framecrc-muxer --component fftools-ffmpeg-wav-framecrc-null --target oracle-wav-duplicate-data-last-empty-framecrc-records --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

`wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample` uses the FATE PCM s16le sample and is wired through `tests/fate/upstream-mappings.txt` as both `avformat-wav-demuxer|fate-wav-pcm-s16le-md5` and `avutil-packet|fate-wav-pcm-s16le-md5`. Run it directly with:

```sh
FFMPEG_ORACLE=./third_party/ffmpeg-oracle/build/bin/ffmpeg FATE_WAV_SAMPLE=./third_party/fate-samples/audio-reference/luckynight_2ch_44kHz_s16.wav cargo test -p fftools --test wav_oracle wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample -- --ignored
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avformat-wav-demuxer --target fate-wav-pcm-s16le-md5 --samples ./third_party/fate-samples --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
cargo run -p fate-runner -- run --mappings tests/fate/upstream-mappings.txt --component avutil-packet --target fate-wav-pcm-s16le-md5 --samples ./third_party/fate-samples --oracle-ffmpeg ./third_party/ffmpeg-oracle/build/bin/ffmpeg
```

On Windows PowerShell:

```powershell
$env:FFMPEG_ORACLE = ".\third_party\ffmpeg-oracle\build\bin\ffmpeg.exe"
$env:FATE_WAV_SAMPLE = ".\third_party\fate-samples\audio-reference\luckynight_2ch_44kHz_s16.wav"
cargo test -p fftools --test wav_oracle wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample -- --ignored
```

The `fftools` PCM, WAV, and rawvideo oracle harnesses resolve repo-relative `FFMPEG_ORACLE` values against the workspace root, so `fate-runner` can pass `./third_party/ffmpeg-oracle/build/bin/ffmpeg.cmd` even when Cargo executes the test binary from a package directory.

## Source-Checked Notes

The `avutil-color` inventory and parser slice was checked against FFmpeg 8.1.1 `libavutil/parseutils.c` and `fftools/opt_common.c`. The pinned `color_table` contains 140 named RGB colors from `AliceBlue` to `YellowGreen`, including the source spelling `Darkorange`; `av_get_known_color_name` returns indexed table entries; `av_parse_color` strips leading `#` or lowercase `0x` for forced hex parsing, accepts bare all-hex color strings as hex, accepts only six- or eight-digit color hex payloads, treats eight-digit colors as `RRGGBBAA`, applies alpha suffixes after named or hex parsing, parses lowercase-`0x` alpha suffixes as byte-range hex values, parses decimal alpha suffixes as normalized values multiplied by 255 and truncated, and uses case-insensitive named lookup as the fallback branch. `show_colors` prints a `name`/`#RRGGBB` table with lowercase hex. The current Rust model covers that bounded deterministic subset plus typed invalid-input rejection, and the ignored libavutil helper now pins representative parser rows including `red`, `LIGHTGOLDENRODYELLOW`, `#112233`, `0x11223344`, bare hex, alpha suffix override, and invalid empty/short/bad-alpha forms. FFmpeg's nondeterministic `random`/`bikeshed` seed branch, exact C buffer truncation behavior, and oracle-vector calibration for unusual `strtod`/`strtoul` edges remain pending.

The `avutil-error` string slice was checked against FFmpeg 8.1.1 `libavutil/error.h` and `libavutil/error.c`. The pinned header defines `AVERROR(e)` as the negated platform errno value on the current default-native oracle profile, tag-based `AVERROR_*` constants, and `AV_ERROR_MAX_STRING_SIZE`; the pinned C implementation maps the `AVERROR_LIST` entries to user-facing strings, delegates errno-backed descriptions through the platform strerror path, and falls back to `Error number <n> occurred` when no description is found. The ignored `error_oracle` harness now validates the FFmpeg-defined table, representative POSIX errno-backed descriptions, and unknown-code fallback against the pinned local libavutil oracle. This component is complete for the selected default-native profile; non-POSIX platform errno profiles can add their own oracle rows later if needed.

The `avutil-rational` helper slices were checked against FFmpeg 8.1.1 `libavutil/rational.h` and `libavutil/rational.c`. The pinned header documents `av_cmp_q`, `av_nearer_q`, `av_find_nearest_q_idx`, `av_q2intfloat`, and `av_gcd_q`; the pinned implementation treats positive and negative zero-denominator rationals as infinities for comparison, treats `0/0` as indeterminate, scans a `{0,0}`-terminated nearest list while preserving the first candidate on ties, converts rationals to platform-independent IEEE single-precision bit patterns with `av_q2intfloat`, and returns `av_gcd_q` results as raw `gcd(num)/lcm(den)` rationals only when `lcm < max_den`. The ignored `rational_oracle` harness now validates the modeled rational helper surface against pinned local libavutil, including arithmetic helpers, reduction, double conversion, nearest selection, int-float conversion, and rational GCD/default vectors. This component is complete for the selected default-native profile; C-level undefined-overflow cases remain outside the safe Rust API boundary and are rejected with typed errors where exposed.

The `avutil-timebase` helper slices were checked against FFmpeg 8.1.1 `libavutil/mathematics.h` and `libavutil/mathematics.c`. The pinned source defines `AV_ROUND_*`, `AV_ROUND_PASS_MINMAX`, `av_rescale*`, `av_compare_ts`, `av_compare_mod`, `av_rescale_delta`, and `av_add_stable`; the current Rust model covers checked rescale helpers, `av_compare_ts`, `av_compare_mod`, `av_rescale_delta`-style stateful duration-preserving timestamp conversion for nonnegative FFmpeg-int durations, and `av_add_stable`-style timestamp increments with exact positive tick addition, negative-increment no-op behavior through FFmpeg's `m < d` branch, sub-tick no-op behavior, and positive fractional no-drift updates. The ignored `timebase_oracle` harness now validates that modeled surface against pinned local libavutil. This component is complete for the selected default-native profile; C-level undefined-overflow or assertion-only invalid cases remain outside the safe Rust API boundary and are rejected with typed errors where exposed.

The `avutil-channel-layout` slice was checked against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c`. The pinned header defines the currently modeled native channels from front/center/back/side positions through height, wide, downmix, binaural, and 22.2-specific entries including `AV_CHAN_TOP_CENTER`, `AV_CHAN_SURROUND_DIRECT_LEFT`, `AV_CHAN_SURROUND_DIRECT_RIGHT`, `AV_CHAN_BOTTOM_FRONT_CENTER`, `AV_CHAN_BOTTOM_FRONT_LEFT`, `AV_CHAN_BOTTOM_FRONT_RIGHT`, `AV_CHAN_SIDE_SURROUND_LEFT`, `AV_CHAN_SIDE_SURROUND_RIGHT`, `AV_CHAN_TOP_SURROUND_LEFT`, and `AV_CHAN_TOP_SURROUND_RIGHT`; it maps their `AV_CH_*` macros to native mask bits including `1 << 11`, `1 << 33`, `1 << 34`, `1 << 38`, `1 << 39`, `1 << 40`, `1 << 41`, `1 << 42`, `1 << 43`, and `1 << 44` for `TC`, `SDL`, `SDR`, `BFC`, `BFL`, `BFR`, `SSL`, `SSR`, `TTL`, and `TTR`. The pinned source maps those short names to `top center`, `surround direct left`, `surround direct right`, `bottom front center`, `bottom front left`, `bottom front right`, `side surround left`, `side surround right`, `top surround left`, and `top surround right`, and `channel_layout_map` names the currently modeled layouts from `mono` through `downmix` plus `22.2`, including `3.1.2`. The same header defines `AV_CHAN_NONE = -1`, `AV_CHAN_UNUSED = 0x200`, `AV_CHAN_UNKNOWN = 0x300`, and ambisonic custom-channel IDs from `AV_CHAN_AMBISONIC_BASE = 0x400` through `AV_CHAN_AMBISONIC_END = 0x7ff`, where ACN is `id - AV_CHAN_AMBISONIC_BASE`. The pinned source formats those channel IDs as `NONE`, `UNSD`, `UNK`, `AMBI<n>`, or `USR<raw>` and descriptions as `none`, `unused`, `unknown`, `ambisonic ACN <n>`, or `user <raw>`. The pinned `AVChannelCustom` shape stores an `AVChannel id`, `char name[16]`, and opaque pointer; `av_channel_layout_custom_init` rejects nonpositive channel counts and initializes every custom entry to `AV_CHAN_UNKNOWN`; `av_channel_layout_check` rejects custom layouts with a null map or any `AV_CHAN_NONE` entry; custom channel-index lookup returns the first matching raw channel ID; string lookup supports custom names as `CH@name` or `@name`; and custom descriptions print `<n> channels (CH[@name]+...)` when a custom map cannot be reduced to a named/native description. Parser-calibrated custom-name rows prove FFmpeg stores non-UTF-8 custom-name bytes in that 16-byte array, emits those bytes from `av_channel_layout_describe`, supports byte exact `CH@name` and `@name` lookup, and truncates parsed custom names to 15 stored bytes. The pinned `masked_description` helper accepts a custom map as a native mask only when every remaining entry is a native channel ID below 63 and native IDs appear in strictly increasing order; `canonical_order` refuses native reduction for custom maps with any custom names, returns `UNSPEC` for all-unknown maps, selects `NATIVE` when `masked_description` succeeds, and can select `AMBISONIC` when `av_channel_layout_ambisonic_order` accepts a complete standard-order ambisonic prefix. For custom maps, `av_channel_layout_ambisonic_order` requires all ambisonic channels to appear before any non-ambisonic channel, requires each ambisonic ACN to equal its map index, rejects missing or incomplete ambisonic orders, and describes valid maps as `ambisonic <order>` plus optional trailing-channel descriptions. `av_channel_layout_compare` first rejects different channel counts, treats one unspecified layout as unequal and two unspecified layouts as equal, compares masks directly for same-order native or ambisonic layouts, and otherwise compares `av_channel_layout_channel_from_index` results for each position, ignoring custom names. `av_channel_layout_subset` returns `mask & layout->u.mask` for native and ambisonic layouts, and for custom layouts scans native channel mask bits 0 through 63 and includes a bit when `av_channel_layout_index_from_channel` finds that raw channel ID in the custom map. `av_channel_layout_channel_from_index` returns native channels in mask-bit order, returns ambisonic channels as `AMBI0..AMBI<N>` followed by native mask extras, returns custom map entries in map order, and returns `AV_CHAN_NONE` for out-of-range or unsupported order values; `av_channel_layout_index_from_channel` returns the first map or mask-bit index for a channel and rejects absent or `AV_CHAN_NONE` channels; `av_channel_layout_index_from_string` applies `av_channel_from_string` lookup for native layouts and additionally supports `CH@name`/`@name` custom map lookup before falling back to raw channel lookup for custom layouts; and `av_channel_layout_channel_from_string` combines string lookup with index lookup. The Rust custom-map and explicit-ambisonic helpers mirror bounded native canonicalization, byte-preserving custom names, custom-map ambisonic prefix detection/description, current native/ambisonic/custom channel-by-index equivalence, current native/ambisonic/custom subset-mask extraction, and current native/custom index/string lookup for the currently modeled layouts. The current oracle vectors also pin the base-0 raw `USR` and no-conversion `AMBI` parser-facing string behavior plus raw-user native and raw-ambisonic-extra retyping. Broader unspecified parsing/retyping and full `AV_CHANNEL_ORDER_AMBISONIC` layout semantics remain for later oracle-backed slices. The `22.2` macro is `AV_CH_LAYOUT_9POINT1POINT6 | AV_CH_BACK_CENTER | AV_CH_LOW_FREQUENCY_2 | AV_CH_TOP_FRONT_CENTER | AV_CH_TOP_CENTER | AV_CH_TOP_BACK_CENTER | AV_CH_BOTTOM_FRONT_CENTER | AV_CH_BOTTOM_FRONT_LEFT | AV_CH_BOTTOM_FRONT_RIGHT`, with `AV_CHANNEL_LAYOUT_22POINT2` declaring 24 channels. The Rust model exposes those source-checked channel ID, native-layout, explicit ambisonic native-extra-layout, count-only unspecified-layout, and initial custom-map shapes, and its known channel/layout inventory now passes `ffmpeg -layouts`; full `av_channel_layout_from_string()` grammar, broader custom/native/ambisonic/unspecified retyping, and full ambisonic layout order semantics remain for later oracle-backed slices.

The latest channel-layout lookup oracle slice extends the parser and retype helper rows with direct `av_channel_layout_channel_from_string()` raw-ID output alongside the existing index-from-string fields. This pins native, custom-name, byte custom-name, unspecified, explicit ambisonic, and raw native-mask lookup behavior without adding runtime FFmpeg linkage.

The latest default-layout slice additionally oracle-checks that `av_channel_layout_default` scans `channel_layout_map` in source order and chooses the first layout whose `nb_channels` matches the requested count, otherwise producing an unspecified-order layout with that count. `av_channel_layout_check` rejects nonpositive counts and accepts unspecified layouts without a union payload. The pinned rows cover invalid `-1` and `0`, native default counts 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, and 24, plus unspecified fallback counts 9, 11, 13, 15, 17, 23, 25, and 64. `av_channel_layout_describe` formats unspecified layouts as `<n> channels`; `av_channel_layout_compare` compares two unspecified layouts equal only when the channel counts match and treats one unspecified plus one non-unspecified layout as unequal; and `av_channel_layout_subset` returns no native bits for unspecified layouts. `ChannelLayout::default_for_count` remains a native-only helper for the modeled source-order defaults, while `ChannelLayoutSpec::default_for_count` returns those native defaults for 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, and 24 channels and returns a count-only `UnspecifiedChannelLayout` for other positive counts. AudioFrame and avformat AudioStreamParameters now store `ChannelLayoutSpec`; their legacy `channel_layout()` accessors remain native-only compatibility helpers.

The latest parser slice source-checks `av_channel_layout_from_string()` channel-list, numeric-mask, count-suffix, and ambisonic branch behavior. Pinned FFmpeg checks exact case-sensitive named layouts before the later parser branches, supports an `ambisonic <order>` branch with optional `+extra` layout, tries channel lists and `N channels (<list>)` descriptions, then parses numeric masks, `Nc`, `NC`, and `N channels`. The `parse_channel_list` path uses `av_opt_get_key_value` with `@` channel-name separators and `+` pair separators, resolves each channel token with the case-sensitive `av_channel_from_string`, stores optional `AVChannelCustom.name` values, initially creates `AV_CHANNEL_ORDER_CUSTOM`, and then canonical retyping can reduce nameless native-channel maps to native masks or all-unknown maps to unspecified order. Source checking against pinned `libavutil/opt.c` and `libavutil/avstring.c` confirms the channel-list tokenizer also skips FFmpeg ASCII whitespace, treats a missing key as an implicit channel token, accepts single-quoted token segments, backslash-escaped separators, unescaped `@` in the value after the first key separator, and a final trailing `+` after a valid token, while leading or repeated empty tokens such as `+FL` and `FL++FR` still fail after channel-token resolution. The current Rust `ChannelLayoutSpec` parser mirrors the bounded native/custom channel-list subset by resolving lists such as `FL+FR` and quoted IDs such as `'FL'+FR` to named native layouts when possible, preserving lists such as `FL`, `FL+`, `FL+FC`, and `2 channels (FL+FC)` as exact `NativeChannelMaskLayout` values, preserving named or otherwise non-canonical maps such as `FL@Left+FR@Right`, `FL@Left\+Right+FR`, `FL@Left\@Name+FR`, `FL@Left@Again`, `FL+FL`, `UNK+UNSD`, raw-byte custom names such as `FL@\xff+FR`, and `UNK+UNSD+AMBI2@Height+USR2048@Vendor` as first-class custom specs, and reducing all-UNK nameless maps to `UnspecifiedChannelLayout`; it rejects empty/all-space inputs, lowercase or otherwise mismatched channel/layout names such as `fl+fr`, `unk+unk`, `ambi0`, `usr0`, and `STEREO`, mismatched described counts, and nested ambisonic extras rather than falling through to the ergonomic `ChannelLayout::parse` helper. The current Rust ambisonic branch accepts bounded orders that fit the AMBI0..AMBI1023 range, stores no-extra and native-extra forms such as `ambisonic 0`, `ambisonic 1`, `ambisonic 0x1+stereo`, `ambisonic 1+stereo`, `ambisonic 1+FL+FC`, `ambisonic 1+0x5`, signed-zero `ambisonic -0`, and no-conversion `ambisonic +stereo` / `ambisonic -0+stereo` as explicit `AmbisonicChannelLayout` order/native-mask specs where applicable, preserves named/custom extras such as `+FL@Left+FR@Right` as custom AMBI-prefix maps, and rejects real negative orders, trailing junk, octal-junk orders, unspecified extras, or nested ambisonic extras. The mask branch uses `strtoull(str, &end, 0)`, requires no parse overflow, full-string consumption, no `-` anywhere in the input, and a nonzero mask, then initializes an `AV_CHANNEL_ORDER_NATIVE` layout with that exact mask. The current Rust parser resolves modeled base-0 masks such as `0x3`, `3`, `03`, leading-whitespace ` 0x3`, and ` +0x3` to named native layouts, preserves arbitrary FFmpeg-valid nonzero masks such as `0x5` and `0x8000000000000000` as exact `NativeChannelMaskLayout` values, and rejects zero or trailing-junk mask forms such as `0x0` and `0x3 `. The lowercase `Nc` branch calls `av_channel_layout_default` and returns success only when the resulting default order is native, so `2c`, leading-whitespace ` 2c`, and `10c` are accepted by the current Rust model while `9c` and trailing-junk `2c ` are rejected. Uppercase `NC` and `N channels` produce `AV_CHANNEL_ORDER_UNSPEC` count-only layouts for positive counts, including leading-plus forms such as `+2C` and ` +2 channels`, while trailing-junk forms such as `2C `, `2 channels `, and `2 channels (FL+FR) ` reject after full-consumption checks. The ignored pinned libavutil parser oracle now validates the current bounded row set for native names/lists/masks, count forms, described custom forms, escaped/custom/raw-byte names, overlong parser truncation, duplicate and trailing-separator custom maps, ambisonic native-extra forms, invalid exact-case/trailing-junk forms, described strings, channel order, stereo subset masks, and representative string or byte lookup results. Full `AV_CHANNEL_ORDER_AMBISONIC` comparison/index string parity, long-tail parser-vector coverage, and broad retyping remain pending.

The latest ambisonic lookup slice uses the pinned `av_channel_layout_channel_from_index`, `av_channel_layout_index_from_channel`, `av_channel_layout_index_from_string`, and `av_channel_layout_channel_from_string` behavior for the bounded explicit ambisonic surface: ambisonic ACNs map to the leading indexes, native extra-mask channels follow in mask-bit order, canonical strings map through the same lookup, and absent/custom-name/invalid lookups fail without producing channels. Broader `AV_CHANNEL_ORDER_AMBISONIC` retyping and oracle-vector calibration remain pending.

The latest ambisonic retype slice uses the pinned `canonical_order` and `av_channel_layout_retype(..., AV_CHANNEL_ORDER_AMBISONIC, CANONICAL)` shape for the bounded lossless custom-map subset: custom names prevent lossless retyping, a complete standard-order `AMBI0..AMBI<N>` prefix is required, and trailing extras must be native raw channel IDs in strictly increasing mask-bit order. The Rust model now maps matching nameless channel-list parses to explicit `AmbisonicChannelLayout` specs and keeps named, unknown-extra, incomplete, or out-of-order forms as custom maps.

The latest custom-target retype slice uses the pinned `av_channel_layout_retype(..., AV_CHANNEL_ORDER_CUSTOM, ...)` shape for the current bounded variants. FFmpeg initializes a custom map with UNKNOWN entries for count-only UNSPEC layouts, and otherwise fills each custom entry from `av_channel_layout_channel_from_index` before replacing the layout. The Rust `ChannelLayoutSpec::to_custom_layout` mirrors that bounded behavior for native, arbitrary native-mask, explicit ambisonic, custom, and count-only unspecified specs without claiming the remaining lossy retype flags or complete order coverage.

The latest native/unspecified-target retype slice uses the pinned `av_channel_layout_retype(..., AV_CHANNEL_ORDER_NATIVE, flags)` and `av_channel_layout_retype(..., AV_CHANNEL_ORDER_UNSPEC, flags)` shape for the current bounded variants. FFmpeg treats already-target-order layouts as no-ops; accepts target-NATIVE from custom order only when `masked_description` can form a strictly ordered native mask, returning a lossy result only when custom names are dropped; and accepts target-UNSPEC from any non-target order when lossy conversion is allowed, with custom all-UNKNOWN/no-name maps remaining lossless. The Rust helpers `ChannelLayoutSpec::retype_to_native_order` and `ChannelLayoutSpec::retype_to_unspecified_order` mirror those bounded branches and expose the returned lossy bit through `ChannelLayoutRetypeResult`, while the lossless wrapper methods now reject impossible or lossless-forbidden conversions with Rust `Unsupported` / FFmpeg-shaped `ENOSYS` rather than `EINVAL`.

The latest ambisonic/canonical-target retype slice uses the pinned `av_channel_layout_retype(..., AV_CHANNEL_ORDER_AMBISONIC, flags)` and `AV_CHANNEL_LAYOUT_RETYPE_FLAG_CANONICAL` branches. FFmpeg accepts target-AMBISONIC from custom order only when `av_channel_layout_ambisonic_order` finds a complete standard-order AMBI prefix and `masked_description` can form a native extra-channel mask after that prefix; custom names only make that conversion lossy. With CANONICAL, FFmpeg first computes `canonical_order`, which refuses to drop custom names, turns all-UNKNOWN nameless maps into UNSPEC, turns strict native maps into NATIVE, turns strict ambisonic maps into AMBISONIC, and leaves unreducible custom maps as CUSTOM no-ops. The ignored pinned retype oracle now validates the current bounded target-CUSTOM, target-NATIVE, target-UNSPEC, target-AMBISONIC, and CANONICAL rows, including unspecified-to-native rejection, `UNK+UNSD` to unspecified lossy and lossless-reject rows, ambisonic-to-unspecified lossy and lossless-reject rows, named-ambisonic canonical no-op, raw-ambisonic-extra canonical reduction, unspecified canonical no-op, and ENOSYS no-mutation failures; long-tail retype oracle-vector coverage is still pending.

The latest raw-channel-mask retype slice source-checks the pinned `masked_description` condition `ch >= 0 && ch < 63` and `mask < (1ULL << ch)`: custom channel IDs do not have to be named `AV_CHAN_*` entries to reduce to native-mask order. The Rust model now lets nameless custom lists such as `USR45+USR46` retype to exact `NativeMask` values and lets complete AMBI prefixes with `USR45`-style trailing extras retype to explicit ambisonic native-extra masks, while `USR63`, out-of-range `USR2048`, duplicates, and descending raw-ID order still reject native/ambisonic retyping.

The latest explicit-ambisonic raw-extra lookup slice applies the same raw-ID native-mask principle to explicit `AmbisonicChannelLayout` lookup for the bounded Rust model. Layouts such as `ambisonic 1+0x200000000000` now expose the extra channel as canonical `USR45` through index lookup, string lookup, channel-by-index, subset-mask, and custom-equivalence helpers, while noncanonical direct user IDs such as `User(0)`, absent raw extras, special unknown/unused IDs, and custom-name strings still reject.

The latest channel-string parser slice source-checks pinned `av_channel_from_string`. Native short names and `UNK`/`UNSD` are exact case-sensitive matches; `AMBI` names use `strtol(..., base 0)` after the uppercase prefix with a null end pointer and only range-check the parsed ACN, so trailing text and no-conversion suffixes such as `AMBIx`, `AMBI+`, and signed-zero `AMBI-0tail` resolve as ACN 0 while real negative ACNs reject; `USR` names use the same base-0 parser after the uppercase prefix, require full-string consumption and a nonnegative raw ID, and accept the bare `USR` no-conversion case plus signed-zero `USR-0` as raw ID 0. The Rust model keeps canonical generated names strict while routing layout parsing and channel string lookup through this parser-facing form, covering inputs such as `USR0x2d+USR056`, `USR`, `USR+`, `USR-0`, `USR0`, `USR055`, `USR+45`, `AMBIx`, and `AMBI0x1tail`, and rejecting lowercase forms such as `fl`, `ambi0`, and `usr0`.

The `avutil-sample-format` table-string, buffer-layout, fill-array, allocation, silence-fill, and sample-copy slices were checked against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` and its implementations of `av_get_sample_fmt_name`, `av_get_sample_fmt`, `av_get_alt_sample_fmt`, `av_get_packed_sample_fmt`, `av_get_planar_sample_fmt`, `av_get_sample_fmt_string`, `av_get_bytes_per_sample`, `av_sample_fmt_is_planar`, `av_samples_get_buffer_size`, `av_samples_fill_arrays`, `av_samples_alloc`, `av_samples_alloc_array_and_samples`, `av_samples_set_silence`, and `av_samples_copy`. The upstream table-string helper prints the fixed header `name   depth` for negative format values and formats valid native sample formats as a 6-column left-aligned name, three spaces, a 2-column depth, and a trailing space. The upstream buffer-size helper rejects missing sample size, nonpositive sample count, and nonpositive channel count; treats `align=0` as automatic 32-sample padding with byte alignment 1; aligns packed line size as `samples * bytes_per_sample * channels`; aligns planar line size as `samples * bytes_per_sample`; and returns total size as one line for packed data or one line per channel for planar data. The upstream fill-array helper writes a single packed plane or one line-size-spaced pointer per planar channel, returns the computed buffer size, and supports a null input buffer as size-only calculation. The upstream allocation helper allocates one contiguous buffer, fills the plane pointer array, then applies silence for the originally requested sample count; `av_samples_alloc_array_and_samples` first allocates the pointer array and delegates to the same sample allocation path. The upstream silence helper computes packed byte spans with all channels, planar byte spans per channel plane, fills `u8`/`u8p` samples with `0x80`, and fills all other native formats with `0x00`. The upstream copy helper computes packed byte spans with all channels, planar byte spans per channel plane, multiplies source and destination sample offsets by that block alignment, then uses overlap-aware movement when source and destination ranges overlap. The Rust model exposes the table-string shape through `SampleFormat::sample_fmt_string_header` and `SampleFormat::sample_fmt_string`, exposes those storage values through `SampleBufferLayout`, exposes contiguous-buffer plane ranges and safe split helpers through `SampleArrayLayout`, exposes owned contiguous allocation through `SampleAllocation`, exposes silence byte/range/fill helpers through `SampleSilenceRange`, exposes copy byte spans and safe copy helpers through `SampleCopyRange`, and rejects invalid or overflowing bounded Rust inputs before mutation instead of exposing implementation-defined C pointer or integer-overflow behavior. Unlike FFmpeg's raw `av_malloc` tail storage, Rust allocation keeps alignment padding and auto-aligned tail bytes deterministically zeroed.

The latest ambisonic lookup slice uses the pinned `av_channel_layout_channel_from_index`, `av_channel_layout_index_from_channel`, `av_channel_layout_index_from_string`, and `av_channel_layout_channel_from_string` behavior for the bounded explicit ambisonic surface: ambisonic ACNs map to the leading indexes, native extra-mask channels follow in mask-bit order, canonical strings map through the same lookup, and absent/custom-name/invalid lookups fail without producing channels. Broader `AV_CHANNEL_ORDER_AMBISONIC` retyping and oracle-vector calibration remain pending.

The `pal8` rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h`, `libavutil/pixdesc.c`, `libavutil/imgutils.c`, and `libavcodec/rawdec.c`. FFmpeg's descriptor marks `pal8` as paletted and alpha-bearing, defines 256 RGB32 palette entries for 1024 palette bytes, includes the palette in image buffer sizing, and lets ordinary rawvideo packets carry only the index plane while the decoder supplies palette state separately. The `av_frame_apply_cropping()` oracle rows further show that FFmpeg crops the one-byte index plane with default alignment rounding or exact `AV_FRAME_CROP_UNALIGNED` offsets while leaving the palette plane offset at zero. The Rust model currently covers the raw packet index plane, constants, and this bounded crop pointer behavior only.

The 8-bit planar YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuva420p` as 20 bpp with log2 chroma `(1,1)`, `yuva422p` as 24 bpp with `(1,0)`, and `yuva444p` as 32 bpp with `(0,0)`, all with four 8-bit planes and full-resolution alpha.

The high-bit-depth planar YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuva420p9*` as 22.5 bpp (`45/2`) with log2 chroma `(1,1)`, `yuva422p9*` as 27 bpp with `(1,0)`, `yuva444p9*` as 36 bpp with `(0,0)`, `yuva420p10*` as 25 bpp with `(1,1)`, `yuva422p10*` as 30 bpp with `(1,0)`, `yuva444p10*` as 40 bpp with `(0,0)`, `yuva422p12*` as 36 bpp with `(1,0)`, `yuva444p12*` as 48 bpp with `(0,0)`, `yuva420p16*` as 40 bpp with `(1,1)`, `yuva422p16*` as 48 bpp with `(1,0)`, and `yuva444p16*` as 64 bpp with `(0,0)`, all using two stored bytes per component sample and a full-resolution alpha plane. FFmpeg 8.1.1 does not define `yuva420p12*`.

The semi-planar NV rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `nv16` as 8-bit 4:2:2 with log2 chroma `(1,0)` and 16 bpp, `nv20le`/`nv20be` as 10-bit 4:2:2 with `(1,0)` and 20 bpp using two stored bytes per component sample, and `nv24`/`nv42` as 8-bit 4:4:4 with `(0,0)` and 24 bpp. All five are two-plane formats with one luma plane and one interleaved chroma plane; `nv42` swaps the U/V component order relative to `nv24`.

The high-bit semi-planar P-family rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `p010*`, `p012*`, and `p016*` as 4:2:0 two-plane formats with log2 chroma `(1,1)` and 15/18/24 bpp; `p210*`, `p212*`, and `p216*` as 4:2:2 with `(1,0)` and 20/24/32 bpp; and `p410*`, `p412*`, and `p416*` as 4:4:4 with `(0,0)` and 30/36/48 bpp. All use one luma plane plus one interleaved chroma plane with two stored bytes per component sample.

The high-bit packed YUV 4:2:2 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `y210*`, `y212*`, and `y216*` as one-plane packed 4:2:2 YUV with log2 chroma `(1,0)`, 10/12/16-bit component descriptors, 20/24/32 logical average bpp, and four stored bytes per pixel; `be` variants carry the big-endian descriptor flag.

The packed UYYVYY411 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h`, `libavutil/pixdesc.c`, and `libavutil/imgutils.c`. The pinned enum comment describes packed YUV 4:1:1 storage as `Cb Y0 Y1 Cr Y2 Y3`; the descriptor names it `uyyvyy411`, sets log2 chroma `(2,0)`, 12 logical bpp, one plane, three 8-bit components, and component steps/offsets that make image line sizing use `ceil(width / 4) * 6` bytes rather than requiring width to be divisible by four. The `av_frame_apply_cropping()` oracle rows further show default nonzero-left crop returns `AVERROR_BUG`, while `AV_FRAME_CROP_UNALIGNED` uses the descriptor first-component step of four bytes for a one-pixel left offset.

The packed AYUV64 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `ayuv64le` and `ayuv64be` as one-plane packed AYUV 4:4:4:4 with four 16-bit components, alpha, log2 chroma `(0,0)`, 64 bpp, and eight stored bytes per pixel; the `be` variant carries the big-endian descriptor flag.

The packed XYZ12 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `xyz12le` and `xyz12be` as one-plane packed XYZ 4:4:4 with three 12-bit components, log2 chroma `(0,0)`, 36 bpp, six stored bytes per pixel, and lower four bits of each two-byte component unused; the `be` variant carries the big-endian descriptor flag.

The packed X2RGB10/X2BGR10 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `x2rgb10le`, `x2rgb10be`, `x2bgr10le`, and `x2bgr10be` as one-plane packed RGB-class 10:10:10 formats with three exposed color components, log2 chroma `(0,0)`, 30 bpp, four stored bytes per pixel, and an unused two-bit X lane; the `be` variants carry the big-endian descriptor flag.

The packed X/V YUV 4:4:4 rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `xv30le`/`xv30be` and `v30xle`/`v30xbe` as one-plane packed 10-bit 4:4:4 YUV-family formats with three exposed components, no alpha, log2 chroma `(0,0)`, 30 bpp, and four stored bytes per pixel; `xv36le`/`xv36be` have 12-bit components and 36 logical bpp but use an eight-byte descriptor step that stores the undefined X lane; `xv48le`/`xv48be` have 16-bit components, 48 bpp, and eight stored bytes per pixel. The X lane is undefined storage padding rather than alpha, and `be` variants carry the big-endian descriptor flag. The same source check shows `AV_PIX_FMT_FLAG_BITSTREAM` on `xv30be` and `v30xbe`, which is why pinned `av_frame_apply_cropping()` rows for those formats take the right/bottom-only bitstream branch and preserve top/left crop.

The Bayer rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h`, `libavutil/pixdesc.c`, and `ffmpeg -pix_fmts`. The pinned enum defines `bayer_bggr8`, `bayer_rggb8`, `bayer_gbrg8`, `bayer_grbg8`, and 16-bit little/big-endian variants for each pattern. The pinned descriptors use one plane, three exposed components, no alpha, log2 chroma `(0,0)`, RGB plus Bayer descriptor flags, and the big-endian flag on `be` variants. `ffmpeg -pix_fmts` reports Bayer `BIT_DEPTHS` as `2-4-2` for 8-bit variants and `4-8-4` for 16-bit variants, not uniform 8/16-bit exposed components. The Rust model preserves the CFA pattern, byte order, and component-depth vectors in the pixel-format name and sizes payloads as one byte per pixel for 8-bit variants or two bytes per pixel for 16-bit variants.

The packed 8-bit YUV/YUVA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `vuya`, `ayuv`, and `uyva` as one-plane packed 4:4:4:4 YUV-family formats with four 8-bit components, alpha, log2 chroma `(0,0)`, 32 bpp, and four stored bytes per pixel; `vuyx` has three exposed 8-bit components, no alpha, 24 logical bpp, and four stored bytes per pixel with the X lane undefined; `vyu444` has three exposed 8-bit components, no alpha, log2 chroma `(0,0)`, 24 bpp, and three stored bytes per pixel.

The packed floating gray+alpha rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yaf16le`/`yaf16be` as one-plane packed YA formats with two 16-bit IEEE-754 half-float components, alpha, float metadata, log2 chroma `(0,0)`, 32 bpp, and four stored bytes per pixel; `yaf32le`/`yaf32be` are the same shape with two 32-bit IEEE-754 single-precision components, 64 bpp, and eight stored bytes per pixel.

The planar floating GBR rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `gbrpf16le`/`gbrpf16be` as three-plane GBR formats with 16-bit IEEE-754 half-float components, RGB plus planar plus float descriptor flags, no alpha, log2 chroma `(0,0)`, 48 bpp, and two stored bytes per component sample; `gbrpf32le`/`gbrpf32be` use the same layout with 32-bit IEEE-754 single-precision components, 96 bpp, and four stored bytes per component sample.

The packed floating RGB rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `rgbf16le`/`rgbf16be` as one-plane packed RGB formats with 16-bit IEEE-754 half-float components, RGB plus float descriptor flags, no alpha, log2 chroma `(0,0)`, 48 bpp, and six stored bytes per pixel; `rgbf32le`/`rgbf32be` use the same packed shape with 32-bit IEEE-754 single-precision components, 96 bpp, and twelve stored bytes per pixel.

The packed floating RGBA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `rgbaf16le`/`rgbaf16be` as one-plane packed RGBA formats with four 16-bit IEEE-754 half-float components, RGB plus alpha plus float descriptor flags, log2 chroma `(0,0)`, 64 bpp, and eight stored bytes per pixel; `rgbaf32le`/`rgbaf32be` use the same packed shape with 32-bit IEEE-754 single-precision components, 128 bpp, and sixteen stored bytes per pixel.

The packed 32-bit integer RGB/RGBA rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `rgb96le`/`rgb96be` as one-plane packed RGB formats with three 32-bit integer components, RGB descriptor flags, no alpha, log2 chroma `(0,0)`, 96 bpp, and twelve stored bytes per pixel; `rgba128le`/`rgba128be` use the same packed integer shape with four 32-bit components, alpha metadata, 128 bpp, and sixteen stored bytes per pixel. The source check also confirmed that `Y410`/`Y412`/`Y416` are documented conceptual aliases in comments around `xv30`/`xv36`/`xv48`, not separate FFmpeg 8.1.1 `AV_PIX_FMT_*` descriptors.

The packed floating and 32-bit RGB/RGBA `av_frame_apply_cropping()` slice was checked against pinned FFmpeg 8.1.1 libavutil through `crates/avutil/tests/frame_oracle.rs`. Covered rows are `rgbf16le`, `rgbf16be`, `rgbf32le`, `rgbf32be`, `rgbaf16le`, `rgbaf16be`, `rgbaf32le`, `rgbaf32be`, `rgb96le`, `rgb96be`, `rgba128le`, and `rgba128be`. FFmpeg returns `AVERROR_BUG` without mutating the frame for default nonzero-left crop on these multi-byte packed formats. With `AV_FRAME_CROP_UNALIGNED`, FFmpeg applies exact six-, eight-, twelve-, or sixteen-byte-per-pixel left offsets and resets crop fields on success.

The MSB-aligned planar YUV444/GBR rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuv444p10msble`/`yuv444p10msbbe` and `gbrp10msble`/`gbrp10msbbe` as three-plane 4:4:4 formats with 10-bit components, 30 bpp, two stored bytes per component sample, and descriptor component shifts that place valid bits in the high bits. The `yuv444p12msble`/`yuv444p12msbbe` and `gbrp12msble`/`gbrp12msbbe` descriptors have the same layout with 12-bit components and 36 bpp. The `be` variants carry the big-endian descriptor flag, and the native header aliases map `AV_PIX_FMT_YUV444P10MSB`, `AV_PIX_FMT_YUV444P12MSB`, `AV_PIX_FMT_GBRP10MSB`, and `AV_PIX_FMT_GBRP12MSB` to the target-platform-endian variant.

The 9-bit, 10-bit, 12-bit, 14-bit, and 16-bit planar YUV rawvideo slice was checked against FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`. The pinned descriptors define `yuv420p9*` as 13.5 bpp (`27/2`) with log2 chroma `(1,1)`, `yuv422p9*` as 18 bpp with `(1,0)`, `yuv444p9*` as 27 bpp with `(0,0)`, `yuv420p10*` as 15 bpp with `(1,1)`, `yuv422p10*` as 20 bpp with `(1,0)`, `yuv440p10*` as 20 bpp with `(0,1)`, `yuv444p10*` as 30 bpp with `(0,0)`, `yuv420p12*` as 18 bpp with `(1,1)`, `yuv422p12*` as 24 bpp with `(1,0)`, `yuv440p12*` as 24 bpp with `(0,1)`, `yuv444p12*` as 36 bpp with `(0,0)`, `yuv420p14*` as 21 bpp with `(1,1)`, `yuv422p14*` as 28 bpp with `(1,0)`, `yuv444p14*` as 42 bpp with `(0,0)`, `yuv420p16*` as 24 bpp with `(1,1)`, `yuv422p16*` as 32 bpp with `(1,0)`, and `yuv444p16*` as 48 bpp with `(0,0)`, all using two stored bytes per component sample.

The high-bit planar YUV `av_frame_apply_cropping()` slice was checked against pinned FFmpeg 8.1.1 libavutil through `crates/avutil/tests/frame_oracle.rs`. The covered rows are the modeled 9/10/12/14/16-bit planar YUV families, 10/12-bit 4:4:0 rows, and `yuv444p10msb*`/`yuv444p12msb*`. FFmpeg returns `AVERROR_BUG` without mutating the frame for default nonzero-left crop on these two-byte-per-sample planar formats. With `AV_FRAME_CROP_UNALIGNED`, FFmpeg applies exact luma offsets, subsampled chroma offsets derived from the format chroma geometry, floor-shifted visible chroma dimensions, and resets crop fields on success.

The planar GBR `av_frame_apply_cropping()` slice was checked against pinned FFmpeg 8.1.1 libavutil through `crates/avutil/tests/frame_oracle.rs`. Covered rows are `gbrp`, integer/MSB `gbrp*`, and floating `gbrpf16*`/`gbrpf32*`. FFmpeg succeeds for default nonzero-left crop on one-byte `gbrp` by rounding left crop down to preserve 32-byte data-pointer alignment. For multi-byte planar GBR, default nonzero-left crop returns `AVERROR_BUG` without mutation; with `AV_FRAME_CROP_UNALIGNED`, FFmpeg applies exact plane offsets, full-resolution visible dimensions, and resets crop fields on success.

The planar alpha `av_frame_apply_cropping()` slice was checked against pinned FFmpeg 8.1.1 libavutil through the same `frame_oracle.rs` harness. Covered rows are `yuva420p`, `yuva422p`, `yuva444p`, the high-bit `yuva*9/10/12/16*` variants present in FFmpeg 8.1.1, `gbrap`, integer `gbrap*`, and floating `gbrapf16*`/`gbrapf32*`. FFmpeg succeeds for default nonzero-left crop on one-byte YUVA/GBRA rows by rounding left crop down to preserve 32-byte data-pointer alignment. For multi-byte planar alpha formats, default nonzero-left crop returns `AVERROR_BUG` without mutation; with `AV_FRAME_CROP_UNALIGNED`, FFmpeg applies exact luma/chroma/alpha sample offsets and resets crop fields on success.

The bitstream `av_frame_apply_cropping()` slice was checked against pinned FFmpeg 8.1.1 libavutil through the same `frame_oracle.rs` harness. Covered rows are `monow`, `monob`, `rgb4`, `bgr4`, plus the existing `xv30be`/`v30xbe` packed 4:4:4 bitstream rows. FFmpeg crops only right/bottom dimensions in the backing frame, preserves top/left crop fields, and does not advance data pointers for both default and `AV_FRAME_CROP_UNALIGNED`.

The legacy/full-range 8-bit planar YUV `av_frame_apply_cropping()` slice was checked against pinned FFmpeg 8.1.1 libavutil through `crates/avutil/tests/frame_oracle.rs`. The covered rows are `yuvj420p`, `yuvj422p`, `yuv410p`, `yuv411p`, `yuvj411p`, `yuv440p`, `yuvj440p`, and `yuvj444p`. FFmpeg succeeds for default nonzero-left crop on these one-byte-per-sample planar formats by rounding luma left crop down to preserve 32-byte data-pointer alignment. With `AV_FRAME_CROP_UNALIGNED`, FFmpeg applies exact luma offsets, subsampled chroma offsets derived from the format chroma geometry, floor-shifted visible chroma dimensions, and resets crop fields on success.

The packet side-data name boundary slice was checked against pinned FFmpeg 8.1.1 `libavcodec/packet.h` and `av_packet_side_data_name()`. The oracle row records that invalid enum values `INT_MIN` and `-1`, sentinel `AV_PKT_DATA_NB`, `AV_PKT_DATA_NB + 1`, and `INT_MAX` return NULL; the Rust model exposes the same bounded surface through `PacketSideDataKind::from_ffmpeg_value()` and `ffmpeg_side_data_name_for_value()`.

The standalone packet side-data duplicate slice now pins `av_packet_side_data_get()`, `av_packet_side_data_add()`, `av_packet_side_data_remove()`, and `av_packet_side_data_free()` on empty and duplicate-type arrays: empty lookup/remove/free preserve empty state; duplicate lookup returns the first matching entry, add replaces the first matching entry, remove scans from the end, removes the last matching entry, swap-fills with the previous tail, and free resets the duplicate-rich array. The standalone flag rows also prove FFmpeg ignores nonzero flags for `av_packet_side_data_new()` and `av_packet_side_data_add()` while retaining first-match replacement and ownership-transfer behavior.

The packet dictionary destination-mutation rows `packet:dict-unpack-into-*`
pin `av_packet_unpack_dictionary()` when the caller passes a nonempty
destination dictionary. Valid unpack removes matching existing keys before
appending replacements using FFmpeg's swap-removal dictionary order, empty
input is a no-op, malformed first pairs preserve the destination, and malformed
tails keep earlier successfully unpacked pairs while returning
`AVERROR_INVALIDDATA`.

The packet side-data padding rows `packet:side-new-padding` and
`packet:array-new-padding` read the `AV_INPUT_BUFFER_PADDING_SIZE` bytes after
the visible `AV_PKT_DATA_NEW_EXTRADATA` payload allocated by
`av_packet_new_side_data()` and standalone `av_packet_side_data_new()`. The
pinned FFmpeg 8.1.1 oracle reports those trailing bytes as zero, while ordinary
summary and lookup rows still compare only the public visible side-data size.
The companion `packet:side-new-oversize-*` and
`packet:array-new-oversize-*` rows pass `SIZE_MAX` as the requested visible
size and prove both APIs return NULL without mutating the existing side-data
collection.
