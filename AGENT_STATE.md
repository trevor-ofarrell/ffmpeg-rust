# Agent State

## Current Status

Latest `fate-runner` update: changes under `tests/differential/` now select the `fate-runner` component during changed-path analysis, and unit coverage parses the live `tests/differential/mappings.txt` file against the live ledger IDs. The parser check covers the current rawvideo oracle rows plus the `avutil-channel-layout|oracle-ffmpeg-layouts` row without requiring a pinned oracle binary. Validation passed with focused `fate-runner` tests through `target-fate-runner-diff-test`, differential mapping listing and changed dry-run through `target-fate-runner-diff-bin`, a differential channel-layout dry-run with an existing placeholder oracle path, `fate-runner` clippy, formatting, and diff checks. The component remains `scaffolded`, not complete, because upstream FATE sample-based media mappings are still absent.

Correction for the latest `avutil-channel-layout` oracle-harness slice: after adding the `fate-runner` regression test, full `run --changed` execution is blocked in this Windows environment by Application Control on conflicting rebuilt test executables. The direct equivalent checks pass: `cargo test -p fate-runner --target-dir target-codex`, `cargo test -p avutil --test channel_layout_oracle`, and `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`. `run --changed --dry-run` also passes and selects `fate-runner` plus `avutil-channel-layout`.

Latest `avutil-channel-layout` update: added an ignored `ffmpeg -layouts` oracle harness at `crates/avutil/tests/channel_layout_oracle.rs`. The harness runs the pinned FFmpeg oracle with `-hide_banner -layouts`, parses the individual-channel and standard-layout tables, and compares them to `Channel::ALL` and `ChannelLayout::known_layouts()`. It is wired into `tests/differential/mappings.txt` as `avutil-channel-layout|oracle-ffmpeg-layouts`, and `fate-runner` now maps changes to that integration test back to `avutil-channel-layout`. The default ignored test compile, focused channel-layout tests, differential mapping listing, local component FATE, changed-path FATE dry-run/execution, avutil and fate-runner clippy, formatting, and diff checks passed; `git diff --check` reported CRLF warnings only. The component remains `implemented`, not `complete`, because the pinned FFmpeg oracle binary is absent locally, so the new ignored differential has not executed, and byte-preserving non-UTF-8 custom-name parity, upstream FATE parity, and actual fuzz execution remain pending.

Latest `avutil-channel-layout` update: parser-facing numeric-mask, count-suffix, and described-list parsing now follows pinned FFmpeg 8.1.1 `strtoull`/`strtol` leading-whitespace and full-consumption behavior more closely. `ChannelLayoutSpec::parse` accepts leading C whitespace and leading `+` where FFmpeg's C parsers accept them, such as ` 0x3`, ` 2c`, `+2C`, ` +2 channels`, and ` +2 channels (FL+FR)`, while trailing-junk forms such as `0x3 `, `3 `, `2c `, `2C `, `2 channels `, and `2 channels (FL+FR) ` reject with typed invalid-argument errors. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed; `git diff --check` reported CRLF warnings only. The component remains `implemented`, not `complete`, because byte-preserving non-UTF-8 custom-name parity, oracle vectors, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: parser-facing channel/layout string matching now follows pinned FFmpeg 8.1.1 case-sensitive `strcmp` behavior for native channel names, `UNK`/`UNSD`, uppercase `AMBI`/`USR` prefixes, and exact layout names. `ChannelLayoutSpec::parse` no longer falls through to the ergonomic case-insensitive `ChannelLayout::parse` helper, so case-mismatched forms such as `fl+fr`, `unk+unk`, `ambi0`, `usr0`, `STEREO`, `stereo `, and `ambisonic 1+STEREO` reject with typed invalid-argument errors while canonical uppercase channel IDs and exact lowercase layout names such as `stereo` remain supported. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed; `git diff --check` reported CRLF warnings only. The component remains `implemented`, not `complete`, because byte-preserving non-UTF-8 custom-name parity, oracle vectors, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: the explicit `ambisonic <order>[+extra]` parser branch now uses the bounded FFmpeg `strtol`-shaped base-0 parser for the order prefix. This covers signed-zero and no-conversion edge cases from pinned FFmpeg 8.1.1: `ambisonic -0` resolves to order 0, `ambisonic -0+stereo` resolves to order 0 with stereo extras, and `ambisonic +stereo` resolves to zeroth-order ambisonics with stereo extras because the C `strtol` branch consumes no digits before the `+`. Negative nonzero orders still reject. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed; `git diff --check` reported CRLF warnings only. The component remains `implemented`, not `complete`, because byte-preserving non-UTF-8 custom-name parity, oracle vectors, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: `CustomChannelLayout::parse_channel_list_bytes` and `ChannelLayoutSpec::parse_bytes` now provide explicit byte-entry parser coverage for valid UTF-8 byte strings while rejecting non-UTF-8 inputs with typed `InvalidData` errors instead of lossy conversion. NUL-containing byte strings still flow through the existing string parser and return typed `InvalidArgument` errors. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed; `git diff --check` reported CRLF warnings only. The component remains `implemented`, not `complete`, because byte-preserving non-UTF-8 custom-name parity, oracle vectors, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `masked_description` clarified that native-mask reduction accepts raw channel IDs `0..62`, not only modeled named native channel enum entries. `CustomChannelLayout::canonical_native_mask`, `ChannelLayoutSpec::retype_to_native_order`, the bounded target-AMBISONIC extra-mask path, and canonical parsing/retyping now handle `USR45+USR46` as an exact native mask and `AMBI0+AMBI1+AMBI2+AMBI3+USR45` as an explicit ambisonic extra mask, while `USR63`, out-of-range user IDs, duplicates, and descending raw-ID order still reject native/ambisonic retyping. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed; `git diff --check` reported CRLF warnings only. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, oracle vectors, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `av_channel_layout_retype(..., AV_CHANNEL_ORDER_AMBISONIC, flags)` and `AV_CHANNEL_LAYOUT_RETYPE_FLAG_CANONICAL` shaped the next bounded retype slice. `ChannelLayoutSpec::retype_to_ambisonic_order` now converts custom maps with complete standard-order AMBI prefixes and strictly ordered native extras into explicit `AmbisonicChannelLayout` specs, reporting custom-name drops through `ChannelLayoutRetypeResult::is_lossy`; `retype_to_canonical_order` now mirrors the bounded `canonical_order` path by leaving named or unreducible custom maps as custom no-ops while reducing nameless all-UNKNOWN, strict native, and strict ambisonic maps to unspecified, native, or ambisonic specs. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, and formatting passed. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, broader raw-channel/custom/ambisonic retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `av_channel_layout_retype(..., AV_CHANNEL_ORDER_NATIVE/UNSPEC, flags)` shaped the next bounded lossy retype slice. `ChannelLayoutSpec::retype_to_native_order` and `retype_to_unspecified_order` now return `ChannelLayoutRetypeResult` with an explicit lossy flag: named custom maps can drop names when retyped to native only if lossy conversion is allowed, and native/native-mask/ambisonic or concrete custom identities can reduce to count-only unspecified only if lossy conversion is allowed. The existing `to_native_order_lossless` and `to_unspecified_order_lossless` wrappers remain strict. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, target-AMBISONIC/CANONICAL retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `av_channel_layout_retype(..., AV_CHANNEL_ORDER_NATIVE/UNSPEC, LOSSLESS)` shaped the next bounded retype slice. `ChannelLayoutSpec::to_native_order_lossless` now treats native specs as no-ops and retypes nameless strictly ordered native custom maps to `Native` or `NativeMask`; `ChannelLayoutSpec::to_unspecified_order_lossless` now treats count-only unspecified specs as no-ops and retypes nameless all-`UNK` custom maps to count-only `Unspecified`. Named, duplicate, mixed, ambisonic, and other lossy cases return typed invalid-argument errors instead of dropping channel names or identities. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, lossy and broader order retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `av_channel_layout_retype(..., AV_CHANNEL_ORDER_CUSTOM, ...)` shaped the next bounded retype slice. `ChannelLayoutSpec::to_custom_layout` now expands native, arbitrary native-mask, and explicit ambisonic specs through their source-shaped `channel_from_index` order into nameless `CustomChannelLayout` maps, clones existing custom maps without dropping names, and turns count-only unspecified specs into `UNK` custom maps instead of inventing channel identities. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, and formatting passed. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, broader lossy and unspecified/custom/native/ambisonic retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `canonical_order` and `av_channel_layout_retype(..., AV_CHANNEL_ORDER_AMBISONIC, CANONICAL)` can losslessly retype nameless custom maps with a complete standard-order `AMBI0..AMBI<N>` prefix and strictly ordered native extra channels into `AV_CHANNEL_ORDER_AMBISONIC`, while names, unknown extras, incomplete prefixes, or out-of-order native extras remain custom/non-lossless. `CustomChannelLayout::canonical_ambisonic_layout` now exposes that bounded lossless helper, and `ChannelLayoutSpec::parse` retypes nameless lists such as `AMBI0+AMBI1+AMBI2+AMBI3` and `AMBI0+AMBI1+AMBI2+AMBI3+FL+FR` to explicit `AmbisonicChannelLayout` specs while preserving named, unknown-extra, and out-of-order forms as Custom. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, broader lossy and unspecified/custom/native/ambisonic retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed explicit `AV_CHANNEL_ORDER_AMBISONIC` lookup indexes AMBI channels before native extra-mask channels, uses native mask-bit order for extras, and routes string lookup through canonical `av_channel_from_string` before returning the channel at that index. `AmbisonicChannelLayout`, `NativeChannelMaskLayout`, `UnspecifiedChannelLayout`, and `ChannelLayoutSpec` now expose source-shaped `index_from_channel`, `index_from_string`, and `channel_from_string` where applicable, with absent or invalid lookups returning typed invalid-argument errors or `None`. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, and formatting passed. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, broad native/custom/ambisonic/unspecified retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/opt.c` and `libavutil/avstring.c` confirmed the `parse_channel_list` tokenizer is `av_opt_get_key_value`/`av_get_token` shaped: FFmpeg ASCII whitespace is skipped around keys and values, missing keys become implicit channel tokens, single quotes group token text, backslash escapes separators, unescaped `@` after the first key separator stays in the custom name, and a trailing `+` after a valid token is accepted while leading/repeated empty tokens still fail. `CustomChannelLayout::parse_channel_list` and `ChannelLayoutSpec::parse` now cover that bounded Rust/UTF-8 subset, including `FL@Left\+Right+FR`, `FL@Left\@Name+FR`, `FL@Left@Again`, `FL @ Left + FR`, `FL@'Left Right'+FR`, `'FL'+FR`, and `FL+`; focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed. The component remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, broad retyping, full `AV_CHANNEL_ORDER_AMBISONIC` comparison/index-string parity, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: `ChannelLayoutSpec` now has a bounded explicit `AmbisonicChannelLayout` variant for `AV_CHANNEL_ORDER_AMBISONIC`-like order plus native-extra-mask layouts. `ChannelLayoutSpec::parse` returns that variant for `ambisonic <order>` no-extra forms and native-extra forms such as `+stereo` and `+0x5`, preserves named/custom extras such as `+FL@Left+FR@Right` as Custom-backed AMBI0..AMBI<N> maps, and rejects unsupported unspecified or nested ambisonic extras. Focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed. The component remains `implemented`, not `complete`, because full `av_opt_get_key_value` escaping/quoting parity, broad retyping, full `AV_CHANNEL_ORDER_AMBISONIC` comparison/index-string parity, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `parse_channel_list` creates `AV_CHANNEL_ORDER_CUSTOM` maps from `CH[@name]+...` tokens and then canonical retyping can reduce nameless native maps to native masks or all-unknown maps to unspecified order. `ChannelLayoutSpec` now has a first-class `Custom` variant, `ChannelLayoutSpec::parse` wires the bounded custom-list syntax into that variant, and accessors now cover native, native-mask, custom, and unspecified specs without relying on `Copy`. Parsed custom specs preserve named maps such as `FL@Left+FR@Right`, duplicate native-ID maps such as `FL+FL`, and mixed raw-ID maps, while still reducing strictly ordered nameless native maps to `Native`/`NativeMask` and all-UNK nameless maps to `Unspecified`. Focused avutil channel-layout tests, avformat audio tests, fuzz-package check/clippy, avutil/avformat clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and diff checks passed. The component remains `implemented`, not `complete`, because full `av_opt_get_key_value` escaping/quoting parity, explicit `AV_CHANNEL_ORDER_AMBISONIC`, broad retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `parse_channel_list` uses `av_opt_get_key_value` to parse `+`-separated `CH[@name]` entries, resolves channel tokens with `av_channel_from_string`, stores optional `AVChannelCustom.name` values, and relies on canonical retyping afterward. `CustomChannelLayout::parse_channel_list` now parses the bounded custom-list helper syntax directly into Rust custom maps, including native IDs, `UNK`, `UNSD`, `AMBI<n>`, `USR<raw>`, duplicate IDs, 15-byte names, custom descriptions, and typed rejection for empty tokens, `NONE`, unknown IDs, missing IDs, multiple unescaped `@` separators, overlong names, and NUL bytes. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, and diff checks passed. The component remains `implemented`, not `complete`, because this helper is not yet wired into `ChannelLayoutSpec::parse` as `AV_CHANNEL_ORDER_CUSTOM`, full `av_opt_get_key_value` escaping/quoting parity is absent, implicit/broad ambisonic parsing and retyping remain incomplete, and oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `parse_channel_list` builds a channel map from `+`-separated native channel names and canonical retyping can reduce nameless native maps to native masks. `ChannelLayoutSpec::parse` now accepts arbitrary nameless native channel lists for the bounded native-channel subset, so `FL+FR` still resolves to the modeled `stereo` layout while `FL`, `FL+FC`, and `2 channels (FL+FC)` preserve exact native bitmasks through `NativeChannelMaskLayout`. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, and diff checks passed. The component remains `implemented`, not `complete`, because custom `@name` map parsing, implicit/broad ambisonic parsing and retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed the numeric-mask branch initializes `AV_CHANNEL_ORDER_NATIVE` with the exact nonzero mask. `NativeChannelMaskLayout` now preserves arbitrary nonzero native masks that are not modeled named layouts, including `0x5` and high-bit masks, with popcount channel counts, mask-bit-order index lookup, subset intersections, FFmpeg-shaped descriptions such as `2 channels (FL+FC)` and `1 channels (USR63)`, and native/custom equivalence helpers. `ChannelLayoutSpec::parse` still returns named `Native` layouts when a numeric mask maps exactly to the modeled inventory, but otherwise returns `NativeMask` instead of rejecting the mask. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, and diff checks passed. The component remains `implemented`, not `complete`, because full custom-map parsing, implicit/broad ambisonic parsing and retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed the `av_channel_layout_from_string()` numeric-mask branch uses `strtoull(..., base 0)`, requires a nonzero fully consumed value, rejects any input containing `-`, and initializes a native-order layout with the exact mask. The Rust parser now accepts numeric masks only when they map exactly to the modeled native `ChannelLayout` inventory, covering hex/decimal/octal and plus-prefixed forms such as `0x3`, `3`, `03`, and `+0x3`; arbitrary FFmpeg-valid native masks that do not map to a modeled layout, such as `0x5`, remain explicit invalid inputs until native layouts can carry arbitrary masks. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local component FATE, changed-path FATE, and diff checks passed. The component remains `implemented`, not `complete`, because arbitrary native masks, full custom-map parsing, implicit/broad ambisonic parsing and retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed the bounded `av_channel_layout_from_string()` count-suffix behavior used by native defaults and count-only unspecified layouts. `ChannelLayoutSpec::parse` now accepts current native layout names and `FL+FR`-style native expressions, `Nc` only when the modeled default count is native (`2c`, `10c`), `NC`/`N channels` as count-only unspecified layouts (`2C`, `9 channels`), and `N channels (<native channel-list>)` with count validation. Invalid zero counts, `9c`, mismatched or unterminated described lists, empty lists, trailing text, and NUL-containing strings return typed invalid-argument errors. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local component FATE, changed-path FATE, and diff checks passed. The component remains `implemented`, not `complete`, because numeric masks, full custom-map parsing, implicit/broad ambisonic parsing and retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed the `AV_CHANNEL_ORDER_UNSPEC` count-only contract used by `av_channel_layout_default` fallback layouts. The Rust model now has `UnspecifiedChannelLayout` and `ChannelLayoutSpec`: default channel-count paths return modeled native layouts for known source-order defaults or a count-only unspecified layout for other positive counts, unspecified descriptions render as `<n> channels`, same-count unspecified layouts compare equal, unspecified-vs-native/custom layouts compare unequal, unspecified subset masks are zero, and unsupported index lookup returns no channel. `AudioFrame` and `avformat::AudioStreamParameters` now store `ChannelLayoutSpec`, expose `channel_layout_spec()`, and keep legacy `channel_layout()` accessors native-only. Focused unit validation, formatting, fuzz-package build/clippy, avutil/avformat clippy, local changed-path FATE, FATE listing, and diff checks passed. The component remains `implemented`, not `complete`, because full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Previous `avutil-channel-layout` default-count update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `av_channel_layout_default` scans `channel_layout_map` in source order and falls back to `AV_CHANNEL_ORDER_UNSPEC` when no exact count match exists. `ChannelLayout::default_for_count` now uses the modeled source-order layout inventory instead of a short hard-coded table, covering FFmpeg's first modeled native defaults for counts 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, and 24: mono, stereo, 2.1, 4.0, 5.0, 5.1, 6.1, 7.1, 5.1.4, 7.1.4, 9.1.4, 9.1.6, and 22.2. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, FATE listing, and final diff checks passed. That slice remained `implemented`, not `complete`, because count-only fallback threading, full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, and actual fuzz execution were still absent.

Previous `avutil-channel-layout` lookup update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `av_channel_layout_channel_from_index`, `av_channel_layout_index_from_channel`, `av_channel_layout_index_from_string`, and `av_channel_layout_channel_from_string` behavior for native and custom layouts. The Rust model now exposes current native/custom lookup helpers: native channel lookup by index and string follows mask-bit source order, custom string lookup returns the first matching map entry, invalid lookups return typed invalid-argument errors or `None`, and deterministic fuzz fixtures cover the new lookup contracts. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, FATE listing, and final diff checks passed. The component remains `implemented`, not `complete`, because full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Previous `avutil-channel-layout` subset update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `av_channel_layout_subset` returns `layout->u.mask & mask` for native or ambisonic layouts and, for custom maps, scans requested native mask bits and includes a bit when `av_channel_layout_index_from_channel()` finds that raw channel ID in the custom map. The Rust model now exposes `ChannelLayout::subset_mask` and `CustomChannelLayout::subset_native_mask` for the current native/custom subset, including name-insensitive custom presence checks, duplicate collapse, out-of-order custom presence, and non-native ID exclusion. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, FATE listing, and final diff checks passed. The component remains `implemented`, not `complete`, because full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Previous `avutil-channel-layout` equivalence update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `av_channel_layout_compare` rejects different channel counts, treats one unspecified layout as unequal and two unspecified layouts as equal, compares same-order native or ambisonic masks directly, and otherwise compares `av_channel_layout_channel_from_index()` results by position while ignoring custom channel names. The Rust model now exposes current-subset native/custom equivalence helpers on `ChannelLayout` and `CustomChannelLayout`, covering native mask equality, native-vs-custom channel ID equality by index, custom-vs-custom channel ID equality by index, name-insensitive custom equivalence, out-of-order rejection, unknown custom equality, and ambisonic custom ID equality. Focused unit validation passed for this slice so far. The component remains `implemented`, not `complete`, because full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `av_channel_layout_ambisonic_order` for custom maps requires a complete standard-order `AMBI0..AMBI<N>` prefix, rejects ambisonic channels after non-ambisonic channels, rejects ACN/index mismatches and incomplete square orders, and `try_describe_ambisonic` describes valid layouts as `ambisonic <order>` plus optional trailing-channel descriptions. The Rust model now exposes `CustomChannelLayout::ambisonic_order` and uses that path in `describe()` before native/custom fallback, including trailing native-name reduction such as `ambisonic 1+stereo` and named/custom trailing-channel descriptions. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, and final diff checks passed for this slice. The component remains `implemented`, not `complete`, because full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, layout comparison, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.c` confirmed `masked_description` accepts custom maps as native masks only when every remaining entry is a native channel ID below 63 and native IDs appear in strictly increasing order, while `canonical_order` refuses native canonicalization when any custom channel name exists. The Rust model now exposes `CustomChannelLayout::canonical_native_mask`, `canonical_native_layout`, and lossless native-name description reduction for nameless strictly ordered custom maps such as `FL+FR`, while named, duplicate, out-of-order, unknown, unused, user, or ambisonic entries remain custom or return typed errors. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, and final diff checks passed for this slice. The component remains `implemented`, not `complete`, because full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic retyping, `AV_CHANNEL_ORDER_AMBISONIC`, layout comparison, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed `AVChannelCustom` stores `id`, `name[16]`, and `opaque`; `av_channel_layout_custom_init` rejects nonpositive counts and initializes entries to `AV_CHAN_UNKNOWN`; `av_channel_layout_check` rejects custom maps with `AV_CHAN_NONE`; custom lookup returns the first matching raw channel ID and supports `CH@name`/`@name`; and custom descriptions use `<n> channels (CH[@name]+...)` when not reducible to a named/native layout. The Rust model now exposes `ChannelCustom` and `CustomChannelLayout` for positive-length custom maps, bounded names, duplicate channel IDs, first-match lookup, and custom-order descriptions. Focused unit validation, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, and final diff checks passed. The component remains `implemented`, not `complete`, because full `av_channel_layout_from_string()` grammar, native/custom/ambisonic retyping, `AV_CHANNEL_ORDER_AMBISONIC`, layout comparison, oracle inventory parity, upstream FATE parity, and actual fuzz execution remain absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed `AV_CHAN_NONE = -1`, `AV_CHAN_UNUSED = 0x200`, `AV_CHAN_UNKNOWN = 0x300`, ambisonic raw channel IDs from `AV_CHAN_AMBISONIC_BASE = 0x400` through `AV_CHAN_AMBISONIC_END = 0x7ff`, and FFmpeg's raw channel name/description formatting for `NONE`, `UNSD`, `UNK`, `AMBI<n>`, and `USR<raw>`. The Rust model now exposes a separate `ChannelId` helper for native channel raw IDs, those special IDs, ambisonic ACN IDs, and user raw IDs without forcing any of them into native-mask `ChannelLayout` values. Focused unit, fuzz-package build/clippy, avutil clippy, local FATE, changed-path, downstream avformat audio, format, and diff-whitespace checks passed after re-exporting `ChannelId` from `avutil`. The component remains `implemented`, not `complete`, because full custom/native/ambisonic `AVChannelLayout` order semantics, remaining `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed standalone native channel IDs `AV_CHAN_SURROUND_DIRECT_LEFT`, `AV_CHAN_SURROUND_DIRECT_RIGHT`, `AV_CHAN_SIDE_SURROUND_LEFT`, `AV_CHAN_SIDE_SURROUND_RIGHT`, `AV_CHAN_TOP_SURROUND_LEFT`, and `AV_CHAN_TOP_SURROUND_RIGHT`, their public short names `SDL`/`SDR`/`SSL`/`SSR`/`TTL`/`TTR`, and native mask bits `1 << 33`, `1 << 34`, `1 << 41`, `1 << 42`, `1 << 43`, and `1 << 44`. The Rust model now exposes those as source-checked channel IDs with name/from_name/mask tests while deliberately keeping `SDL+SDR`, `SSL+SSR`, and `TTL+TTR` unsupported parser expressions because no currently modeled named layout maps to those standalone pairs. Focused unit, fuzz-build, clippy, local FATE, changed-path, downstream avformat audio, format, and diff-whitespace checks passed. The component remains `implemented`, not `complete`, because full `ffmpeg -layouts` inventory comparison, remaining default-layout count behavior, `AV_CHAN_UNUSED`/`AV_CHAN_UNKNOWN`/ambisonic IDs, custom/native/ambisonic orders, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-channel-layout` update: source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed `AV_CHAN_TOP_CENTER`, `AV_CHAN_BOTTOM_FRONT_CENTER`, `AV_CHAN_BOTTOM_FRONT_LEFT`, `AV_CHAN_BOTTOM_FRONT_RIGHT`, their native mask macros, public short names `TC`/`BFC`/`BFL`/`BFR`, `AV_CH_LAYOUT_22POINT2`, `AV_CHANNEL_LAYOUT_22POINT2`, and the `channel_layout_map` name `22.2`. The Rust model now exposes those four channels and the named 24-channel `22.2` layout with canonical `FL+FR+FC+LFE+BL+BR+FLC+FRC+SL+SR+TFL+TFR+TBL+TBR+TSL+TSR+BC+LFE2+TFC+TC+TBC+BFC+BFL+BFR` ordering. Focused unit, fuzz-build, clippy, local FATE, changed-path, downstream avformat audio, format, and diff-whitespace checks passed. The component remains `implemented`, not `complete`, because full `ffmpeg -layouts` inventory comparison, remaining default-layout count behavior, custom/native/ambisonic orders, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-channel-layout` update: `ChannelLayout` now exposes source-checked FFmpeg 8.1.1 `DL`/downmix-left, `DR`/downmix-right, `BIL`/binaural-left, `BIR`/binaural-right, plus `binaural` and `downmix` from `channel_layout_map`, mapped to `AV_CH_LAYOUT_BINAURAL` and `AV_CH_LAYOUT_STEREO_DOWNMIX`. This builds on prior `TFC`/top-front-center, `WL`/wide-left, `WR`/wide-right, `hexadecagonal`, `TSL`/top-side-left, `TSR`/top-side-right, `9.1.6`, `9.1.4`, `TBC`/top-back-center, `LFE2`/low-frequency-2, `7.2.3`, `5.1.4`, `7.1.2`, `7.1.4`, `TBL`/top-back-left, `TBR`/top-back-right, `octagonal`, `cube`, `FLC`/front-left-of-center, `FRC`/front-right-of-center, `BC`/back-center, `TFL`/top-front-left, `TFR`/top-front-right, `2.1`, `3.0`, `3.0(back)`, `4.0`, `quad(side)`, `3.1`, `5.0`, `5.0(side)`, `4.1`, `6.0`, `6.0(front)`, `hexagonal`, `6.1`, `6.1(back)`, `6.1(front)`, `7.0`, `7.0(front)`, `7.1(wide)`, `7.1(wide-side)`, `5.1.2`, `5.1.2(back)`, and mono/stereo/quad/5.1/5.1(side)/7.1 subset. `ChannelLayout::default_for_count` still follows FFmpeg's first `channel_layout_map` match for modeled 1-, 2-, 3-, 4-, 5-, 6-, 7-, and 8-channel defaults: mono, stereo, 2.1, 4.0, 5.0, 5.1, 6.1, and 7.1. Local unit tests and the shared `avutil_core_models` fuzz harness cover the new channels/layouts, canonical `FL+FR`-style strings, mask/name/channel-expression round trips, default lookups, and unsupported-expression rejection. The component remains `implemented`, not `complete`, because full `ffmpeg -layouts` inventory parity, `22.2` layout inventory, remaining FFmpeg default-layout count behavior, custom/ambisonic orders, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-sample-format` update: `SampleFormat` now exposes `sample_fmt_string_header` and `sample_fmt_string` helpers shaped after pinned FFmpeg 8.1.1 `av_get_sample_fmt_string` output for native sample-format table rows. The helper returns the exact `name   depth` header and fixed-width 12-byte row strings (`%-6s   %2d `) for the currently modeled `AVSampleFormat` inventory, giving future `ffmpeg -sample_fmts`-style Rust inventory output a source-checked formatting primitive. Local unit tests and the shared `avutil_core_models` fuzz harness cover the header, row width, name padding, depth alignment, and trailing-space shape. The component remains `implemented`, not `complete`, because pinned `ffmpeg -sample_fmts`/libavutil differential vectors, upstream FATE parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest `avutil-sample-format` update: `SampleFormat` now exposes `SampleAllocation`, `alloc_samples`, and `alloc_array_and_samples` helpers shaped after pinned FFmpeg 8.1.1 `av_samples_alloc` / `av_samples_alloc_array_and_samples` behavior for Rust-owned buffers. The implementation allocates one contiguous writable `BufferRef`, reuses `SampleArrayLayout` for packed or planar plane ranges, records the originally requested sample count separately from `align=0` effective samples, silence-fills requested samples with the existing source-checked `av_samples_set_silence` byte semantics, and keeps alignment padding plus auto-aligned tail bytes deterministically zeroed instead of exposing FFmpeg's uninitialized `av_malloc` tail storage. Local unit tests and the shared `avutil_core_models` fuzz harness cover allocation shape, packed/planar silence initialization, mutable plane access, invalid input rejection, and deterministic padding/tail behavior. The component remains `implemented`, not `complete`, because pinned `ffmpeg -sample_fmts`/libavutil differential vectors, upstream FATE parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest `avutil-sample-format` update: `SampleFormat` now exposes `SampleArrayLayout`, `SamplePlaneRange`, `fill_arrays_layout`, `split_buffer`, and `split_buffer_mut` helpers shaped after pinned FFmpeg 8.1.1 `av_samples_fill_arrays` behavior for bounded Rust buffers. The implementation reuses the source-checked buffer-size arithmetic, reports one packed plane or line-size-spaced planar channel planes, supports immutable and mutable contiguous-buffer splitting, ignores caller trailing bytes, and rejects short buffers before exposing slices. Local unit tests and the shared `avutil_core_models` fuzz harness cover packed/planar plane offsets, `align=0` layouts, extra/short buffer behavior, mutable split behavior, and invalid input rejection. The component remains `implemented`, not `complete`, because pinned `ffmpeg -sample_fmts`/libavutil differential vectors, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest `avutil-sample-format` update: `SampleFormat` now exposes `SampleCopyRange`, `copy_range`, `copy_samples`, and `copy_samples_within` helpers shaped after pinned FFmpeg 8.1.1 `av_samples_copy` behavior for bounded Rust planes. The implementation computes packed copy spans across all channels and planar copy spans per channel plane, validates every selected source and destination range before mutation, supports zero-length copies, and uses Rust's memmove-like `copy_within` for overlapping in-place copies. Local unit tests and the shared `avutil_core_models` fuzz harness cover packed/planar range math, cross-buffer copy behavior, overlap handling, invalid input rejection, and no-mutation failure paths. The component remains `implemented`, not `complete`, because pinned `ffmpeg -sample_fmts`/libavutil differential vectors, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest `avutil-sample-format` update: `SampleFormat` now exposes `SampleSilenceRange`, `silence_byte`, `silence_range`, and `fill_silence` helpers shaped after pinned FFmpeg 8.1.1 `av_samples_set_silence` behavior for bounded Rust planes. The implementation fills `u8`/`u8p` silence with `0x80`, fills every other current native sample format with `0x00`, computes packed spans across all channels and planar spans per channel plane, allows zero-length fills, and validates plane count plus byte ranges before mutating any plane. Local unit tests and the shared `avutil_core_models` fuzz harness cover silence bytes, packed/planar range math, fill behavior, invalid input rejection, and no-mutation failure paths. The component remains `implemented`, not `complete`, because pinned `ffmpeg -sample_fmts`/libavutil differential vectors, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest `avutil-sample-format` update: `SampleFormat` now exposes `SampleBufferLayout`, `buffer_layout`, and `aligned_plane_sizes` helpers shaped after pinned FFmpeg 8.1.1 `av_samples_get_buffer_size` arithmetic for Rust callers. The implementation reports per-plane line size, total buffer size, plane count, effective sample count, and effective alignment; preserves FFmpeg's `align=0` behavior by padding sample counts up to a 32-sample boundary with byte alignment 1; and rejects zero samples, zero channels, overlarge alignment, and sizes outside FFmpeg's `int` return range with typed errors. Local unit tests and the shared `avutil_core_models` fuzz harness cover packed/planar layouts, explicit alignment, zero-alignment auto-padding, invalid inputs, overflow rejection, and aligned plane-size vectors. The component remains `implemented`, not `complete`, because pinned `ffmpeg -sample_fmts`/libavutil differential vectors, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest `avutil-bitwriter` update: `BitWriter` now has clear/reset support and checked bit-level truncation. `truncate_bits` rejects attempts to grow past the written bit count without mutation, truncates storage to the retained bit length, masks unused bits in a partial tail byte, and allows later writes to resume exactly at the truncated bit position. Local unit tests and the build-checked `avutil_bitreader` fuzz target cover clear/reset, truncate success, tail masking, truncation no-mutation failures, and continued write behavior. The component remains `implemented`, not `complete`, because pinned PutBitContext differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-timebase` update: `add_stable` now models the source-shaped signed `av_add_stable` branch for bounded Rust inputs. Exact negative tick increments subtract from the timestamp, fractional negative increments keep the timestamp unchanged through the same `m < d` branch as pinned FFmpeg 8.1.1 `libavutil/mathematics.c`, and exact-result overflow is reported as a typed Rust error instead of relying on C's bounded/undefined timestamp behavior. Local unit tests and the shared `avutil_core_models` fuzz harness cover signed stable-add inputs against an independent model. The component remains `implemented`, not `complete`, because pinned differential vectors, upstream FATE parity, actual fuzz execution, and out-of-range C behavior calibration are still incomplete.

Latest `avutil-timebase` update: `rescale_delta` now models FFmpeg 8.1.1 `av_rescale_delta`-style stateful timestamp conversion from `libavutil/mathematics.c`. It covers first-call initialization from `AV_NOPTS_VALUE`, zero-duration/simple-round paths, reuse of an in-window `last` sample/frame timestamp, clipping to the source timestamp window, fallback when state is outside the accepted window, positive time-base validation, FFmpeg-int nonnegative duration validation, and no `last` mutation on typed validation or overflow errors. Local unit tests and the shared `avutil_core_models` fuzz harness cover the helper against an independent model. The component remains `implemented`, not `complete`, because pinned differential vectors, upstream FATE parity, actual fuzz execution, and exact out-of-range C behavior calibration are still incomplete.

Latest `avutil-timebase` update: `add_stable` now models FFmpeg 8.1.1 `av_add_stable`-style nonnegative timestamp increments from `libavutil/mathematics.c`. It covers exact tick addition, zero increments, sub-tick no-op behavior, repeated fractional increments without accumulated rounding drift, existing fractional phase preservation, positive time-base validation, and typed rejection for negative increments or malformed time bases. Local unit tests and the shared `avutil_core_models` fuzz harness cover the helper against an independent model. The component remains `implemented`, not `complete`, because pinned differential vectors, upstream FATE parity, actual fuzz execution, `av_rescale_delta`, and negative-increment behavior calibration are still incomplete.

Latest `avutil-timebase` update: `compare_ts` and `compare_mod` now model FFmpeg 8.1.1 `av_compare_ts` and `av_compare_mod` behavior from `libavutil/mathematics.c`. `compare_ts` orders timestamps across positive time bases with exact integer cross-products, while `compare_mod` performs centered modular timestamp comparison for nonzero power-of-two moduli. Local unit tests cover cross-timebase ordering, negative timestamps, invalid time bases, modular wraparound, and invalid moduli; the shared `avutil_core_models` fuzz harness build-checks both helpers against independent models. The component remains `implemented`, not `complete`, because pinned differential vectors, upstream FATE parity, actual fuzz execution, and `av_rescale_delta` are still incomplete.

Latest `avutil-rational` update: `Rational::to_int_float_bits` now models FFmpeg 8.1.1 `av_q2intfloat`-style platform-independent IEEE-754 single-precision bit conversion. It covers FFmpeg's special `0/0` NaN bits, zero numerator, zero denominator, finite signed rationals, negative denominators, and typed rejection for raw `i32::MIN` negation cases where the C implementation would rely on signed-overflow behavior. Local unit tests and the shared `avutil_core_models` fuzz harness cover finite bit vectors, special-value bits, and invalid raw inputs. The component remains `implemented`, not `complete`, because pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still incomplete.

Latest `avutil-rational` update: `Rational::gcd_with_limit` now models FFmpeg 8.1.1 `av_gcd_q` for finite positive-denominator inputs. It preserves the raw non-reduced `gcd(num)/lcm(den)` result shape, uses FFmpeg's strict `lcm < max_den` selection before returning that result, returns the caller default when the limit is not met, and rejects zero/negative denominators or nonpositive limits with typed errors instead of inventing behavior for invalid Rust inputs. Local unit tests and the shared `avutil_core_models` fuzz harness cover direct results, non-reduced raw shape, zero numerators, strict limit/default behavior, and invalid inputs. The component remains `implemented`, not `complete`, because pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still incomplete.

Latest `avutil-rational` update: `Rational` now exposes `av_cmp_q`-style comparison through `av_cmp` and `PartialOrd`, including FFmpeg-shaped handling for raw positive/negative infinity sentinels and `0/0` indeterminate forms. It also adds exact `av_nearer_q`/`av_find_nearest_q_idx`-style nearest-candidate helpers over Rust slices, preserving first-candidate ties and rejecting zero-denominator nearest inputs with typed errors. The slice was source-checked against pinned FFmpeg 8.1.1 `libavutil/rational.h` and `libavutil/rational.c`, and local unit plus fuzz-harness build checks cover sentinel comparison, nearest relation reversal, nearest index selection, empty lists, and malformed zero-denominator candidates. The component remains `implemented`, not `complete`, because pinned FFmpeg differential vectors, upstream FATE parity, broader rational helper coverage, and actual fuzz execution are still incomplete.

Latest `avutil-error` update: `AvErrorCode` now exposes Rust-native `av_error_description`, `av_strerror`, and `av_make_error_string` helpers for the pinned FFmpeg 8.1.1 `AVERROR_LIST` table from `libavutil/error.c`, while preserving the FFmpeg-shaped generic `Error number <n> occurred` fallback for unknown codes. The slice also records the upstream duplicate-code behavior where `AVERROR_INPUT_CHANGED | AVERROR_OUTPUT_CHANGED` resolves to the first matching `Input changed` string, not a distinct Rust-only description. Platform `AVERROR(errno)` string parity remains unclaimed.

Latest `fate-runner` update: mapping rows now support `env:NAME=value` fields that are parsed as child-process environment assignments instead of command arguments. Placeholder resolution applies to those environment values, so `tests/differential/mappings.txt` can validate `--oracle-ffmpeg <path>` and inject it as `FFMPEG_ORACLE` for the ignored rawvideo oracle integration harness. Dry-run and prerequisite-check coverage now prove the differential mapping file resolves all three rawvideo components through the same oracle path; real parity execution is still blocked until a pinned FFmpeg 8.1.1 oracle binary is available locally.

Latest `fate-runner` update: explicit FATE runs now accept repeated `--component <id>` flags for multi-component execution, deduplicate duplicate component IDs while preserving first occurrence order, and still reject mixed `--changed` plus `--component` mode selection. A real dry-run verified that `avformat-rawvideo-demuxer` and `avformat-rawvideo-muxer` can be selected together in one command. This removes the prior repeated-`--component` local runner blocker; upstream FATE media mappings and samples remain absent.

Latest rawvideo oracle update: added an ignored `fftools` integration harness that compares Rust rawvideo file-output bytes against a pinned FFmpeg 8.1.1 oracle using streamcopy rawvideo output. The harness currently covers `rgb24` and `gbrp10msble`, exercises the constrained `ffmpeg-rs -f rawvideo ... -f rawvideo <file>` path through the Rust rawvideo demuxer and muxer, and records the tests in the rawvideo CLI/demuxer/muxer ledger entries. `fate-runner` now maps changes to that harness back to `fftools-ffmpeg-rawvideo-file-output`, `avformat-rawvideo-demuxer`, and `avformat-rawvideo-muxer`, so changed-path FATE dry-runs and local smoke runs no longer treat the test file as unmapped implementation work. Default compilation passes with the tests ignored; explicit `--ignored` execution fails locally because neither `FFMPEG_ORACLE` nor `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)` is present. This is a measurable differential-test slot, not completed parity.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's MSB-aligned planar 10/12-bit YUV444 and GBR formats `yuv444p10msble`, `yuv444p10msbbe`, `yuv444p12msble`, `yuv444p12msbbe`, `gbrp10msble`, `gbrp10msbbe`, `gbrp12msble`, and `gbrp12msbbe`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models these as three full-resolution planes with two stored bytes per component sample, 10 or 12 valid high bits, low bits reserved as zero, 30 or 36 logical bpp, YUV or RGB descriptor class, and big-endian flags on the `be` variants. Rust now exposes the variants, descriptor metadata, frame sizing and plane splitting, `VideoFrame` line sizes, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrp10msble -f null -` execution, and affected fuzz-harness invariants. A descriptor-name comparison against the pinned FFmpeg 8.1.1 descriptor table now leaves only hardware-only descriptor names unmatched. Validation passed with focused avutil, avcodec, avformat, and fftools tests, main and fuzz-package clippy, main and fuzz-package check, changed-path FATE dry-run, and directly affected single-component FATE dry-runs. This remains below `complete` because pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 32-bit integer RGB/RGBA formats `rgb96le`, `rgb96be`, `rgba128le`, and `rgba128be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `rgb96*` as one-plane packed RGB with three 32-bit integer components, no alpha, no chroma subsampling, 96 logical bpp, and twelve stored bytes per pixel; `rgba128*` has the same integer shape with four 32-bit components, alpha metadata, 128 logical bpp, and sixteen stored bytes per pixel. Source checking also confirmed that `Y410`/`Y412`/`Y416` are comment-level conceptual aliases around `xv30`/`xv36`/`xv48`, not real FFmpeg 8.1.1 descriptors. Rust now exposes the variants, descriptor metadata, frame sizing and plane splitting, `VideoFrame` line sizes, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgba128le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused avutil, avformat, avcodec FATE, and fftools target-cache tests, workspace formatting, workspace clippy, fuzz-package check/clippy, changed-path FATE dry-run, and directly affected local FATE component mappings except the broad fftools default-target mapping, which is blocked before execution by Windows Application Control. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed floating RGBA formats `rgbaf16le`, `rgbaf16be`, `rgbaf32le`, and `rgbaf32be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `rgbaf16*` as one-plane packed RGBA half-float with RGB plus alpha plus float descriptor flags, no chroma subsampling, 64 logical bpp, and eight stored bytes per pixel; `rgbaf32*` has the same shape with single-precision components, 128 logical bpp, and sixteen stored bytes per pixel. Rust now exposes the variants, descriptor metadata, frame sizing and plane splitting, `VideoFrame` line sizes, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgbaf32le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, workspace formatting, workspace clippy, fuzz-package check/clippy, changed-path FATE dry-run, directly affected local FATE component mappings including `avutil-frame`, and `git diff --check` with CRLF warnings only. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed floating RGB formats `rgbf16le`, `rgbf16be`, `rgbf32le`, and `rgbf32be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `rgbf16*` as one-plane packed RGB half-float with RGB plus float descriptor flags, no alpha, no chroma subsampling, 48 logical bpp, and six stored bytes per pixel; `rgbf32*` has the same shape with single-precision components, 96 logical bpp, and twelve stored bytes per pixel. Rust now exposes the variants, descriptor metadata, frame sizing and plane splitting, `VideoFrame` line sizes, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgbf32le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, workspace formatting, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` with CRLF warnings only. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar floating GBR formats `gbrpf16le`, `gbrpf16be`, `gbrpf32le`, and `gbrpf32be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `gbrpf16*` as planar GBR half-float with three full-resolution planes, RGB plus planar plus float descriptor flags, no alpha, no chroma subsampling, 48 logical bpp, and two stored bytes per component sample; `gbrpf32*` has the same layout with single-precision components, 96 logical bpp, and four stored bytes per component sample. Rust now exposes the variants, descriptor metadata, frame sizing and plane splitting, `VideoFrame` line sizes, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrpf32le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, workspace formatting, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` with CRLF warnings only. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed UYYVYY411 format `uyyvyy411`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream describes storage as `Cb Y0 Y1 Cr Y2 Y3`, one 6-byte group per 4 pixels, with log2 chroma `(2,0)`, 12 logical bpp, one plane, and three 8-bit components. Rust now exposes `PixelFormat::Uyyvyy411`, descriptor metadata, width-divisible-by-4 frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt uyyvyy411 -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, workspace formatting, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` with CRLF warnings only. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's Bayer CFA formats `bayer_bggr8`, `bayer_rggb8`, `bayer_gbrg8`, `bayer_grbg8`, and the matching 16-bit endian variants. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as one-plane Bayer/RGB-class formats with three exposed components, RGB plus Bayer descriptor flags, no alpha, no chroma subsampling, 8 or 16 logical bpp, and one or two stored bytes per pixel. Rust now exposes the new `PixelFormat` variants and names, Bayer descriptor metadata through `is_bayer()`, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt bayer_bggr8 -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, workspace formatting, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, and directly affected local FATE component mappings. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, Bayer conversion/demosaic behavior, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed X/V YUV 4:4:4 formats `xv30le`, `xv30be`, `xv36le`, `xv36be`, `xv48le`, `xv48be`, `v30xle`, and `v30xbe`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `xv30*` and `v30x*` as one-plane 10-bit formats with three exposed YUV components, no alpha, 30 logical bpp, and four stored bytes per pixel; `xv36*` has 12-bit components, 36 logical bpp, and six stored bytes per pixel; `xv48*` has 16-bit components, 48 logical bpp, and eight stored bytes per pixel. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt xv30le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests through accepted target caches, workspace formatting, workspace clippy, fuzz-package clippy, and diff hygiene. A broad workspace library sweep and local changed-path FATE execution were blocked by Windows Application Control on unrelated or target-cache-specific test executables, with no Rust assertion failures observed. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed floating gray+alpha formats `yaf16le`, `yaf16be`, `yaf32le`, and `yaf32be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `yaf16*` as one-plane packed YA half-float with two 16-bit components, alpha and float metadata, 32 logical bpp, and four stored bytes per pixel; `yaf32*` has the same shape with two 32-bit single-precision components, 64 logical bpp, and eight stored bytes per pixel. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt yaf16le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 8-bit YUV/YUVA 4:4:4 formats `vuya`, `vuyx`, `ayuv`, `uyva`, and `vyu444`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `vuya`, `ayuv`, and `uyva` as one-plane packed four-byte-per-pixel YUVA formats with alpha, `vuyx` as a four-byte-per-pixel VUYX format with three exposed components and no alpha, and `vyu444` as a one-plane three-byte-per-pixel VYU 4:4:4 format. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata including `vuyx` 24 logical bpp with four stored bytes per pixel, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt vuya -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed X2RGB10/X2BGR10 formats `x2rgb10le`, `x2rgb10be`, `x2bgr10le`, and `x2bgr10be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as one-plane packed RGB/BGR 10:10:10 formats with three exposed color components, no alpha, log2 chroma `(0,0)`, 30 logical bpp, four stored bytes per pixel, and an unused two-bit X lane. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, four-byte-per-pixel frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt x2rgb10le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed XYZ12 formats `xyz12le` and `xyz12be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as one-plane packed XYZ 4:4:4 formats with three 12-bit components, no alpha, log2 chroma `(0,0)`, 36 logical bpp, six stored bytes per pixel, and lower four bits of each two-byte component unused. Rust now exposes the new `PixelFormat` variants and names, a distinct XYZ descriptor class, six-byte-per-pixel frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt xyz12le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed AYUV64 formats `ayuv64le` and `ayuv64be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models these as one-plane packed AYUV 4:4:4:4 formats with four 16-bit components, alpha, log2 chroma `(0,0)`, 64 logical bpp, big-endian flag on `ayuv64be`, and eight stored bytes per pixel. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, eight-byte-per-pixel frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt ayuv64le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's high-bit packed YUV 4:2:2 formats `y210le`, `y210be`, `y212le`, `y212be`, `y216le`, and `y216be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models these as one-plane packed 4:2:2 YUV formats with log2 chroma `(1,0)`, 10/12/16-bit component descriptors, 20/24/32 logical average bpp, big-endian flags on the `be` variants, and four stored bytes per pixel. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, even-width validated four-byte-per-pixel frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt y210le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's high-bit semi-planar P-family formats `p010le`, `p010be`, `p012le`, `p012be`, `p016le`, `p016be`, `p210le`, `p210be`, `p212le`, `p212be`, `p216le`, `p216be`, `p410le`, `p410be`, `p412le`, `p412be`, `p416le`, and `p416be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `p010`/`p012`/`p016` as two-plane 4:2:0 formats, `p210`/`p212`/`p216` as two-plane 4:2:2 formats, and `p410`/`p412`/`p416` as two-plane 4:4:4 formats, all with one luma plane, one interleaved chroma plane, two stored bytes per component sample, and 10/12/16-bit component descriptors. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, chroma-validated two-plane frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt p010le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's semi-planar YUV 4:2:2 and 4:4:4 NV family: `nv16`, `nv20le`, `nv20be`, `nv24`, and `nv42`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models `nv16` as 8-bit 4:2:2 with one luma plane plus one interleaved UV plane, `nv20le`/`nv20be` as 10-bit 4:2:2 with two stored bytes per component and endian-specific descriptors, and `nv24`/`nv42` as 8-bit 4:4:4 with interleaved UV or VU chroma. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, even-width validation for the 4:2:2 formats, two-plane frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt nv20le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's high-bit-depth planar YUVA formats `yuva420p9le`, `yuva420p9be`, `yuva422p9le`, `yuva422p9be`, `yuva444p9le`, `yuva444p9be`, `yuva420p10le`, `yuva420p10be`, `yuva422p10le`, `yuva422p10be`, `yuva444p10le`, `yuva444p10be`, `yuva422p12le`, `yuva422p12be`, `yuva444p12le`, `yuva444p12be`, `yuva420p16le`, `yuva420p16be`, `yuva422p16le`, `yuva422p16be`, `yuva444p16le`, and `yuva444p16be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream has no `yuva420p12*` formats, models `yuva420p9*` as 22.5 descriptor bpp, and uses four planar two-byte-per-sample payload planes for these high-bit alpha formats with full-resolution alpha. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, chroma-validated four-plane frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt yuva420p10le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's 8-bit planar YUVA formats `yuva420p`, `yuva422p`, and `yuva444p`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream defines `yuva420p` as 20 bpp with log2 chroma `(1,1)`, `yuva422p` as 24 bpp with `(1,0)`, `yuva444p` as 32 bpp with `(0,0)`, and all three use four 8-bit planes with full-resolution alpha. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, chroma-validated four-plane frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt yuva420p -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused checks, workspace formatting, workspace clippy, fuzz-package clippy, workspace library tests, local changed-path FATE mappings, and diff hygiene. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, high-bit YUVA variants, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's high-bit-depth planar YUV 4:4:0 formats `yuv440p10le`, `yuv440p10be`, `yuv440p12le`, and `yuv440p12be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream defines `yuv440p10*` as 20 bpp with log2 chroma `(0,1)`, `yuv440p12*` as 24 bpp with `(0,1)`, and all four use two stored bytes per component sample. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, height-divisible-by-2 frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt yuv440p10le -f null -` execution, and affected fuzz-harness invariants. Validation passed with focused tests, clippy, workspace library tests, directly affected local FATE mappings, and broad local `fate-runner run --changed` through an accepted target directory. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's 14-bit and 16-bit planar YUV 4:2:0/4:2:2/4:4:4 formats: `yuv420p14le`, `yuv420p14be`, `yuv422p14le`, `yuv422p14be`, `yuv444p14le`, `yuv444p14be`, `yuv420p16le`, `yuv420p16be`, `yuv422p16le`, `yuv422p16be`, `yuv444p16le`, and `yuv444p16be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream defines `yuv420p14*` as 21 bpp with log2 chroma `(1,1)`, `yuv422p14*` as 28 bpp with `(1,0)`, `yuv444p14*` as 42 bpp with `(0,0)`, `yuv420p16*` as 24 bpp with `(1,1)`, `yuv422p16*` as 32 bpp with `(1,0)`, `yuv444p16*` as 48 bpp with `(0,0)`, and all use two stored bytes per component sample. Rust now exposes the new `PixelFormat` variants and names, descriptor metadata, two-byte-per-sample frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt yuv420p14le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's paletted `pal8` format. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h`, `libavutil/pixdesc.c`, `libavutil/imgutils.c`, and `libavcodec/rawdec.c`: upstream marks `pal8` as paletted and alpha-bearing, defines 256 RGB32 palette entries for 1024 palette bytes, includes the palette in full image-buffer sizing, and lets ordinary rawvideo packets carry just the one byte-per-pixel index plane while decoder context supplies palette state. Rust now exposes `PixelFormat::Pal8`, `AVPALETTE_COUNT`, `AVPALETTE_SIZE`, `PixelFormatDescriptor::is_paletted`, one-plane index-payload sizing, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt pal8 -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVFrame.data[1]` palette side-plane allocation/propagation, palette side-data behavior, full `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's deprecated full-range planar YUVJ formats `yuvj420p`, `yuvj422p`, `yuvj411p`, `yuvj440p`, and `yuvj444p`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream marks the `yuvj*` names as deprecated full-scale variants of the corresponding planar YUV formats, with the same component geometry and chroma subsampling as `yuv420p`, `yuv422p`, `yuv411p`, `yuv440p`, and `yuv444p`. Rust now exposes distinct `PixelFormat` variants and FFmpeg names, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt yuvj420p -f null -` execution, and affected fuzz-harness invariants. Full-range color semantics are preserved as naming only until a color-range model exists. This remains below `complete` because full color-range metadata, complete `AVPixFmtDescriptor` parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's 1bpp monochrome bitstream formats `monow` and `monob`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as one-component grayscale bitstream formats with MSB-first pixels, `monow` using 0 white/1 black and `monob` using 0 black/1 white. Rust now exposes the new `PixelFormat` variants, name lookup, descriptor metadata, frame-size and plane-splitting math with `ceil(width / 8)` bytes per row, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt monow -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full per-component `AVPixFmtDescriptor` bitstream flag/component-offset parity, palette and color-range formats, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 4bpp RGB bitstream formats `rgb4` and `bgr4`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as 1:2:1 RGB bitstream formats with two pixels per byte and the first pixel in the high nibble. Rust now exposes the new `PixelFormat` variants, name lookup, descriptor metadata, frame-size and plane-splitting math with `ceil(width / 2)` bytes per row, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgb4 -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full per-component `AVPixFmtDescriptor` bitstream flag/layout parity, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's byte-packed low-bit-depth RGB formats `rgb8`, `bgr8`, `rgb4_byte`, and `bgr4_byte`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as one-byte-per-pixel RGB-class formats with three components, no alpha, no chroma subsampling, and byte-packed 3:3:2 or 1:2:1 channel layouts. Rust now exposes the new `PixelFormat` variants, name lookup, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgb8 -f null -` execution, and affected fuzz-harness invariants. The current scalar descriptor reports the maximum component depth for `rgb4_byte`/`bgr4_byte` because full per-component `AVPixFmtDescriptor` component layout parity is still pending. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's semi-planar YUV 4:2:0 formats `nv12` and `nv21`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as planar YUV formats with one full-resolution luma plane, one interleaved chroma plane, three 8-bit components, horizontal and vertical chroma subsampling, and no alpha. Rust now exposes the new `PixelFormat` variants, name lookup, descriptor metadata, frame-size and plane-splitting math with even-width/even-height validation, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt nv12 -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` component layout parity, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed YUV 4:2:2 formats `yuyv422`, `uyvy422`, and `yvyu422`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as packed YUV formats with one two-byte-per-pixel plane, three 8-bit components, horizontal chroma subsampling, and no vertical chroma subsampling. Rust now exposes the new `PixelFormat` variants, name lookup, descriptor metadata, frame-size and plane-splitting math with even-width validation, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt yuyv422 -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 16-bit RGB/BGR family: `rgb565be`, `rgb565le`, `rgb555be`, `rgb555le`, `bgr565be`, `bgr565le`, `bgr555be`, `bgr555le`, `rgb444le`, `rgb444be`, `bgr444le`, and `bgr444be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream models them as packed RGB-class formats with one two-byte-per-pixel plane. Rust now exposes the new `PixelFormat` variants, name lookup, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgb565le -f null -` execution, and affected fuzz-harness invariants. The current scalar descriptor reports the maximum component depth for mixed-depth 565 formats because the full per-component `AVPixFmtDescriptor` component model is still pending. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed high-bit-depth grayscale formats `gray9le`, `gray9be`, `gray10le`, `gray10be`, `gray12le`, `gray12be`, `gray14le`, and `gray14be`, with `y9le`/`y9be`, `y10le`/`y10be`, `y12le`/`y12be`, and `y14le`/`y14be` aliases. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h`/`pixdesc.c`: upstream models them as one-component grayscale formats with no chroma subsampling, 9/10/12/14 descriptor bits per pixel, two stored bytes per sample, and the big-endian descriptor flag on `be` names. Rust now exposes the new `PixelFormat` variants, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt y10le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar GBRA family: `gbrap`, `gbrap10le`, `gbrap10be`, `gbrap12le`, `gbrap12be`, `gbrap14le`, `gbrap14be`, `gbrap16le`, `gbrap16be`, `gbrap32le`, `gbrap32be`, `gbrapf16le`, `gbrapf16be`, `gbrapf32le`, and `gbrapf32be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream names these formats as planar RGB+alpha, four full-resolution GBRA planes, no chroma subsampling, alpha flag set, float flag only for `gbrapf16*` and `gbrapf32*`, and one-, two-, or four-byte sample storage according to component depth. Rust now exposes the new `PixelFormat` variants, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrapf32le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar 16-bit RGB formats `gbrp16le` and `gbrp16be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream names the formats `gbrp16le`/`gbrp16be`, marks them planar RGB with three 16-bit components, 48 descriptor bits per pixel, no chroma subsampling, no alpha, no float flag, and storage split across three full-resolution GBR planes with two bytes per stored sample. Rust now exposes `PixelFormat::Gbrp16Le` and `PixelFormat::Gbrp16Be`, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrp16le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar 14-bit RGB formats `gbrp14le` and `gbrp14be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream names the formats `gbrp14le`/`gbrp14be`, marks them planar RGB with three 14-bit components, 42 descriptor bits per pixel, no chroma subsampling, no alpha, no float flag, and storage split across three full-resolution GBR planes with two bytes per stored sample. Rust now exposes `PixelFormat::Gbrp14Le` and `PixelFormat::Gbrp14Be`, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrp14le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar 12-bit RGB formats `gbrp12le` and `gbrp12be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream names the formats `gbrp12le`/`gbrp12be`, marks them planar RGB with three 12-bit components, 36 descriptor bits per pixel, no chroma subsampling, no alpha, no float flag, and storage split across three full-resolution GBR planes with two bytes per stored sample. Rust now exposes `PixelFormat::Gbrp12Le` and `PixelFormat::Gbrp12Be`, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrp12le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar 10-bit RGB formats `gbrp10le` and `gbrp10be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream names the formats `gbrp10le`/`gbrp10be`, marks them planar RGB with three 10-bit components, 30 descriptor bits per pixel, no chroma subsampling, no alpha, no float flag, and storage split across three full-resolution GBR planes with two bytes per stored sample. Rust now exposes `PixelFormat::Gbrp10Le` and `PixelFormat::Gbrp10Be`, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrp10le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar 9-bit RGB formats `gbrp9le` and `gbrp9be`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream names the formats `gbrp9le`/`gbrp9be`, marks them planar RGB with three 9-bit components, 27 descriptor bits per pixel, no chroma subsampling, no alpha, no float flag, and storage split across three full-resolution GBR planes with two bytes per stored sample. Rust now exposes `PixelFormat::Gbrp9Le` and `PixelFormat::Gbrp9Be`, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrp9le -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's planar 8-bit RGB format `gbrp`. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: upstream names the format `gbrp`, marks it planar RGB with three 8-bit components, no chroma subsampling, no alpha, no float flag, and storage split across three full-resolution planes in GBR plane order. Rust now exposes `PixelFormat::Gbrp`, descriptor metadata, frame-size and plane-splitting math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gbrp -f null -` execution, and affected fuzz-harness invariants. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed floating grayscale formats `grayf16le`, `grayf16be`, `grayf32le`, and `grayf32be`, with `yf32le` and `yf32be` accepted as FFmpeg aliases for the 32-bit pair. The slice was checked against pinned FFmpeg 8.1.1 `libavutil/pixfmt.h` and `libavutil/pixdesc.c`: 16-bit formats are one-component 2-byte floating gray storage, 32-bit formats are one-component 4-byte floating gray storage, big-endian names carry the descriptor big-endian flag upstream, and all four carry upstream float descriptor semantics. Rust now exposes `PixelFormatDescriptor::is_float`, `PixelFormat::is_float`, bits-per-component/frame-size/plane-splitting metadata, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt grayf32le -f null -` execution, and affected fuzz-harness invariants for these formats while treating byte order as raw storage naming only, not conversion. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 32-bit grayscale formats `gray32le` and `gray32be`, with `y32le` and `y32be` accepted as aliases. Descriptor metadata, 32-bit-per-component reporting, 4-byte-per-pixel frame-size math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gray32le -f null -` execution, and the affected fuzz harnesses now exercise these single-plane grayscale formats while treating byte order as raw storage naming only, not conversion. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 16-bit gray+alpha formats `ya16le` and `ya16be`. Descriptor metadata, 16-bit-per-component reporting, 4-byte-per-pixel frame-size math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt ya16le -f null -` execution, and the affected fuzz harnesses now exercise these single-plane alpha-carrying grayscale formats while treating byte order as raw storage naming only, not conversion. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 64-bit RGBA/BGRA formats `rgba64le`, `rgba64be`, `bgra64le`, and `bgra64be`. Descriptor metadata, 16-bit-per-component reporting, 8-byte-per-pixel frame-size math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgba64le -f null -` execution, and the affected fuzz harnesses now exercise these single-plane alpha-carrying formats while treating byte order as raw storage naming only, not conversion. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed gray+alpha `ya8` format, with `gray8a` and `y400a` accepted as aliases. Descriptor metadata, 2-byte-per-pixel frame-size math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt ya8 -f null -` execution, and the affected fuzz harnesses now exercise this single-plane gray+alpha storage format. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 48-bit RGB/BGR formats `rgb48le`, `rgb48be`, `bgr48le`, and `bgr48be`. Descriptor metadata, 16-bit-per-component reporting, 6-byte-per-pixel frame-size math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgb48le -f null -` execution, and the affected fuzz harnesses now exercise these single-plane packed formats while treating byte order as raw storage naming only, not conversion. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 16-bit grayscale formats `gray16le` and `gray16be`. Descriptor metadata, packed frame-size math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt gray16le -f null -` execution, and the affected fuzz harnesses now exercise these single-plane 2-byte-per-pixel formats while treating byte order as raw storage naming only, not conversion. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes FFmpeg's packed 32-bit no-alpha padding RGB formats `0rgb`, `rgb0`, `0bgr`, and `bgr0`. Descriptor metadata, packed frame-size math, `VideoFrame` line sizing, rawvideo decode/demux/mux packet sizing, constrained `ffmpeg-rs -f rawvideo ... -pix_fmt rgb0 -f null -` execution, and the affected fuzz harnesses now exercise these single-plane 4-byte-per-pixel formats while preserving the distinction between padding bytes and real alpha channels. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes planar 8-bit `yuv440p` with descriptor metadata, height-divisible-by-2 chroma geometry, plane splitting, and rawvideo packet-size validation. `VideoFrame` line sizing, rawvideo decode/demux/mux paths, constrained `ffmpeg-rs -f rawvideo ... -f null -` execution, and the affected fuzz harnesses now exercise `yuv420p`, `yuv422p`, `yuv410p`, `yuv411p`, `yuv440p`, and `yuv444p` through the same descriptor-driven YUV plane model. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes planar 8-bit `yuv410p` with descriptor metadata, width-and-height-divisible-by-4 chroma geometry, plane splitting, and rawvideo packet-size validation. `VideoFrame` line sizing, rawvideo decode/demux/mux paths, constrained `ffmpeg-rs -f rawvideo ... -f null -` execution, and the affected fuzz harnesses now exercise `yuv420p`, `yuv422p`, `yuv410p`, `yuv411p`, and `yuv444p` through the same descriptor-driven YUV plane model. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes planar 8-bit `yuv411p` with descriptor metadata, width-divisible-by-4 chroma geometry, plane splitting, and rawvideo packet-size validation. `VideoFrame` line sizing, rawvideo decode/demux/mux paths, constrained `ffmpeg-rs -f rawvideo ... -f null -` execution, and the affected fuzz harnesses now exercise `yuv420p`, `yuv422p`, `yuv411p`, and `yuv444p` through the same descriptor-driven YUV plane model. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes planar 8-bit `yuv444p` with descriptor metadata, full-resolution chroma plane sizing, plane splitting, and no chroma parity requirement. `VideoFrame` line sizing, rawvideo decode/demux/mux paths, constrained `ffmpeg-rs -f rawvideo ... -f null -` execution, and the affected fuzz harnesses now exercise `yuv420p`, `yuv422p`, and `yuv444p` through the same descriptor-driven YUV plane model. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now includes planar 8-bit `yuv422p` with descriptor metadata, frame-size math, plane splitting, and even-width validation. `VideoFrame` line sizing, rawvideo decode/demux/mux paths, constrained `ffmpeg-rs -f rawvideo ... -f null -` execution, and the affected fuzz harnesses now use the shared chroma-subsampling metadata for both `yuv420p` and `yuv422p`. This remains below `complete` because full `AVPixFmtDescriptor` coverage, full `ffmpeg -pix_fmts` inventory parity, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` update: the shared pixel format model now exposes descriptor-style metadata for the current `gray`/`gray8`, `rgb24`, `bgr24`, `rgba`, `bgra`, `argb`, `abgr`, and `yuv420p` subset. `PixelFormatDescriptor` records FFmpeg-style name, class, component count, bits per component, average bits per pixel, plane count, packed/planar status, alpha status, packed bytes per pixel, and log2 chroma subsampling; `PixelFormat` exposes convenience helpers for those fields. Unit tests and the build-checked `avutil_core_models` fuzz target cover descriptor/name/class/component/bit-depth/chroma invariants alongside existing frame-size and plane-splitting checks. This remains below `complete` because full `AVPixFmtDescriptor` parity, the full `ffmpeg -pix_fmts` inventory, pinned oracle differential vectors, upstream FATE parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest `avutil-pixel-format` / rawvideo update: the shared pixel format model now covers `bgr24`, `bgra`, `argb`, and `abgr` in addition to the existing `gray`/`gray8`, `rgb24`, `rgba`, and `yuv420p` subset. `PixelFormat::ALL`, packed/planar metadata, alpha metadata, and packed bytes-per-pixel metadata are wired through video frame line sizing, the rawvideo decoder, rawvideo demuxer/muxer frame sizing, relevant fuzz harnesses, and constrained `ffmpeg-rs` rawvideo input parsing. A new CLI test covers uppercase `BGRA` rawvideo input to `-f null -`. Local FATE-runner mappings now cover rawvideo demuxer/muxer plus the shared basic muxer components selected by `avformat_basic_muxers` changes. This remains below `complete` because full `AVPixFmtDescriptor` coverage, many FFmpeg pixel formats, pinned `ffmpeg -pix_fmts` differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-channel-layout` update: the current channel model now exposes FFmpeg-native mask bits for modeled common channels, canonical layout masks, canonical `FL+FR`-style channel strings, and a narrow `ChannelLayout::parse` path for current named layouts or `+`-separated channel expressions that resolve exactly to one modeled layout. Empty, NUL-containing, unknown-channel, duplicate-channel, and unsupported custom expressions return typed invalid-argument errors. Focused unit tests and the build-checked `avutil_core_models` fuzz target cover name/mask/expression round trips and invalid expression rejection. This remains below `complete` because full `AVChannelLayout` order/native/custom/ambisonic semantics, all `ffmpeg -layouts` entries, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-dict` update: dictionaries now support FFmpeg-shaped escaped pair serialization and parsing. `Dictionary::to_pairs_string` renders insertion-ordered key/value pairs with backslash escaping for separators and literal backslashes, while `parse_pairs` and `parse_pairs_into` accept configurable key/value and pair separator sets, apply the caller-selected match/set mode, reject invalid separator sets and dangling escapes, and preserve successfully parsed entries when a later token is malformed. Focused unit tests and the build-checked `avutil_metadata_options` fuzz target cover round trips with duplicate keys, escaped separators, mode application, partial-success parse failures, separator validation, and malformed token rejection. This remains below `complete` because pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-options` update: child option namespaces are now mutable through explicit parent helpers. `OptionChild` exposes mutable option-set access, and `OptionSet` can get child values, query child ranges, set typed child values, and parse string child values while preserving state on missing-child, missing-option, read-only, type, and range failures. Focused unit tests and the build-checked `avutil_metadata_options` fuzz target cover child mutation, unit-constant parsing inside children, root/child namespace independence, and failed-mutation preservation. This remains below `complete` because full AVOption API parity, CLI option-ordering integration, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-bitreader`/`avutil-bitwriter` update: `BitReader` now supports single-bit peeks, checked absolute bit positioning, checked relative bit seeking, and rewind for bounded MSB-first bitstreams, preserving the cursor on invalid seeks. `BitWriter` now supports aligned byte-slice appends that validate byte alignment and bit-count overflow before mutating state. Local unit tests and the build-checked `avutil_bitreader` fuzz target cover seek/cursor invariants and aligned-byte no-mutation invariants. These components remain below `complete` because pinned FFmpeg GetBitContext/PutBitContext differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-byteio` update: `ByteReader` now supports checked absolute positioning, relative seek, and rewind for bounded in-memory byte streams, preserving the cursor on invalid seeks. `ByteWriter` now exposes append-position, clear, checked truncate, raw patching, and endian-aware signed/unsigned patch helpers for existing bytes; invalid bounds and constrained-width failures preserve the existing buffer. Local unit tests and the build-checked `avutil_byteio` fuzz target cover seek/cursor invariants and patch/truncate mutation invariants. This remains below `complete` because pinned FFmpeg AVIO/GetByte differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-hash` / hash-muxer update: `avutil` now has a Rust-native SHA-1 implementation with one-shot and streaming APIs, standard known-vector tests, and fuzz-harness streaming equivalence coverage. `avformat::HashAlgorithm` now exposes SHA-160/SHA-1 as `SHA160` for hash, framehash, and streamhash muxers, and `ffmpeg-rs -f hash -hash sha-160 -` accepts the normalized CLI spelling without invoking FFmpeg at runtime. Local FATE-runner mappings now include the avformat null/hash/framecrc/framehash/streamhash unit filters so packet-muxer and shared fuzz-target changes are measurable by `run --changed`. This remains below `complete` because the SHA-160 behavior has no pinned FFmpeg 8.1.1 oracle differential vectors, no upstream FATE media coverage, and no actual local fuzz execution.

Latest `fftools-option-parser`/logging integration update: process-level CLI diagnostic formatting now routes through `avutil::Logger` instead of formatting a single `LogRecord` directly. This preserves existing quiet, level, time/datetime, and color behavior while adding deterministic repeat-summary compression for consecutive identical diagnostics when the parsed `repeat` flag enables `LogFlags::SKIP_REPEATED`; without the flag, repeated diagnostics remain separate lines. This remains below `complete` because byte-identical upstream repeat behavior, media progress logs, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `fftools-option-parser`/logging integration update: process-level `ffmpeg-rs` and `ffprobe-rs` error formatting now has deterministic terminal-color coverage through the same `LogFormatOptions::with_ffmpeg_env_color_vars_and_stderr` path used by runtime `with_ffmpeg_env_color()`. Tests now prove terminal stderr enables ANSI error coloring when no force env vars are present, non-terminal stderr stays plain, and `AV_LOG_FORCE_NOCOLOR` still wins over terminal detection. This remains below `complete` because byte-identical upstream color policy/formatting, media progress logs, repeat-summary stderr, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-logging` update: `LogColorMode::from_ffmpeg_env` now resolves color using FFmpeg's forced color environment variables plus terminal stderr detection. `AV_LOG_FORCE_NOCOLOR` still wins over `AV_LOG_FORCE_COLOR`, `AV_LOG_FORCE_COLOR` forces ANSI severity coloring, and otherwise stderr terminals enable color while redirected/non-terminal stderr stays uncolored. Focused unit tests and the build-checked `avutil_core_models` fuzz target cover the terminal and forced-env resolver invariants. This remains below `complete` because byte-identical upstream color policy/formatting, full C ABI `av_log_set_callback` semantics, local-time formatting parity, CLI repeat/media-progress stderr parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `fftools-option-parser`/logging integration update: process-level `ffmpeg-rs` and `ffprobe-rs` errors now apply the shared `LogFormatOptions` color resolver. `AV_LOG_FORCE_COLOR` enables ANSI severity coloring for entrypoint error lines, `AV_LOG_FORCE_NOCOLOR` takes precedence and keeps them uncolored, and deterministic tests inject env presence without mutating process environment. This remains below `complete` because the color formatting/policy is not byte-identical to upstream, media progress logs and repeat-summary stderr are not implemented, pinned FFmpeg differential vectors are absent, upstream FATE parity is absent, and actual fuzz execution is still blocked by the missing cargo-fuzz subcommand.

Latest `avutil-logging` update: `LogColorMode` now resolves FFmpeg's forced color environment variables, treating `AV_LOG_FORCE_NOCOLOR` as taking precedence over `AV_LOG_FORCE_COLOR`, and `LogFormatOptions` can apply that resolver without changing its other formatting flags. Focused unit tests cover forced-color, default no-color, no-color precedence, and formatted output under the resolved mode; the build-checked `avutil_core_models` fuzz target mirrors those deterministic env-resolution invariants. This remains below `complete` because byte-identical upstream color formatting/policy, full C ABI `av_log_set_callback` semantics, local-time formatting parity, CLI stderr/repeat/time/datetime parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `fftools-option-parser`/logging integration update: process-level `ffmpeg-rs` and `ffprobe-rs` errors now attach current `LogTimestamp` values when `-loglevel`/`-v` enables `time` or `datetime`, reusing the same `avutil::LogRecord` formatter as the existing quiet and level-prefix path. Deterministic unit tests inject a fixed timestamp and cover `time+error`, `time+datetime+level+error` with datetime precedence, and the clock-unavailable fallback that keeps the previous non-timestamp shape. This remains below `complete` because timestamp formatting is not byte-identical to upstream, local-time parity is unresolved, media progress logs and repeat-summary stderr are not implemented, pinned FFmpeg differential vectors are absent, upstream FATE parity is absent, and actual fuzz execution is still blocked by the missing cargo-fuzz subcommand.

Latest `fftools-option-parser`/logging integration update: process-level `ffmpeg-rs` and `ffprobe-rs` errors now route through a shared private CLI logging helper instead of ad hoc `eprintln!` formatting. The helper performs a best-effort scan for `-loglevel`/`-v`, applies the existing `CliLogConfig` and `avutil::LogRecord` formatting, preserves the default `tool: error` shape, suppresses entrypoint errors at `quiet`/`-8`, and emits `[error] tool: ...` when the `level` flag is active. Local tests cover default formatting, quiet suppression including numeric quiet, level prefixes, last-loglevel-wins behavior, and malformed loglevel fallback. `fate-runner` changed-path selection now maps `crates/fftools/src/cli_logging.rs` to `fftools-option-parser`, and the local mapping runs both the option parser and CLI logging filters. This remains below `complete` because it is not byte-identical upstream stderr, does not cover media progress logging, timestamp/repeat stderr behavior, terminal/env color auto-detection, pinned FFmpeg differential vectors, upstream FATE parity, or actual fuzz execution.

Latest `avutil-logging` update: logging formatting now has explicit `LogFormatOptions` and `LogColorMode` support. The default path remains uncolored, while opt-in `Always` color mode wraps warning records in ANSI yellow and error/fatal/panic records in ANSI red, including the `Logger` and global formatted-record helpers. Repetition summaries and info/verbose/debug/trace records stay uncolored. Focused unit tests cover record, logger, repeat-summary, and global formatted-record color behavior; the build-checked `avutil_core_models` fuzz target mirrors deterministic color formatting. This remains below `complete` because terminal/env color auto-detection, byte-identical upstream color formatting, full C ABI `av_log_set_callback` semantics, local-time formatting parity, CLI stderr/repeat/time/datetime parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-logging` update: `LogTimestamp` now converts `SystemTime` values to the existing Unix-microsecond timestamp domain, including pre-epoch floor-to-microsecond behavior, and exposes `now_utc()` for current system-clock capture. `LogRecord::with_current_timestamp()` attaches that current timestamp for callers that need real-time records. Focused unit tests cover exact post-epoch conversion, pre-epoch conversion, sub-microsecond rounding, current timestamp bounds, and record timestamp attachment; the build-checked `avutil_core_models` fuzz target mirrors deterministic representable `SystemTime` conversion plus record timestamp attachment. This remains below `complete` because full C ABI `av_log_set_callback` semantics, byte-identical `av_log` formatting, local-time formatting parity, color handling, CLI stderr/repeat/time/datetime parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-logging` update: the logging module now exposes a mutex-backed process-global `Logger` primitive with shared level, flag, callback, log, flush, formatted-record, clear, and take-record helpers. The global path reuses the existing filtering, callback, and repeat-summary behavior, and serialized unit tests plus the build-checked `avutil_core_models` fuzz target cover shared level/flag state, filtered records, repeated-summary flushing, callback delivery, callback clearing, and record draining. This remains below `complete` because full C ABI `av_log_set_callback` semantics, byte-identical `av_log` formatting, local-time formatting parity, color handling, CLI stderr/repeat/time/datetime parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-logging` update: `Logger` now supports an installed per-instance callback hook in addition to the existing one-shot callback helper. Installed callbacks receive every accepted emitted record and any materialized repeated-message summary, while filtered records and suppressed repeated duplicates do not dispatch. `set_callback`, `clear_callback`, and `has_callback` expose the lifecycle, and focused unit tests plus the build-checked `avutil_core_models` fuzz target cover accepted-record delivery, repeat-summary dispatch, and callback clearing. This remains below `complete` because byte-identical `av_log` formatting, process-global `av_log_set_callback` parity, local-time formatting parity, color handling, CLI stderr/repeat/time/datetime parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-logging` update: `LogTimestamp` now carries Unix-microsecond timestamps for deterministic UTC `PRINT_TIME` and `PRINT_DATETIME` rendering on timestamped `LogRecord` values. `PRINT_DATETIME` takes precedence when both timestamp flags are selected, records without timestamps keep the previous deterministic format, and repeated-record comparison includes timestamp equality whenever time or datetime prefixes are printed. Focused unit tests and the build-checked `avutil_core_models` fuzz target cover UTC time/datetime formatting, pre-epoch microsecond handling, timestamped record formatting, absent-timestamp behavior, and timestamp-aware repeat suppression. This remains below `complete` because byte-identical `av_log` formatting, global callback installation, local-time formatting parity, color handling, CLI stderr/repeat/time/datetime parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-logging` update: `Logger` now implements deterministic `AV_LOG_SKIP_REPEATED`-style compression for consecutive identical accepted records. With `LogFlags::SKIP_REPEATED`, duplicates are suppressed, `formatted_records()` exposes a pending `Last message repeated n times` summary without mutating the buffer, `flush_repeated()` and `take_records()` materialize the summary, `clear()` drops pending repeated state, disabling `SKIP_REPEATED` flushes pending state, and per-call callbacks run only for emitted records. Focused unit tests and the build-checked `avutil_core_models` fuzz target cover repeat suppression, pending summary formatting, explicit flushing, non-skip retention, flag-change flushing, clear behavior, and callback dispatch. This remains below `complete` because byte-identical `av_log` formatting, global callback installation, time/datetime rendering, color handling, CLI stderr/repeat-flag parity, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-options` update: `OptionValue` and `OptionKind` now include rational values backed by the shared `Rational` type. Rational options parse `num/den` and integer text through `set_from_str`, reject malformed or zero-denominator inputs, validate typed/default values against positive-denominator min/max bounds, and expose rational bounds through `OptionRange`. Focused unit tests cover valid rational parsing, integer-form rational parsing, invalid kind/default/range metadata, typed mismatches, out-of-range values, zero denominators, and malformed fractions; `avutil_metadata_options` now build-checks generated rational option definitions, typed rational values, rational string values, range invariants, and failed-mutation preservation. This remains below `complete` because full AVOption API parity, CLI option ordering integration, pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-dict` update: `Dictionary` now exposes ordered exact-match and prefix-match iterators so duplicate metadata keys and `AV_DICT_IGNORE_SUFFIX`-style scans can be traversed without flattening to only the first match. Focused unit tests cover duplicate-key iteration order, case-sensitive versus case-insensitive matching, prefix iteration order, and empty-prefix all-entry scans; `avutil_metadata_options` now build-checks first-match consistency against those iterators and validates generated exact/prefix matches. This remains below `complete` because pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-byteio` update: `ByteReader` now exposes non-advancing `peek_exact` plus endian-aware unsigned and signed 8/16/24/32/48/64-bit peek helpers that share the same checked EOF/overflow path as advancing reads. Focused unit tests cover unsigned lookahead, signed sign-extension lookahead, no-advance success behavior, EOF no-advance behavior, and checked length-overflow errors; `avutil_byteio` now build-checks those peek operations and asserts peek cursor preservation under fuzz-generated operation sequences. This remains below `complete` because pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-timebase` update: `AV_TIME_BASE`, `AV_TIME_BASE_Q`, direct `rescale`, `rescale_rnd`, and `rescale_rnd_pass_minmax` helpers now cover `av_rescale`-style integer-term timestamp math through the same checked rounding core used by `rescale_q` and `rescale_q_rnd`. Direct and rational paths validate invalid multiplier/divisor/time-base shapes as typed errors, preserve `i64::MIN`/`i64::MAX` sentinels through pass-min/max helpers, and reject out-of-range results. Focused unit tests cover constants, direct integer-term rounding modes, direct pass-min/max behavior, invalid terms, overflow, common media timebase conversion, rational rounding modes, rational pass-min/max, and malformed time bases; `avutil_core_models` build-checks constants, direct rescale terms, invalid direct terms, sentinel preservation, and rational rescale against an independent i128 model. This remains below `complete` because pinned FFmpeg edge-case differential vectors, upstream FATE parity, and actual fuzz execution are still absent.

Latest `avutil-rational` update: `Rational::reduce_i64` now models max-bounded `av_reduce`-style reduction over signed i64 numerators/denominators, including exactness reporting, sign normalization, zero and infinity raw-rational sentinels, invalid-limit rejection, and continued-fraction approximation when either output side exceeds the caller max. `Rational::from_f64_limited` adds an `av_d2q`-style conversion path with finite max-bounded output, NaN as `0/0`, and infinity or overlarge values as `+/-1/0`; `Rational::to_f64` provides the matching `av_q2d`-style conversion. Focused unit tests cover exact reduction, approximation under tight limits, invalid limits, finite f64 conversion, bounded approximation, and NaN/infinity sentinels; `avutil_core_models` build-checks reduction bounds, exactness cross-products, and f64 sentinel invariants. This remains below `complete` because pinned FFmpeg differential vectors, upstream FATE parity, broader rational helper coverage, and actual fuzz execution are still absent.

Latest `avutil-error` update: `AvErrorCode` now models the documented FFmpeg tag-based `libavutil/error.h` constants through an exact `FFERRTAG` helper plus raw-code preservation. `AvError` carries optional code metadata, preserves caller-supplied custom codes, and attaches unambiguous FFmpeg codes for invalid-data, EOF, external, and bug constructors while leaving platform errno-derived invalid-argument/not-found/unsupported cases code-less until a pinned oracle or platform profile defines exact `AVERROR(errno)` behavior. Focused unit tests cover `AV_ERROR_MAX_STRING_SIZE`, tag constants, raw-code round trips, custom-code preservation, constructor code metadata, IO-code mapping, and EOF predicates; `avutil_core_models` build-checks the same code invariants. This remains below `complete` because pinned `av_strerror`/`AVERROR(errno)` differential coverage and upstream FATE parity are still absent.

Latest `avutil-packet` update: `PacketOpaque` now models the raw `AVPacket.opaque` field as nullable, non-dereferenceable address metadata. Null addresses map to `None`; nonzero values copy through `ref_from` and `copy_props_from`, transfer through `move_ref_from`, reset through `unref`, and have explicit set/take/clear helpers without Rust-side pointer ownership or dereference. Focused packet unit tests cover zero-address rejection, nullable clearing, copy-props payload preservation, ref/move/unref propagation, and source/destination independence; `avutil_core_models` build-checks the same packet raw-opaque invariants alongside `opaque_ref` ownership. This remains below `complete` because full AVPacket ABI parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Latest `avutil-packet` update: packet side-data now parses the three IAMF parameter side-data payloads, `AV_PKT_DATA_IAMF_MIX_GAIN_PARAM`, `AV_PKT_DATA_IAMF_DEMIXING_INFO_PARAM`, and `AV_PKT_DATA_IAMF_RECON_GAIN_INFO_PARAM`, in addition to the existing encryption-info, encryption-init-info, strings-metadata, metadata-update, new-extradata, H.263 macroblock-info, palette, Dynamic HDR10+, Dolby Vision config, EXIF, 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketIamfParamDefinition` preserves FFmpeg's native `AVIAMFParamDefinition` envelope and trailing bytes while validating expected definition type, native subblock offset/size/count bounds, nonzero `parameter_rate`, and nonzero subblock durations; the typed mix-gain, demixing, and recon-gain views expose animation/rational fields, `dmixp_mode`, and the 6x12 recon-gain table. Focused tests cover valid constructors/deferred accessors, raw byte preservation, field access, trailing-byte preservation, non-matching kind behavior, wrong definition type, zero parameter rate, invalid offsets/sizes, truncation, zero subblock durations, invalid mix-gain animation values, and deferred accessor errors; `avutil_core_models` now build-checks the same IAMF parser invariants. This remains below `complete` because full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_ENCRYPTION_INFO` and `AV_PKT_DATA_ENCRYPTION_INIT_INFO` payloads in addition to the existing strings-metadata, metadata-update, new-extradata, H.263 macroblock-info, palette, Dynamic HDR10+, Dolby Vision config, EXIF, 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketEncryptionInfo` preserves FFmpeg's big-endian encryption scheme, crypt/skip byte blocks, key ID, IV, clear/protected subsample records, parsed length, and trailing bytes; `PacketEncryptionInitInfo` preserves the counted big-endian init-info list with system IDs, fixed-size key IDs, init data, parsed length, and trailing bytes. The parsers reject truncated or overflowing lengths and reject nonzero key counts with zero key-id size as an unsafe invalid input shape through raw parsing, typed construction, and deferred side-data accessor paths. Focused unit tests cover constructor/deferred accessor behavior, raw byte preservation, field access, trailing-byte preservation, empty init-info lists, non-matching kind behavior, truncated payloads, oversized declared lengths, unsafe zero-sized keyed init-info records, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_STRINGS_METADATA` and `AV_PKT_DATA_METADATA_UPDATE` payloads in addition to the existing new-extradata, H.263 macroblock-info, palette, Dynamic HDR10+, Dolby Vision config, EXIF, 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketStringMetadata` preserves FFmpeg's alternating zero-terminated `key\0value\0` byte list, accepts empty lists and empty values, exposes raw key/value bytes plus optional UTF-8 views, and rejects missing final terminators, empty keys, missing values, and trailing empty keys through raw parsing, typed construction, and deferred side-data accessor paths. Focused unit tests cover FFmpeg constant names, strings-metadata and metadata-update constructors/accessors, byte preservation, empty values, non-UTF-8 value preservation, empty-list behavior, non-matching kind behavior, malformed terminators/keys/values, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_NEW_EXTRADATA` and `AV_PKT_DATA_H263_MB_INFO` payloads in addition to the existing palette, Dynamic HDR10+, Dolby Vision config, EXIF, 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketNewExtradata` preserves the embedded replacement extradata byte buffer including empty buffers, while `PacketH263MbInfo` parses the documented 12-byte little-endian H.263 macroblock-info records and rejects non-record-multiple lengths through raw parsing, typed construction, and deferred side-data accessor paths. Focused unit tests cover FFmpeg constant names, constructor/deferred accessor behavior, byte preservation, empty raw extradata, H.263 entry-count and iterator behavior, every H.263 record field, non-matching kind behavior, malformed H.263 lengths, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_PALETTE` payload in addition to the existing Dynamic HDR10+, Dolby Vision config, EXIF, 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketPalette` models the fixed FFmpeg palette side-data payload as 1024 bytes, preserving 256 four-byte entries while exposing raw entry bytes and native `u32` entry views; malformed lengths are rejected through raw parsing, typed construction, and deferred side-data accessor paths. Focused unit tests cover FFmpeg constant name, constructor/deferred accessor behavior, byte preservation, first and last entry lookup, native-entry conversion, non-matching kind behavior, short/long/empty payload rejection, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_DYNAMIC_HDR10_PLUS` payload in addition to the existing Dolby Vision config, EXIF, 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketDynamicHdr10Plus` models the pinned FFmpeg 8.1.1 native `AVDynamicHDRPlus` packet payload by wrapping the existing frame-side HDR10+ parser, preserving raw bytes and exposing country/application/window, color-transform, and peak-luminance accessors while rejecting malformed lengths, invalid ITU-T T.35 country/application/window values, invalid overlap options, invalid nested counters/flags, and invalid peak-luminance grid flags/counts through raw, constructor, and deferred side-data paths. Focused unit tests cover FFmpeg constant name, constructor/deferred accessor behavior, byte preservation, frame-wrapper access, non-matching kind behavior, malformed lengths, invalid header/window values, invalid nested overlap option, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_DOVI_CONF` payload in addition to the existing EXIF, 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketDolbyVisionConf` models the pinned FFmpeg 8.1.1 nine-byte `AVDOVIDecoderConfigurationRecord`, preserving version, profile, level, base-layer compatibility, and compression fields while rejecting malformed lengths, non-boolean flag bytes, and unknown `AVDOVICompression` raw values through raw and deferred side-data paths. Focused unit tests cover FFmpeg constant names, compression enum values, constructor/deferred accessor behavior, byte round trips, non-matching kind behavior, malformed lengths, invalid flag bytes, invalid compression bytes, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_EXIF` payload in addition to the existing 3D reference displays, ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketExif` reuses the frame-side EXIF/TIFF parser for packet side data, preserving original bytes while validating little-endian and big-endian TIFF headers, first IFD offsets, IFD tables, TIFF entry types, linked IFD limits, and out-of-line value ranges through raw and deferred side-data paths. Focused unit tests cover FFmpeg constant name, constructor/deferred accessor behavior, endian detection, IFD/entry fields, ASCII entry access, byte round trips, non-matching kind behavior, malformed headers, invalid first offsets, truncated tables, invalid TIFF types, out-of-range values, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_3D_REFERENCE_DISPLAYS` payload in addition to the existing ambient viewing environment, display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketThreeDReferenceDisplays` models the pinned native `AV3DReferenceDisplaysInfo` packet side-data shape by wrapping the existing frame-side native allocation envelope, preserving precision fields, display counts, native offsets, entry sizes, view IDs, reference width/distance exponent and mantissa fields, additional-shift flags, and sample shifts while rejecting malformed header lengths, precisions, flags, counts, offsets, entry sizes, truncated/overlong payloads, empty display arrays, oversized display arrays, and invalid display-entry flags through raw and deferred side-data paths. Focused unit tests cover native header length, entry-size constants, FFmpeg constant name, constructor/deferred accessor behavior, indexed display lookup, byte round trips, non-matching kind behavior, malformed lengths, invalid precisions/flags/counts/offsets/sizes/display flags, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_AMBIENT_VIEWING_ENVIRONMENT` payload in addition to the existing display-matrix, stereo3d, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketAmbientViewingEnvironment` models the pinned native `AVAmbientViewingEnvironment` packet side-data shape by wrapping the existing frame-side native envelope, preserving ambient illuminance and CIE 1931 x/y ambient-light chromaticity rationals while rejecting malformed lengths, zero denominators, negative illuminance, and out-of-range normalized chromaticity coordinates through raw and deferred side-data paths. Focused unit tests cover native byte layout, FFmpeg constant name, rational offsets, side-data constructor/deferred accessor behavior, non-matching kind behavior, default zero values, malformed short/long payloads, invalid denominator/sign/range values, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_STEREO3D` payload in addition to the existing display-matrix, replaygain, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketStereo3d` models the pinned native `AVStereo3D` packet side-data shape by wrapping the existing frame-side native envelope, preserving stereo type, invert flags, view, primary eye, baseline, and rational disparity/field-of-view metadata while rejecting malformed lengths and invalid enum/flag/rational values through raw and deferred side-data paths. Focused unit tests cover native byte layout, FFmpeg constant names, unset rational sentinels, side-data constructor/deferred accessor behavior, non-matching kind behavior, malformed short/long payloads, invalid type/flag/view/primary-eye values, invalid rational values, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_REPLAYGAIN` payload in addition to the existing display-matrix, audio-service-type, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketReplayGain` models the pinned native `AVReplayGain` packet side-data shape as a 16-byte native-endian payload, preserves track and album gain/peak fields including FFmpeg's unknown sentinels, and rejects malformed lengths through raw and deferred side-data paths. Focused unit tests cover native byte layout, unknown sentinel helpers, boundary values, side-data constructor/deferred accessor behavior, non-matching kind behavior, malformed short/long payloads, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_AUDIO_SERVICE_TYPE` payload in addition to the existing display-matrix, LCEVC, spherical, mastering-display metadata, A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketAudioServiceType` models the pinned four-byte native `AVAudioServiceType` enum payload, preserves all public raw values 0..=8 with exact FFmpeg constant names, and rejects malformed lengths or unknown values through raw and deferred side-data paths. Focused unit tests cover enum value/constant round trips, native byte layout, side-data constructor/deferred accessor behavior, non-matching kind behavior, malformed short/long payloads, invalid raw values, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_MASTERING_DISPLAY_METADATA` payload in addition to the existing A53 closed captions, ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketMasteringDisplayMetadata` models the pinned native `AVMasteringDisplayMetadata` packet side-data shape as an 88-byte payload, preserves the CIE 1931 display primary/white-point `AVRational` fields, min/max luminance rationals, and raw `has_primaries`/`has_luminance` flags, including raw rational bit patterns with zero denominators. Focused unit tests cover native byte layout, raw rational and flag preservation, side-data constructor/deferred accessor behavior, non-matching kind behavior, malformed short/long payloads, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_A53_CC` payload in addition to the existing ICC profile, content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketA53ClosedCaptions` preserves the raw ATSC A/53 closed-caption bytes while exposing whole three-byte CC entries, accepts empty payloads, and rejects malformed non-entry-multiple byte counts through both constructor and deferred accessor paths. Focused unit tests cover raw byte preservation, entry count/iteration/index lookup, empty payload behavior, side-data constructor/deferred accessor behavior, non-matching kind behavior, malformed one/two/four-byte payloads, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_ICC_PROFILE` payload in addition to the existing content-light, quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketIccProfile` models FFmpeg's ISO 15076-1 opaque ICC profile side-data buffer by validating the 132-byte minimum envelope, big-endian declared profile size, `acsp` signature, and bounded tag table, while exposing the profile version, device class, color space, profile connection space, and tag count. Focused unit tests cover a minimal profile, one-record tag table, side-data constructor/deferred accessor behavior, non-matching kind behavior, short payloads, declared-size mismatch, missing signature, truncated tag table, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_CONTENT_LIGHT_LEVEL` payload in addition to the existing quality-stats, fallback-track, CPB properties, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketContentLightMetadata` models FFmpeg's native `AVContentLightMetadata` side-data shape as an 8-byte native-endian payload containing MaxCLL and MaxFALL values. Focused unit tests cover native byte layout, raw `u32` boundary values, side-data constructors/deferred accessors, non-matching kind behavior, malformed lengths, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_CPB_PROPERTIES` payload in addition to the existing quality-stats, fallback-track, producer-reference-time, RTCP sender report, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketCpbProperties` models FFmpeg's native `AVCPBProperties` side-data shape as a 40-byte native-endian payload containing max/min/average bitrate, buffer size, and VBV delay; constructors and parsers reject negative bitrate/buffer-size fields while preserving valid boundary values including `UINT64_MAX` unknown VBV delay. Focused unit tests cover native byte layout, zero/max boundary values, side-data constructors/deferred accessors, non-matching kind behavior, malformed lengths, negative constructor rejection, negative raw-payload rejection, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_RTCP_SR` payload in addition to the existing quality-stats, fallback-track, producer-reference-time, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketRtcpSenderReport` models FFmpeg's native `AVRTCPSenderReport` side-data shape as a 32-byte native-endian payload on this target, preserving `ssrc`, `ntp_timestamp`, `rtp_timestamp`, sender packet/octet counts, four alignment-padding bytes, and four tail-padding bytes; constructors zero both padding regions while parsers preserve them. Focused unit tests cover native byte layout, field boundary values, zero and nonzero padding preservation, side-data constructors/deferred accessors, non-matching kind behavior, malformed lengths, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_PRFT` payload in addition to the existing quality-stats, fallback-track, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketProducerReferenceTime` models FFmpeg's native `AVProducerReferenceTime` side-data shape as a 16-byte native-endian payload containing `int64_t wallclock`, native `int flags`, and four preserved tail-padding bytes on the current target; constructors zero the padding while parsers preserve it. Focused unit tests cover wallclock/flags values, native byte layout, zero and nonzero padding preservation, side-data constructors/deferred accessors, non-matching kind behavior, malformed lengths, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_FALLBACK_TRACK` payload in addition to the existing quality-stats, parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketFallbackTrack` models FFmpeg's native `int` side-data value as a 4-byte native-endian stream index, accepts nonnegative indexes, rejects malformed lengths and negative indexes, and exposes `SideData` constructor/deferred accessor hooks. Focused unit tests cover stream index boundary values, native byte layout, side-data constructors/deferred accessors, non-matching kind behavior, malformed lengths, negative constructor rejection, negative raw payload rejection, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_QUALITY_STATS` payload in addition to the existing parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, S12M timecode, and frame-cropping payloads. `PacketQualityStats` preserves the little-endian quality factor, `AVPictureType` byte, counted little-endian error-sum array, and any trailing bytes beyond the counted array, while rejecting malformed headers, quality values outside 1..=FF_LAMBDA_MAX, invalid picture-type bytes, nonzero reserved bytes, and truncated error arrays. Focused unit tests cover constructor/accessor behavior, FFmpeg picture-type constants, empty and populated error arrays, side-data constructors/deferred accessors, trailing-byte preservation, non-matching kind behavior, malformed headers, invalid quality values, excessive constructor error counts, invalid picture types, nonzero reserved bytes, truncated arrays, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_S12M_TIMECODE` payload in addition to the existing parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, AFD, and frame-cropping payloads. `PacketS12mTimecode` preserves the four native-endian `uint32_t` words, validates the documented 1..=3 timecode-count word, exposes used timecodes while retaining unused raw slots, and rejects malformed lengths plus invalid counts. Focused unit tests cover constructor and raw-word paths, side-data constructors/deferred accessors, unused-word preservation, non-matching kind behavior, malformed lengths, invalid constructor counts, invalid raw counts, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented single-byte `AV_PKT_DATA_AFD` payload in addition to the existing parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, WebVTT identifier/settings, and frame-cropping payloads. `PacketActiveFormatDescription` mirrors the FFmpeg `AVActiveFormatDescription` value set used by packet and frame AFD side data, exposes byte and constant-name round trips, and rejects malformed lengths plus reserved/unknown AFD values. Focused unit tests cover every accepted AFD enum byte, FFmpeg constant mapping, side-data constructors/deferred accessors, non-matching kind behavior, malformed lengths, invalid values, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses `AV_PKT_DATA_WEBVTT_IDENTIFIER` and `AV_PKT_DATA_WEBVTT_SETTINGS` in addition to the existing parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, Matroska BlockAdditional, and frame-cropping payloads. `PacketWebVttIdentifier` and `PacketWebVttSettings` preserve FFmpeg's raw cue-line bytes without forcing UTF-8, expose fallible UTF-8 views for callers that need text, reject empty/NUL/line-terminator payloads, and reject the WebVTT timestamp arrow in identifier payloads because FFmpeg's demuxer treats arrow-bearing first lines as cue timestamps rather than identifiers. Focused unit tests cover raw byte preservation, UTF-8 and non-UTF-8 views, side-data constructors/deferred accessors, non-matching kind behavior, malformed line payloads, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented `AV_PKT_DATA_MATROSKA_BLOCKADDITIONAL` payload in addition to the existing parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, subtitle-position, and frame-cropping payloads. `PacketMatroskaBlockAdditional` preserves FFmpeg's big-endian 8-byte BlockAddId prefix and arbitrary trailing BlockAdditional bytes, rejects payloads shorter than the prefix, and exposes `SideData` constructor/deferred accessor hooks. Focused unit tests cover big-endian ID layout, empty and non-empty trailing data, non-matching kind behavior, malformed lengths, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented four-field `AV_PKT_DATA_SUBTITLE_POSITION` payload in addition to the existing parameter-change, JP-dual-mono, skip-samples, MPEG-TS stream-id, and frame-cropping payloads. `PacketSubtitlePosition` preserves the four little-endian `u32` subtitle rectangle coordinates (`x1`, `y1`, `x2`, `y2`), rejects malformed lengths, and exposes `SideData` constructor/deferred accessor hooks. Focused unit tests cover boundary coordinate values, byte layout, non-matching kind behavior, malformed lengths, and deferred accessor errors; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now parses the documented one-byte `AV_PKT_DATA_JP_DUALMONO` and `AV_PKT_DATA_MPEGTS_STREAM_ID` payloads in addition to the existing parameter-change, skip-samples, and frame-cropping payloads. `PacketJpDualMono` preserves the selected-channel byte and rejects values outside the documented main/left, sub/right, and both range; `PacketMpegTsStreamId` preserves the full `uint8_t` stream-id range. Focused unit tests cover all JP dual-mono variants, stream-id boundary values, non-matching kind behavior, malformed lengths, and invalid JP channel selections; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data added `AV_PKT_DATA_PARAM_CHANGE` parsing in addition to the existing skip-samples and frame-cropping payloads. `PacketParamChange` preserves the little-endian flags word, optional signed sample-rate field, and optional signed width/height fields in FFmpeg's documented field order, rejects too-short, truncated, trailing-byte, and unknown-flag payloads, and exposes `SideData` constructor/deferred accessor hooks. Focused unit tests cover no-change, sample-rate-only, dimensions-only, combined signed-boundary payloads, non-matching kind behavior, unknown flags, truncation, and trailing bytes; `avutil_core_models` now build-checks the same parser invariants. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data added the first typed payload parsers for `AV_PKT_DATA_SKIP_SAMPLES` and `AV_PKT_DATA_FRAME_CROPPING`. `PacketSkipSamples` preserves the two little-endian skip counts plus start/end reason bytes and rejects malformed lengths or unknown reason values; `PacketFrameCropping` preserves the four little-endian crop fields and rejects malformed lengths. `SideData` exposes constructors and deferred typed accessors for both packet side-data kinds, focused unit tests cover valid bytes, constructors, non-matching kind behavior, and malformed payloads, and `avutil_core_models` fuzz invariants now build-check the same parser contracts. This remains below `complete` because the remaining packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet side-data now has a typed `PacketSideDataKind` inventory matching the 41 public FFmpeg 8.1.1 `AV_PKT_DATA_*` constants before `AV_PKT_DATA_NB`. `SideData` stores the typed kind while preserving the string-facing `kind()` API, known-name aliasing maps FFmpeg constants and common spellings to typed variants, unknown extension names are preserved, and packet lookup/take helpers now work by string or typed kind. Focused unit tests and `avutil_core_models` fuzz invariants cover inventory order, FFmpeg constant mapping, aliases, unknown-name preservation, typed/id lookup, and typed take paths. This remains below `complete` because additional packet side-data payload parsers, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet opaque user-reference metadata now models the `AVPacket.opaque_ref` field for the current Rust packet surface. `Packet` carries optional `BufferRef` opaque storage, exposes set/take/clear helpers, shares it through `ref_from`, transfers it through `move_ref_from`, copies a new shared reference through `copy_props_from`, and releases it through clear/take/drop/unref semantics; focused unit tests and `avutil_core_models` fuzz invariants cover those copy, share, move, take, unref, source-isolation, and release-timing paths. This remains below `complete` because raw `void *opaque`, full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still open.

Previous `avutil-packet` update: packet property copying now mirrors the `AVPacket` copy-props boundary for the modeled fields. `Packet::copy_props_from` copies PTS, DTS, duration, byte position, stream index, flags, and side data while preserving the destination payload `BufferRef`; focused unit tests cover destination-payload preservation, old side-data replacement, property transfer, and copied side-data isolation. The `avutil_core_models` fuzz target now build-checks the same copy-props payload preservation and side-data non-aliasing invariants. This remains below `complete` because full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still blocked.

Previous `avutil-packet` update: packet payload storage now uses the shared `BufferRef` model. `Packet::ref_from` shares payload storage while deep-copying side-data records, `Packet::make_data_writable` detaches shared payloads through copy-on-write, `Packet::move_ref_from` transfers every modeled field and resets the source, and `Packet::unref` resets modeled fields while releasing the previous payload. Focused unit tests cover payload sharing with side-data copy, writable detachment, moved-source reset, destination replacement release timing, and unref release/reset behavior. The `avutil_core_models` fuzz target now build-checks packet reference, copy-on-write, move, and unref invariants. This remains below `complete` because full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still blocked.

Previous `avutil-packet` update: packet side-data now has Rust-native lifecycle helpers shaped after the current `AVPacketSideData` surface. `SideData` exposes length/emptiness and checked shrinking; `Packet` can find the first side-data record by kind, mutate it, shrink it without partial mutation on oversize requests, take or remove the first matching record while preserving duplicate-kind ordering, and clear all side data. Focused unit tests cover lookup/shrink ordering, oversize-shrink no-mutation behavior, and scoped take/remove/clear behavior. The `avutil_core_models` fuzz target now build-checks side-data lifecycle invariants too. This remains below `complete` because full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still blocked.

Previous `avutil-packet` update: packet timestamp rescaling now mirrors the documented `av_packet_rescale_ts` shape for the current Rust packet model. `Packet::rescale_ts` converts valid PTS, DTS, and nonzero duration fields between `Rational` time bases through the shared timebase rescaler, preserves unknown PTS/DTS sentinel values, leaves packet payload, stream index, flags, byte position, and side data untouched, and precomputes all timing fields before assignment so invalid time bases or overflow do not partially mutate the packet. Focused unit tests cover valid rescaling, unknown timestamp preservation, and invalid/overflow no-mutation behavior. The `avutil_core_models` fuzz target now build-checks packet rescale invariants too. This remains below `complete` because full AVPacket ABI/field parity, pinned-oracle/FATE parity, and actual local fuzz execution are still blocked.

Latest `avutil-byteio` update: byte I/O now covers signed 24-bit and signed 48-bit helper paths in addition to the existing unsigned and fixed-width signed helpers. `ByteReader::read_i24_le`/`read_i24_be` and `read_i48_le`/`read_i48_be` sign-extend constrained-width values without advancing on EOF because they use the existing bounded unsigned reads; `ByteWriter::write_i24_le`/`write_i24_be` and `write_i48_le`/`write_i48_be` validate the exact signed range before mutating output. Focused unit tests cover sign extension, full signed read/write round trips, and no-mutation rejection for out-of-range signed 24/48-bit values. The `avutil_byteio` fuzz target now build-checks signed 24/48-bit read/write operations and no-mutation writer failures. This remains below `complete` because pinned-oracle/FATE parity and actual local fuzz execution are still blocked.

Latest current-loop `repo-runtime-guard`/`fate-runner` update: `xtask guard-runtime` now enforces the no-FFmpeg-runtime-linkage policy across Cargo dependency sections, resolved `Cargo.lock` package names, and runtime implementation source scans for wrapper tokens or literal `ffmpeg`/`ffprobe`/`ffplay` process spawns. The guard intentionally leaves oracle/FATE/test/fuzz/xtask tooling outside the runtime-shell-out ban, remains wired into `xtask quick`, `xtask changed`, `xtask full`, and local FATE-runner smoke execution, and `fate-runner` changed-path selection now maps workspace manifests, crate manifests, fuzz/xtask manifests, and `Cargo.lock` to `repo-runtime-guard` while also selecting affected component families for crate manifests. This is a policy/infrastructure guard only; it does not add media parity, oracle snapshots, upstream FATE samples, or actual fuzz execution.

Latest `avutil-bitreader`/`avutil-bitwriter` update: bit I/O now includes unsigned and signed Exp-Golomb helpers for codec bitstreams. `BitReader::read_ue_golomb` and `read_se_golomb` preserve the cursor on EOF, over-range, and signed-overflow errors; `BitWriter::write_ue_golomb` and `write_se_golomb` cover exact small-code vectors, signed round trips, `u64::MAX` unsigned round trips, and `i64::MIN` signed rejection without mutation. The `avutil_bitreader` fuzz target now build-checks Exp-Golomb read/write invariants too. This remains below `complete` because pinned-oracle/FATE parity and actual local fuzz execution are still blocked.

Latest `avutil-frame` update: EXIF frame side-data coverage now includes `AV_FRAME_DATA_EXIF` as a TIFF-header, root IFD-chain, recognized EXIF/GPS/interoperability linked IFD pointer envelope, typed TIFF entry value layer, and selected strict common-tag interpretation including root descriptive/image-layout, linked EXIF exposure/sensitivity/apex/capture/optics/subject/version-timing-comment/related-file/environment/rendering-scene/camera-lens/gamma-composite settings, linked GPS altitude/time/date/acquisition/speed/track/image-direction/map-datum/destination/processing-area/differential/positioning-error metadata, and linked interoperability version/related-image metadata. `FrameExif`, `FrameExifIfd`, `FrameExifEntry`, `FrameExifIfdPointerKind`, `FrameExifLinkedIfd`, `FrameExifCommonTags`, `FrameExifBitsPerSample`, `FrameExifNewSubfileType`, `FrameExifSubfileType`, `FrameExifCompression`, `FrameExifPhotometricInterpretation`, `FrameExifThresholding`, `FrameExifFillOrder`, `FrameExifCompositeImage`, `FrameExifOrientation`, `FrameExifResolutionUnit`, `FrameExifPlanarConfiguration`, `FrameExifYcbCrPositioning`, `FrameExifExposureProgram`, `FrameExifSensitivityType`, `FrameExifMeteringMode`, `FrameExifLightSource`, `FrameExifFlash`, `FrameExifColorSpace`, `FrameExifWhiteBalance`, `FrameExifSensingMethod`, `FrameExifFileSource`, `FrameExifSceneType`, `FrameExifCustomRendered`, `FrameExifExposureMode`, `FrameExifSceneCaptureType`, `FrameExifGainControl`, `FrameExifContrast`, `FrameExifSaturation`, `FrameExifSharpness`, `FrameExifSubjectArea`, `FrameExifSubjectDistanceRange`, `FrameExifGpsLatitudeRef`, `FrameExifGpsLongitudeRef`, `FrameExifGpsAltitudeRef`, `FrameExifGpsStatus`, `FrameExifGpsMeasureMode`, `FrameExifGpsSpeedRef`, `FrameExifGpsDirectionRef`, `FrameExifGpsDistanceRef`, `FrameExifGpsDifferential`, `FrameExifRational`, `FrameExifSignedRational`, and `FrameExifTiffType` preserve raw metadata bytes, byte order, root and linked IFD offsets, parent/source pointer metadata, entry counts, tags, TIFF types, value lengths, inline value bytes, and out-of-line value ranges while rejecting malformed byte-order markers, first/next directory offsets, oversized entry counts, invalid TIFF entry types, invalid linked pointer type/count/offset fields, truncated tables, out-of-range value data, root or linked IFD loops, malformed ASCII strings, zero rational denominators, and invalid selected common-tag type/count/value shapes through typed errors. Current typed parsers or preserved raw views now cover all 32 pinned FFmpeg 8.1.1 frame side-data kinds, but `avutil-frame` remains below `complete` until pinned-oracle/FATE parity, actual fuzz execution, and deeper semantic gaps are closed.

Current typed frame side-data payload coverage includes Pan Scan, A53 closed captions, Stereo3D, display matrix, matrix encoding, downmix info, ReplayGain, AFD, motion vectors, skip samples, audio service type, mastering display metadata, GOP timecode, spherical mapping, content light metadata, ICC profile, S12M timecode, dynamic HDR10+ envelope metadata, regions of interest, video encoding parameters, film grain parameters, detection bounding boxes, Dolby Vision RPU/metadata, HDR Vivid native envelope metadata, ambient viewing environment, video hint, LCEVC, View ID, 3D reference displays, EXIF, and SEI unregistered. This coverage is local parser and envelope coverage only; full EXIF/GPS/interoperability tag coverage beyond the selected common-tag view, color-transform semantics, codec-specific behavior, pinned-oracle differential tests, upstream FATE coverage, and actual local fuzz execution remain pending.

`avutil-frame` now retains BufferRef-backed ownership for video/audio frame planes, frame side-data payloads, and the optional hardware frames context. `FrameData` has an explicit empty/unreferenced state, and `Frame` supports empty/default construction, `unref`, `ref_from`, and `move_ref_from` for clearing, sharing, or moving current frame references while preserving BufferRef release timing. `VideoFrame` and `AudioFrame` expose visible byte snapshots through the existing `planes()` API while retaining underlying plane owners through `plane_buffers()`, validate plane count and visible length, exclude padding from visible snapshots, report writability, detach shared/readonly plane storage through BufferRef copy-on-write, support visible-plane mutation, and keep external plane owners alive until the final frame clone drops. `FrameData` and `Frame` now dispatch current data-plane writability checks, copy-on-write detachment, and visible-plane mutation across audio/video payloads, while `FrameSideData` and `Frame` expose explicit side-data, hw-context, and combined all-reference writable checks plus copy-on-write detachment helpers. `VideoFrame` accepts explicit per-plane line sizes for the current pixel-format subset, computes checked nonzero-alignment-rounded line sizes for that subset, validates caller-provided aligned owned-plane or BufferRef storage, retains full strided storage in `plane_buffers()`, preserves row padding during visible-plane writes, and packs visible rows for `planes()`. `AudioFrame` accepts explicit per-plane line sizes for the shared FFmpeg-native `u8`/`s16`/`s32`/`flt`/`dbl`/`s64` packed and planar sample-format set, computes checked nonzero-alignment-rounded line sizes for that set, validates caller-provided aligned owned-plane or BufferRef storage including explicit channel-layout variants, retains full storage in `plane_buffers()`, preserves padding during visible-plane writes, and exposes only visible sample bytes through `planes()`. `SampleFormat` now exposes descriptor-style numeric kind, family, bit width, and packed/planar counterpart metadata for that current set. `FrameSideData` carries a typed `FrameSideDataKind` inventory matching the 32 FFmpeg 8.1.1 `AV_FRAME_DATA_*` constants from `libavutil/frame.h` in header order, exact constant-name mapping, FFmpeg descriptor names and `AV_SIDE_DATA_PROP_*` flags from `libavutil/side_data.c`, BufferRef payload, per-record `Dictionary` metadata, payload writability checks, and writable data access; known-kind aliases normalize common FFmpeg side-data names while unknown extension names are preserved. `Frame` clone, typed lookup, explicit side-data replacement with duplicate coalescing, remove, remove-by-property, take, replace, make-writable, and drop paths preserve side-data and `hw_frames_context` BufferRef sharing and release timing, including external opaque owner release once the final frame/context reference is gone. The `avutil_core_models` fuzz harness now build-checks frame plane BufferRef ownership, custom and aligned video line-size handling, custom and aligned packed/planar audio line-size handling across the current native sample-format set, sample-format descriptor invariants, direct VideoFrame/AudioFrame make-writable readonly plane detachment, writable plane detachment/mutation, top-level Frame/FrameData writable dispatch, Frame::make_writable data-plane-only detachment while side-data/hw-context references remain shared, full-reference side-data/hw-context writable detachment, empty/unref/ref_from/move_ref_from behavior including unref release timing for external plane/side-data/hw-context owners, ref_from destination replacement/source-reference sharing, and move_ref_from destination replacement release/moved-source lifetime, frame side-data typed-kind inventory/order/constant mapping, descriptor names/properties, Pan Scan/A53 closed-caption/Stereo3D/display matrix/matrix encoding/downmix info/replay gain/active format description/motion vectors/skip samples/audio service type/mastering display metadata/GOP timecode/spherical mapping/content light metadata/ICC profile/S12M timecode/dynamic HDR10+/ROI/video-enc-params/film-grain/detection-bboxes/Dolby Vision RPU and metadata/HDR Vivid/ambient viewing/video hint/LCEVC/View ID/3D reference displays/EXIF/SEI payload parsing or preservation, side-data descriptor property flags, multi-instance flags, remove-by-property ordering and empty-property no-op behavior, remove-by-name/alias behavior, take-side-data metadata and BufferRef preservation, set-style alias duplicate replacement and external side-data payload release timing, validation/replacement/removal/metadata/storage sharing, padding exclusion, hw-frames-context attachment, sharing, typed opaque lookup, take, clone-retention, replacement release ordering, and release invariants. This is still a Rust ownership model only; deeper payload semantics beyond descriptor metadata and the current Pan Scan/A53 closed-caption/Stereo3D/display matrix/matrix encoding/downmix info/replay gain/AFD/motion vectors/skip samples/audio service type/mastering display metadata/GOP timecode/spherical mapping/content light metadata/ICC profile/S12M timecode/dynamic HDR10+ envelope/ROI/video-enc-params/film-grain/detection-bboxes/Dolby Vision RPU and metadata/HDR Vivid/ambient viewing/video hint/LCEVC/View ID/3D reference displays/EXIF/SEI parsers, deeper HDR10+/Dolby Vision/HDR Vivid color transform semantics, ROI encoder constraint handling, codec-specific video encoding parameter quantizer semantics, film grain synthesis/selection behavior, detection-model semantics beyond the native envelope, hardware device/frame context internals, actual hardware formats, deeper AVFrame per-buffer permission flags beyond the current BufferRef-backed writable/readonly model, complete FFmpeg alignment policy, sample-data conversion routines, and pinned-oracle/FATE parity remain absent.

Current avutil-frame coverage also now includes deterministic build-checked `FrameDolbyVisionRpuBuffer` non-empty and empty raw payload preservation plus `FrameDolbyVisionMetadata` malformed length/offset/size/count/level/color coverage, `FrameDynamicHdrVivid` nested maximum-RGB/tone-mapping/three-spline/color-saturation preservation plus malformed length/system/window/tone-mapping/three-spline/color-saturation coverage, `FrameAmbientViewingEnvironment` illuminance/chromaticity preservation plus malformed length/denominator/sign/range coverage, `FrameVideoHint` Changed/Constant payload preservation plus malformed length/offset/size/type/count/rectangle/overflow coverage, and `FrameViewId` signed-boundary preservation plus malformed length coverage, matching the focused side-data unit coverage for raw/deferred side-data paths and constructor rejection paths where the component exposes a typed constructor.

Update for the current slice: View ID coverage now has build-checked fuzz invariants matching the malformed-input branches in `frame_side_data_rejects_malformed_view_id_payload` and strengthening raw-value preservation in `frame_side_data_parses_view_id_payload`. The deterministic `avutil_core_models` fixture now asserts native 4-byte signed `int` payload sizing, positive/negative/signed-boundary round trips, raw byte preservation, empty/short/long payload rejection, and non-view-id lookup absence. `avutil-frame` remains below `complete` because pinned-oracle differential tests, upstream FATE coverage, actual local fuzz execution, and broader AVFrame semantics are still absent.

`avutil-buffer` now has the first Rust-native AVBufferRef-shaped primitive, callback-owned release hooks, readonly owned/callback-owned storage, borrowed static readonly storage, shared Arc-slice readonly storage, external opaque readonly storage, typed opaque owner lookup, visible data pointer access, storage identity checks, fallible resize/reallocation, explicit zeroed padding support, and an exact-shape buffer pool with custom allocation/release callbacks. `BufferRef` wraps reference-counted byte storage with owned/copy constructors, readonly constructors, borrowed static readonly constructors, shared Arc-slice readonly constructors, external opaque readonly constructors, typed opaque owner lookup, visible/padded data pointer helpers, storage identity checks for refs and slices, callback-owned storage release callbacks, fallible zeroed allocation, optional zeroed padding, separate visible and allocated lengths, checked immutable slice views bounded to visible bytes, writability reporting, unique mutable access, copy-on-write mutation, and checked resize with zeroed growth/padding. Callback-owned buffers validate visible lengths, release their original allocation only after the final `BufferRef`/`BufferSlice` reference drops, and keep copy-on-write clones independently owned. Readonly buffers report non-writable even when unique, deny direct mutable access, detach to ordinary writable storage on `make_mut()` or resize, preserve visible/padded bytes across detach, and release unique callback-owned readonly storage when detaching. Borrowed static readonly buffers retain their static storage pointer until `make_mut()` or resize detaches them to ordinary owned storage, reject invalid visible lengths, and keep shared static references untouched after detach. Shared Arc-slice readonly buffers retain their shared storage pointer and Arc ownership until `make_mut()` or resize detaches them to ordinary owned storage, reject invalid visible lengths without taking the caller Arc, and keep shared readonly references untouched after detach. External opaque readonly buffers retain their shared byte-storage pointer until detach, expose typed opaque owner references while the original storage remains alive, return no opaque reference for detached copies or mismatched types, release their opaque owner exactly once when the original storage is released, release unique originals during `make_mut()` detach, wait for the final original reference when resize detaches shared storage, reject invalid visible lengths without invoking the release callback, and keep external bytes readonly. Storage identity checks distinguish shared refs and slices from equal independent bytes, and pointer helpers expose the current visible/padded data pointers for buffer and slice views. Resize rejects overflowing visible+padding shapes without mutating the original, zeroes padding even for same-shape requests, detaches shared storage without changing the original reference, detaches pool-owned storage back to its pool instead of changing pool shape, and delays release of shared callback-owned originals until the final original reference drops. `BufferPool` issues zeroed buffers, manually recycles only unique non-readonly buffers matching the pool's visible/padded shape, automatically returns pool-owned storage after the last shared reference drops, supports custom exact-size allocation and permanent-release callbacks, releases spare storage when the pool drops, releases outstanding storage after pool drop once the final reference disappears, rejects bad allocator shapes without retaining them, and avoids returning copy-on-write cloned allocations to the pool. The `avutil_core_models` fuzz harness now build-checks buffer construction, slicing, sharing, visible/padded byte boundaries, callback release timing, invalid visible-length rejection, readonly writability/detach/release invariants, borrowed static readonly pointer retention and detach, shared Arc-slice readonly ownership and detach, external opaque readonly lookup/release timing and detach, storage identity and pointer invariants, resize prefix preservation, zeroed resize growth/padding, same-shape padding zeroing, resize overflow non-mutation, shared resize detach, custom pool callback allocation/release, bad allocator-shape rejection, copy-on-write isolation, zeroed allocation, zeroed padding, length-overflow rejection, pool shape validation, readonly/shared-buffer recycle rejection, automatic return-on-last-drop, pool-backed copy-on-write return behavior, and zeroing on reuse invariants. `tests/fate/mappings.txt` also has focused local `avutil` unit-test mappings, and `fate-runner` has a regression test proving current avutil changed-path selections resolve to runnable local smoke commands. This remains local unit/fuzz-build coverage only, not upstream FFmpeg FATE or pinned-oracle parity.

`fate-runner` now has local smoke mappings for current shared `fftools` changed-path selections and the first avcodec decoder components selected by shared frame-model changes. `tests/fate/mappings.txt` maps version and hide-banner paths to focused `fftools` unit filters, maps option parsing and basic I/O planning to focused unit filters, maps each current `fftools-ffmpeg-*` ledger component to the shared `cargo test -p fftools --lib ffmpeg` path, maps each current `fftools-ffprobe-*` ledger component to the shared `cargo test -p fftools --lib ffprobe` path, and maps `avcodec-rawvideo`/`avcodec-pcm-s16le` to focused `cargo test -p avcodec` filters. The `default_mappings_cover_current_fftools_smoke_selections` regression test loads the real ledger and default mapping file and now covers `fftools` lib/bin, ffmpeg, ffprobe, option-parser, I/O-plan, and option-parser fuzz-target selections. These are local smoke checks only; they do not count as upstream FFmpeg FATE media parity.

`fftools` now parses the current FFmpeg-style compound `-loglevel`/`-v` directive surface in both the shared ffmpeg option parser and the ffprobe parser. `CliLogConfig` carries the selected `LogLevel` plus `LogFlags`, accepts exact names, numeric severities, `repeat`/`level`/`time`/`datetime` flag+level values such as `repeat+level+debug`, and `+flag`/`-flag` directives that update flags without changing the last selected level. The `fftools_option_parser` fuzz harness now treats these directives as valid loglevel values. This is still parser/planning coverage only; byte-identical stderr formatting, repeated-line behavior, time/datetime rendering, color handling, and oracle differential coverage remain absent.

`fate-runner` changed-component selection now treats mapped cargo-fuzz target files as implementation paths instead of ignoring them. A change to `fuzz/fuzz_targets/avformat_mov.rs` selects `avformat-mov-demuxer`, and related existing fuzz targets select their ledger components in ledger order. This keeps fuzz-only MOV changes tied to the local MOV smoke mapping, though upstream FFmpeg FATE media parity is still blocked by missing samples and oracle configuration.

`avformat_mov` fuzz harness now deterministically seeds constrained MOV fixtures for `tx3g`, `stpp`, `sbtt`, `stxt`, `wvtt`, `metx`, `mett`, and `urim` sample entries and asserts the parsed typed detail invariants for each seeded entry. The seeds also run through packet extraction, lazy `tx3g`/`wvtt` sample payload validation, side-data checks, and the existing generic MOV parser invariants. This is build-checked and clippy-clean coverage only; actual local fuzz execution remains blocked because `cargo-fuzz` is not installed.

`avformat-mov-demuxer` now parses optional `btrt` bitrate boxes for `wvtt` WebVTT sample entries. The `wvtt` path exposes typed bitrate metadata alongside the existing required `vttC` configuration, optional `vlab` source label, and preserved child boxes, while rejecting truncated or duplicate `btrt` boxes as typed errors. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, WebVTT rendering/timing parity, and broader subtitle/data coverage remain absent.

`avformat-mov-demuxer` now parses optional `btrt` bitrate boxes for `stpp` XML subtitle and `metx` XML metadata sample entries. The `stpp` path exposes typed bitrate metadata alongside the existing namespace, schema-location, auxiliary-MIME-types, and preserved child boxes. The `metx` path now accepts validated trailing child boxes, exposes typed optional bitrate metadata, preserves those child boxes, and rejects truncated or duplicate `btrt` boxes as typed errors. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, XML subtitle/metadata payload parity, and broader subtitle/data coverage remain absent.

`avformat-mov-demuxer` now parses `stxt` simple text sample entries and `urim` URI metadata sample entries. The `stxt` path reuses the structured text entry model for null-terminated UTF-8 `content_encoding` and `mime_format` strings, optional `btrt` bitrate boxes, optional `txtC` text configuration full boxes, preserved child boxes, and duplicate optional-box rejection. The `urim` path validates a required `uri ` full box, optional `uriI` full-box initialization payload, optional `btrt`, preserved child boxes, full-box version/flag constraints, UTF-8/null-terminated URI strings, duplicate required/optional boxes, and malformed child boxes as typed errors. `mett` text metadata entries now also accept and validate optional `btrt`/`txtC` child boxes. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, text/metadata sample payload parity, and broader subtitle/data coverage remain absent.

`avformat-mov-demuxer` now parses boxed `tx3g` Timed Text media samples through `parse_timed_text_sample` and validates them lazily when `tx3g` packets are read. It decodes the 16-bit text byte count, UTF-8 text, UTF-16 text with big-endian or little-endian BOMs, `styl` style records, `hlit` static highlight ranges, `hclr` highlight colors, `krok` karaoke event records, `dlay` scroll delays, `href` hyperlink ranges and UTF-8 URL/alt strings, `tbox` text-box overrides, `blnk` blink ranges, `twrp` wrap flags, and `disp` stereo disparity shifts while preserving unknown modifier boxes. It rejects malformed text lengths, invalid text encodings, malformed style/range/karaoke/hyperlink records, duplicate global modifiers, reserved wrap flags, malformed modifier boxes, and lazy packet-read failures as typed errors. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, Timed Text rendering/timing parity, and broader subtitle/data coverage remain absent.

`avformat-mov-demuxer` now parses boxed `wvtt` WebVTT media samples through `parse_webvtt_sample` and validates them lazily when `wvtt` packets are read. It models `vttc` cue boxes, `vtte` empty-cue samples, `vtta` additional text boxes, `vsid`/`iden`/`ctim`/`sttg`/`payl` cue child boxes, rejects malformed payloads as typed errors, requires `vsid` samples to have a sample-entry `vlab` source label, and adds Rust packet side data for WebVTT sample kind, cue count, and additional-text count. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, and broader subtitle/data coverage remain absent.

`avformat-mov-demuxer` now parses `sbtt` text subtitle sample entries into a structured `MovTextSubtitleSampleEntry` model. It captures the ISO Base Media File Format null-terminated UTF-8 `content_encoding` and `mime_format` strings, validates optional `btrt` bitrate boxes, validates optional `txtC` text configuration full boxes, preserves child boxes, and rejects missing terminators, invalid UTF-8, malformed child boxes, unsupported `txtC` versions, nonzero `txtC` flags, duplicate optional boxes, and truncated bitrate data as typed errors. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, and broader subtitle/data sample-entry coverage remain absent.

`avformat-mov-demuxer` now parses `wvtt` WebVTT sample entries into a structured `MovWebVttSampleEntry` model. It validates the required `vttC` WebVTT configuration box, captures its UTF-8 WebVTT header text, enforces a WebVTT signature boundary, captures an optional `vlab` source-label box, preserves validated child boxes, and rejects missing or duplicate `vttC`, duplicate `vlab`, invalid UTF-8, invalid WebVTT signatures, and malformed child-box tails as typed errors. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, and broader subtitle/data sample-entry coverage remain absent.

`avformat-mov-demuxer` now parses `stpp` XML subtitle sample entries into a structured `MovXmlSubtitleSampleEntry` model. It captures the ISO Base Media File Format null-terminated UTF-8 `namespace`, optional `schema_location`, and `auxiliary_mime_types` fields, preserves validated optional child boxes after those strings, and rejects missing terminators, invalid UTF-8, and malformed child-box tails as typed errors. The ledger keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, deeper optional subtitle box semantics, and broader subtitle/data sample-entry coverage remain absent.

`avformat-mov-demuxer` now parses required `alac` Apple Lossless specific boxes for `alac` AudioSampleEntry records, including the current direct full-box form and the older QuickTime `wave/frma` + nested `wave/alac` compatibility form. The parsed model captures ALAC frame length, compatible version, bit depth, tuning parameters, channel count, max run, max frame bytes, average bit rate, sample rate, and optional ALAC channel-layout-info fields, and `MovAudioSampleEntry` now derives effective ALAC channel count, bit depth, and sample rate from the ALAC config. Malformed missing/duplicate/direct-plus-wave `alac` boxes, unsupported box/config versions, nonzero flags, zero frame length, zero/oversized bit depth, zero channel count, zero sample rate, mismatched sample-entry channel/bit-depth/sample-rate fields, truncated channel-layout info, and invalid ALAC channel-layout reserved fields are rejected as typed errors. The ledger still keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, additional codec-specific audio extensions beyond direct `esds`/`btrt`/`damr`/`dac3`/`dec3`/`dOps`/`dfLa`/`alac`/`wave`/`chan`, and broader subtitle/data sample-entry coverage remain absent.

`fate-runner` now has a local `avformat-mov-demuxer|local-mov-unit` mapping in `tests/fate/mappings.txt` that executes `cargo test -p avformat mov::tests` through the same component-selection runner path used for future FATE work. This unblocks explicit local smoke validation for the current MOV component and future `crates/avformat/src/mov.rs` changed-path selections, but it is documented as local unit-test smoke coverage only and does not count as upstream FFmpeg FATE media parity.

`avformat-mov-demuxer` now parses direct audio sample-entry `esds` child boxes into the same structured `MovElementaryStreamDescriptor` model used for nested `wave/esds` atoms, direct audio sample-entry `btrt` bitrate boxes into a structured `MovBitRateBox` model, `damr` AMR decoder-specific boxes into a structured `MovAmrSpecificBox` model, required `dac3` AC-3-specific boxes for `ac-3` sample entries into a structured `MovAc3SpecificBox` model, required `dec3` E-AC-3-specific boxes for `ec-3` sample entries into a structured `MovEc3SpecificBox` model, mandatory `dOps` Opus-specific boxes for `Opus` sample entries into a structured `MovOpusSpecificBox` model, and required `dfLa` FLAC-specific boxes for `fLaC` sample entries into a structured `MovFlacSpecificBox` model. Direct `esds` payloads are validated as version-0 full boxes with zero flags and non-empty descriptor payloads, direct `btrt` payloads are validated as exactly three big-endian u32 fields, direct `damr` payloads are validated as exactly the 3GPP AMRDecSpecStruc field layout with `frames_per_sample` in 1..16, direct `dac3` payloads are validated as exactly 24 bits with non-reserved `fscod`, `bit_rate_code` in 0..=18, and zero reserved bits, direct `dec3` payloads are validated as bit-packed EC3SpecificBox data with zero mandatory reserved bits, required independent substream records, and spec-allowed trailing reserved bytes, direct `dOps` payloads are validated as version 0 with nonzero output channel count, required no trailing bytes for mapping-family 0, optional mapping table sizing for nonzero mapping families, nonzero stream count, and `coupled_count <= stream_count`, and direct `dfLa` payloads are validated as version-0 full boxes with zero flags, at least one native FLAC metadata block, first and unique STREAMINFO metadata, valid last-block flag placement, forbidden block-type rejection, and parsed STREAMINFO sample-rate/channel-count/bit-depth/block-size fields. Malformed direct `esds` versions, empty descriptors, truncated/oversized `btrt` payloads, truncated/oversized/invalid-frame-count `damr` payloads, missing or duplicate `ac-3` `dac3` boxes, invalid `dac3` bitfields, missing or duplicate `ec-3` `dec3` boxes, invalid `dec3` bitfields, missing or duplicate `Opus` `dOps` boxes, unsupported `dOps` versions, truncated/oversized/invalid `dOps` mapping payloads, missing or duplicate `fLaC` `dfLa` boxes, unsupported `dfLa` versions, invalid `dfLa` flags, malformed FLAC metadata block sequences, malformed STREAMINFO payloads, and mismatched FLAC sample-entry channel/bit-depth fields are rejected as typed errors. The ledger still keeps MOV below `complete` because pinned-oracle differential tests, upstream FATE media parity, actual local fuzz execution, additional codec-specific audio extensions beyond direct `esds`/`btrt`/`damr`/`dac3`/`dec3`/`dOps`/`dfLa`/`wave`/`chan`, and broader subtitle/data sample-entry coverage remain absent.

`avformat-mov-demuxer` now parses known MOV/MP4 `AudioSampleEntry` version 0, 1, and 2 metadata plus direct `esds` descriptor child atoms, direct `btrt` bitrate child atoms, direct `damr` AMR decoder-specific child atoms for `samr`/`sawb` sample entries, required direct `dac3` AC-3-specific child atoms for `ac-3` sample entries, required direct `dec3` E-AC-3-specific child atoms for `ec-3` sample entries, required direct `dOps` Opus-specific child atoms for `Opus` sample entries, required direct `dfLa` FLAC-specific child atoms for `fLaC` sample entries, structured QuickTime `wave` extension child atoms, structured audio `chan` channel-layout child atoms, `tx3g` timed-text sample-entry display/default-box/default-style fields and child boxes, and `metx`/`mett` metadata sample-entry null-terminated string fields. The parsed model captures base audio version/revision/vendor/channel-count/sample-size/compression-ID/packet-size/sample-rate fields, version-1 packet sizing fields, version-2 sizeOfStructOnly/audioSampleRate/numAudioChannels/constBitsPerChannel/formatSpecificFlags/constBytesPerAudioPacket/constLPCMFramesPerAudioPacket fields, ordinary child boxes, direct and `wave/esds` full-box descriptor payloads, `btrt` buffer-size/max-bitrate/average-bitrate fields, `damr` vendor/decoder-version/mode-set/mode-change-period/frames-per-sample fields, `dac3` AC-3 fscod/bsid/bsmod/acmod/lfeon/bit-rate-code fields, `dec3` E-AC-3 data-rate/independent-substream/dependent-substream/chan-loc fields, `dOps` Opus version/output-channel-count/pre-skip/input-sample-rate/output-gain/channel-mapping-family fields plus optional stream-count/coupled-count/channel-mapping tables, `dfLa` FLAC metadata block sequences and STREAMINFO sample-rate/channel-count/bit-depth/block-size/total-samples/MD5 fields, `wave/frma` original-format FourCCs, `wave` terminator presence, `chan` channel layout tags/bitmaps/description labels/flags/coordinate bits, `tx3g` text box/style records, XML metadata content encoding/namespace/schema location, and timed text metadata content encoding/MIME format, with invalid truncated entries, unknown audio versions, invalid version-2 offsets/rates, malformed child boxes, malformed direct `esds`/`btrt`/`damr`/`dac3`/`dec3`/`dOps`/`dfLa` payloads, malformed `wave`/`frma`/`esds`/terminator atoms, malformed `chan` payloads, and malformed `tx3g`/`metx`/`mett` payloads rejected as typed errors. `ffprobe-rs` still surfaces constrained MOV audio stream `sample_rate`, `channels`, and `bits_per_sample` fields from that Rust parser, including version-2 effective channel and bit-depth values and FLAC STREAMINFO-derived effective fields, and both `ffprobe-rs` and `ffmpeg-rs` can fall back from missing MOV handlers to parsed sample-entry detail kinds for audio/video/subtitle/data stream typing. The ledger keeps the affected MOV, ffprobe, and streamhash entries at `implemented`, not `complete`, because pinned-oracle differential tests, upstream FATE coverage, additional codec-specific audio extensions beyond direct `esds`/`btrt`/`damr`/`dac3`/`dec3`/`dOps`/`dfLa`/`wave`/`chan`, broader subtitle/data sample-entry coverage, decoder-derived fields, and actual local fuzz execution are still absent.

`fftools-ffmpeg-hash-output`, `fftools-ffmpeg-md5-output`, `fftools-ffmpeg-framehash-output`, `fftools-ffmpeg-framemd5-output`, and `fftools-ffmpeg-streamhash-output` are now implemented for the constrained Rust-native `ffmpeg-rs` execution path. `ffmpeg-rs` accepts one supported local input to stdout `-f hash -`, defaults to SHA-256, accepts output-scoped `-hash` for Adler-32, CRC-32, MD5, SHA-224, SHA-256, SHA-384, and SHA-512, feeds packet payloads through `avformat::HashMuxer`, and rejects unknown hash algorithm names without invoking FFmpeg at runtime. The dedicated `-f md5 -` muxer routes the same packet payload stream through MD5 while preserving the observable output format name as `md5` and rejecting the generic hash muxer's `-hash` option. `ffmpeg-rs` also accepts constrained stdout `-f framehash -`, defaults that packet-record muxer to SHA-256, accepts output-scoped `-hash` on framehash, routes `-f framemd5 -` through the same per-packet record path with fixed MD5 while rejecting generic `-hash` on framemd5, and accepts `-f streamhash [-hash <algorithm>] -` with one accumulated digest line per stream. The streamhash command path now derives labels through a local stream-type map: AVI from demuxer stream media types, MOV from parsed `hdlr` handler types when available, MOV parsed sample-entry detail kinds and video dimensions as fallback, and raw/WAV/YUV4MPEG2/image2 from explicit single-stream metadata; the tests cover MOV video SHA-256 output, MOV audio-handler SHA-256 output, raw `pcm_s16le` MD5 output, AVI SHA-256 output, direct AVI/MOV metadata-map checks, and missing stream-metadata rejection. The ledger keeps the CLI hash/framehash/streamhash components at `implemented`, not `complete`, because exact FFmpeg hash/md5/framehash/framemd5/streamhash muxer output semantics, pinned-oracle differential tests, FATE coverage, broader command forms, and actual local fuzz execution are still absent.

`avformat-hash-muxer` now supports SHA-224, SHA-256, SHA-384, and SHA-512 packet-data hashing in addition to Adler-32, IEEE CRC-32, and MD5, and is wired into constrained `ffmpeg-rs -f hash [-hash <algorithm>] -` and `-f md5 -` stdout execution. `HashMuxerReport` carries a variable-width `HashDigest`, formats MD5/SHA digest lines as lowercase hex, preserves packet/byte accounting, and the `avformat_packet_muxers` fuzz target now build-checks SHA-224/SHA-256/SHA-384/SHA-512 digest stability alongside Adler-32/CRC-32/MD5. The ledger still keeps `avformat-hash-muxer` at `implemented`, not `complete`, because exact FFmpeg hash/md5 muxer output semantics, pinned-oracle differential tests, FATE coverage, other FFmpeg hash algorithms, and actual local fuzz execution are still absent.

`avformat-framehash-muxer` now records one digest line per packet with stream index, PTS/DTS, duration, payload size, and Adler-32/CRC-32/MD5/SHA-224/SHA-256/SHA-384/SHA-512 digest support. It is wired into constrained `ffmpeg-rs -f framehash [-hash <algorithm>] -` and `-f framemd5 -` stdout execution, and `avformat_packet_muxers` now build-checks framehash/framemd5 per-packet digest record fields alongside null/hash/framecrc packet muxer invariants. The ledger keeps `avformat-framehash-muxer` at `implemented`, not `complete`, because exact FFmpeg framehash/framemd5 output semantics, pinned-oracle differential tests, FATE coverage, and actual local fuzz execution are still absent.

`avformat-streamhash-muxer` now records one digest line per stream with stream index, stream type, algorithm, packet count, byte count, and Adler-32/CRC-32/MD5/SHA-224/SHA-256/SHA-384/SHA-512 digest support. It is wired into constrained `ffmpeg-rs -f streamhash [-hash <algorithm>] -` stdout execution, keeps records ordered by stream index, rejects conflicting stream-type labels without changing a digest, and `fftools` now derives stream-type labels from AVI media types, MOV `hdlr` handler types, MOV video sample-entry/dimension metadata fallback, or explicit single-stream metadata while rejecting missing packet stream indexes. `avformat_packet_muxers` now build-checks streamhash per-stream digest invariants alongside null/hash/framecrc/framehash/framemd5 packet muxer invariants. The ledger keeps `avformat-streamhash-muxer` at `implemented`, not `complete`, because exact FFmpeg streamhash output semantics, generated stream type mapping, pinned-oracle differential tests, FATE coverage, and actual local fuzz execution are still absent.

`avutil-hash` now includes Rust-native SHA-224, SHA-256, SHA-384, and SHA-512 support alongside Adler-32, IEEE CRC-32, and MD5 helpers. `Sha224`, `Sha256`, `Sha384`, and `Sha512` support one-shot and streaming updates across block boundaries, `digest_to_hex` formats lowercase digest output for byte digests, and `avutil_core_models` now build-checks SHA-224/SHA-256/SHA-384/SHA-512 streaming equivalence and hex invariants in addition to Adler-32/CRC-32/MD5 streaming equivalence. The ledger still keeps `avutil-hash` at `implemented`, not `complete`, because pinned-oracle differential vectors, FATE coverage, actual local fuzz execution, other FFmpeg hash algorithms, and full FFmpeg hash/framehash behavior are still absent.

`avutil-options` now records known public AVOption flag bits, rejects mutation of read-only options, supports unit-scoped named constants, exposes int/float/rational range queries, registers child option sets with independent option namespaces, and filters root or direct-child option definitions through an `OptionQuery` model. `OptionFlags` truncates unknown bits and reports intersections, `OptionDefinition` stores flags and optional units alongside type/default/help metadata, `OptionSet` resolves constants case-insensitively for matching units during parsed string mutation, failed constant resolution or type mismatch leaves state unchanged, `OptionRange` reports validated numeric bounds for int/float options and positive-denominator rational bounds for rational options, `OptionChild` stores named child option sets with validated metadata and case-insensitive duplicate rejection, and `OptionQuery` filters by case-insensitive name, unit, required flags, rejected flags, and exported/writable presets. The `avutil_metadata_options` fuzz target now build-checks generated option flags, unknown-bit truncation/intersection, unit constant scoping, int/float/rational range invariants, read-only mutation rejection, child option-set registration, query filtering, and child invariants. The ledger still keeps `avutil-options` at `implemented`, not `complete`, because CLI option ordering, full AVOption API parity, pinned-oracle differential vectors, FATE coverage, and actual local fuzz execution are still absent.

`avutil-dict` now supports explicit duplicate-key insertion for AV_DICT_MULTIKEY-like metadata cases. Duplicate entries preserve insertion order, ordinary lookup still returns the first matching entry, case-sensitive lookup can target the duplicate spelling, and removal deletes the first requested match. The `avutil_metadata_options` fuzz target now build-checks duplicate-key insertion alongside the existing mutation and failed-mutation invariants. The ledger still keeps `avutil-dict` at `implemented`, not `complete`, because pinned-oracle differential vectors, FATE coverage, full AVDictionary flag parity, and actual local fuzz execution are still absent.

`avutil-bitreader` and `avutil-bitwriter` now handle signed bitfields in addition to unsigned MSB-first fields. `BitReader` exposes signed read/peek helpers with sign extension and no-advance failure behavior, `BitWriter` validates signed ranges before mutation, and the `avutil_bitreader` fuzz target now build-checks signed read/peek/write cursor invariants. The ledger still keeps both components at `implemented`, not `complete`, because deeper FFmpeg GetBitContext/PutBitContext compatibility vectors, pinned-oracle differential tests, FATE coverage, and actual local fuzz execution are still absent.

`avutil-byteio` now has shared endian-aware 48-bit unsigned read/write helpers. The writer validates u48 bounds before mutating output, the MOV `hvcC` parser now uses the shared big-endian u48 reader for HEVC constraint flags instead of a local helper, and `avutil_byteio` fuzz coverage now build-checks 48-bit and 64-bit read/write paths. The ledger still keeps `avutil-byteio` at `implemented`, not `complete`, because deeper FFmpeg byte helper compatibility vectors, pinned-oracle differential tests, FATE coverage, and actual local fuzz execution are still absent.

`avutil-frame` now reports tightly packed per-plane line sizes for the current audio/video frame subset. Video frames derive line sizes for gray/rgb24/rgba/yuv420p from the validated pixel format and dimensions, audio frames expose packed per-plane byte counts from the validated sample format, and `avutil_core_models` now build-checks these line-size invariants. The ledger still keeps `avutil-frame` at `implemented`, not `complete`, because full AVFrame fields, buffer ownership, custom alignment/stride behavior, side data, pinned-oracle differential tests, FATE, and actual local fuzz execution are still absent.

`avutil-packet` now models more AVPacket-shaped metadata and mutation invariants. Packets expose payload length/emptiness, preserve unknown byte position separately from valid nonnegative positions, reject negative byte positions without mutating existing state, support clearable FFmpeg-style key/corrupt/discard/trusted/disposable flags with known-bit truncation, reject side-data kind names that are empty or contain NUL, and rescale valid PTS/DTS/duration fields between time bases while preserving unknown timestamps. The `avutil_core_models` fuzz target now build-checks packet byte-position, duration, timestamp rescaling, flag, side-data, payload, and non-mutation invariants. The ledger still keeps `avutil-packet` at `implemented`, not `complete`, because full AVPacket field parity, pinned-oracle differential tests, FATE, and actual local fuzz execution are still absent.

Current packet side-data lifecycle coverage adds first-match lookup, mutable lookup, checked shrink, oversize-shrink no-mutation behavior, first-match take/remove, duplicate-kind ordering preservation, and clear-all behavior.

Current packet payload lifecycle coverage stores payload bytes in `BufferRef`, shares payload storage across `ref_from`, keeps side-data records independently owned across refs, detaches shared payload storage for writable mutation, transfers all modeled fields through `move_ref_from`, and resets/releases current packet state through `unref`.

`avutil-timebase` now covers FFmpeg-style timestamp rescaling edge modes more explicitly. It adds an away-from-zero rounding mode, a pass-min/max helper for timestamp sentinels, tests invalid time bases and out-of-range rescale results, and extends `avutil_core_models` to fuzz-check all current rounding modes plus sentinel preservation against an independent i128 model. The ledger still keeps `avutil-timebase` at `implemented`, not `complete`, because pinned-oracle edge-case differential vectors are still absent.

`avutil-rational` now exposes checked add, subtract, negate, multiply, divide, reciprocal, and comparison helpers over normalized rationals. The new arithmetic paths reduce results and reject invalid or out-of-range results with typed errors, and `avutil_core_models` now fuzz-checks rational arithmetic and division round trips in addition to the existing timebase invariants. The ledger still keeps `avutil-rational` at `implemented`, not `complete`, because pinned-oracle differential vectors for the broader FFmpeg rational surface are still absent.

`avutil-error` now has explicit constructors for common error classes, a stable `std::io::ErrorKind` to `AvErrorKind` classifier, optional preservation of the originating IO error kind, and an `is_eof` predicate. `avformat` AVIO now uses this shared IO error conversion instead of maintaining a local mapper. The ledger still keeps `avutil-error` at `implemented`, not `complete`, because no component is complete without the full required parity proof, and this error model still has no FFmpeg oracle differential or FATE coverage.

`fftools` now resolves `-loglevel`/`-v` through the shared `avutil::LogLevel` and `LogFlags` models instead of treating those values as opaque strings. The ffmpeg-style option parser validates exact level names, FFmpeg numeric severities including `-8` for quiet, compound repeat/level/time/datetime flag directives, and flag-only +/- updates while preserving global option ordering and last-option-wins level resolution in `CliLogConfig`. `IoPlan` now carries the resolved global log configuration, and the ffprobe parser validates the same constrained loglevel surface. Real stderr emission parity remains incomplete.

`avutil-logging` now has a stronger flag-aware primitive. `LogLevel` exposes FFmpeg-style numeric severity values and level names; `LogFlags` stores and truncates the known public `av_log` flag bits; `LogTimestamp` carries Unix-microsecond timestamps for deterministic UTC time/datetime rendering and current system-clock capture; `LogRecord` has deterministic no-newline formatting with configurable print-level and timestamp prefixes plus current timestamp attachment; `LogColorMode` resolves forced color/no-color environment variables and terminal stderr detection; and `Logger` now carries configured flags in addition to enablement checks, acceptance returns, quiet suppression, clear/take buffer control, formatted record rendering, per-call callback dispatch, installed callback dispatch, process-global shared state helpers, and deterministic `AV_LOG_SKIP_REPEATED`-style consecutive duplicate compression with pending or materialized repeat summaries. The ledger still keeps `avutil-logging` at `implemented`, not `complete`, because byte-identical `av_log` formatting and color policy, C ABI callback parity, local-time formatting parity, CLI repeat/time/datetime/stderr integration, oracle differential tests, and FATE coverage remain incomplete.

`fate-runner` now has a tested explicit mapping format, prerequisite model, loader, mapping report command, and dry-run path. It parses component IDs from `PORTING_LEDGER.toml`, lists all configured mappings from `tests/fate/mappings.txt` independently of component selection, can audit all mapping prerequisites with `--check-prereqs`, maps git changed paths for the currently covered Rust modules and cargo-fuzz target files to ledger component IDs, preserves ledger order, reports unmapped implementation paths instead of silently ignoring them, includes untracked files in changed-path discovery, parses pipe-separated mappings from `tests/fate/mappings.txt`, expands `{samples}` and `{oracle_ffmpeg}` placeholders only for mappings that require them, validates the samples path as an existing directory and the oracle path as an existing file, can dry-run selected mappings without spawning commands, and executes only explicitly mapped commands when not in dry-run mode. The default mapping file contains `fate-runner|local-self-test`, which runs `cargo test -p fate-runner` and proves runner wiring without claiming upstream FFmpeg FATE media parity; local avutil primitive mappings; focused avcodec rawvideo and pcm_s16le decoder mappings; `avformat-mov-demuxer|local-mov-unit`, which runs `cargo test -p avformat mov::tests` for selected MOV work; local `fftools` version, hide-banner, option-parser, and I/O-plan smoke mappings; and shared ffmpeg/ffprobe unit-test selections. Upstream FATE samples and media component mappings remain absent.

`fuzz` now contains the first cargo-fuzz harness package, kept outside the main workspace. `avcodec_basic_decoders` fuzzes rawvideo and pcm_s16le decoder constructor validation, packet-size rejection, decoded frame shape, payload preservation, and PTS propagation. `avutil_byteio` fuzzes bounded byte reads, EOF cursor invariants, 48/64-bit integer paths, and byte writer helper paths. `avutil_bitreader` fuzzes unsigned and signed bit reads/peeks, skips, byte alignment, unsigned and signed bit writer validation, and cursor invariants. `avutil_metadata_options` fuzzes metadata dictionary key/value validation, case-sensitive and case-insensitive mutation, duplicate-key insertion, lookup/removal/clear operations, AVOption-like definition validation, option flag truncation/intersection, unit constant scoping, numeric range query invariants, read-only mutation rejection, child option-set registration, query filtering, parsed values, direct typed values, range/type/string checks, and failed-mutation invariants. `avutil_core_models` fuzzes typed error constructors and IO error classification, deterministic logging repeat compression plus timestamp-formatting invariants, rational arithmetic/timebase rounding and sentinel handling, packet timestamp/position/duration/flag/side-data invariants, pixel/sample/channel-layout validation, frame shape and tightly packed line-size validation, Adler-32/CRC-32/MD5/SHA-224/SHA-256/SHA-384/SHA-512 streaming equivalence, and digest hex invariants. `avformat_probe` fuzzes probe descriptor validation, generated registry mutation, AVI/MOV descriptors, extension/MIME/signature scoring, deterministic tie behavior, and explainable matches. `avformat_wav` fuzzes RIFF/WAVE PCM s16le demuxer opening, packet emission, and parsed stream invariants. `avformat_yuv4mpegpipe` fuzzes YUV4MPEG2 demuxer opening, frame packet emission, and parsed stream invariants. `avformat_pcm_s16le` fuzzes raw PCM s16le demuxer parameter validation, packet slicing, packet timing, and side-data invariants. `avformat_rawvideo` fuzzes rawvideo demuxer geometry/format/rate validation, frame slicing, packet timing, and side-data invariants. `avformat_avi` fuzzes constrained RIFF AVI demuxer chunk parsing, stream metadata, packet timing, and side-data invariants. `avformat_avi_muxer` fuzzes constrained RGB24 AVI muxer constructor validation, packet validation, header/render stability, word padding behavior, finish behavior, and demuxer round trips. `avformat_mov` fuzzes constrained MOV/MP4 box parsing, sample-table packet extraction, stream metadata, packet timing, and side-data invariants. `avformat_image2` fuzzes image2 pattern parsing, entry sequence validation, packet timing/path side data, muxer path generation, invalid packet handling, finish behavior, and mux-demux round trips. `avformat_basic_muxers` fuzzes WAV, raw PCM s16le, rawvideo, and yuv4mpegpipe muxer packet validation, state non-mutation on rejected packets, accounting, render/finish behavior, post-finish rejection, and matching-demuxer round trips. `avformat_packet_muxers` fuzzes null/hash/framecrc/framehash/framemd5/streamhash packet accounting, sparse stream stats, timestamp propagation, Adler-32/CRC-32/MD5/SHA-224/SHA-256/SHA-384/SHA-512 digest stability, per-packet CRC/hash and per-stream hash record fields, render/finish stability, and post-finish rejection. `fftools_option_parser` fuzzes bounded argv parsing, option arity invariants, valid loglevel directives, stream-specifier option names, file/global grouping, and parse/render/parse stability. The harness package builds and passes clippy when Cargo can resolve cached/downloaded `libfuzzer-sys`, but the `cargo fuzz` subcommand is not installed in this environment, so actual fuzz execution remains blocked locally.

`avformat-video-parameters` now provides a shared video stream-parameter helper for the current rawvideo/yuv4mpegpipe/AVI subset. It validates dimensions, u32 container dimensions, pixel format, derived frame byte size, whole-frame input byte counts, and exact packet payload lengths while preserving distinct error kinds for user-supplied parameters versus untrusted container fields. Rawvideo demuxer/muxer, yuv4mpegpipe demuxer/muxer, and the AVI RGB24 muxer now store or validate their video shape through this helper, while format-specific constraints such as AVI classic header limits and YUV4MPEG2 4:2:0 even dimensions remain local. MOV visual sample-entry integration is intentionally pending until a tested sample-entry FourCC/depth to `PixelFormat` mapping exists. The ledger records this helper as implemented but not complete because oracle differential tests, FATE, fuzz coverage, and generated pixel-format coverage are still pending.

`avformat-audio-parameters` now provides a shared audio stream-parameter helper for the current PCM/WAV subset. It validates sample rate, channel count, sample format, derived native or count-only `ChannelLayoutSpec`, packed sample-frame byte sizing, bits-per-sample reporting, and whole-sample-frame byte lengths while preserving distinct error kinds for user-supplied parameters versus untrusted container fields. Raw `pcm_s16le` demuxer/muxer and RIFF/WAVE s16le demuxer/muxer now store and validate their audio metadata through this helper. The ledger records this helper as implemented but not complete because richer audio codec-parameter parity, oracle differential tests, FATE, and fuzz coverage are still pending.

Raw PCM and WAV format paths now use the shared audio format primitives instead of duplicating local s16 byte math. `PcmS16leDemuxer`, `PcmS16leMuxer`, `WavDemuxer`, and `WavMuxer` expose `SampleFormat::S16`, derive mono/stereo `ChannelLayout` metadata where channel-count-only inputs are unambiguous, and compute sample-frame sizes, WAV block alignment, and WAV bit depth through `avutil::SampleFormat`. Focused PCM/WAV tests and the full workspace validation suite pass. The related ledger entries remain `implemented`, not `complete`, because pinned-oracle differential tests, FATE, and fuzz coverage are still absent.

`avutil-channel-layout` now provides an initial shared channel layout model. It covers common native channel positions plus `mono`, `stereo`, `quad`, `5.1`, `5.1(side)`, and `7.1` named layouts, and `AudioFrame` now carries an optional validated channel layout. Existing channel-count-only PCM decode paths derive mono/stereo layouts where safe and leave other counts unspecified rather than inventing a default. The ledger records the component as implemented but not complete because full `ffmpeg -layouts` coverage, custom/native order semantics, oracle differential tests, FATE, and fuzz coverage are still pending.

`avutil-pixel-format` and `avutil-sample-format` now provide shared initial FFmpeg-style format models for the already-supported raw media subset. `PixelFormat` covers `gray`/`gray8`, `rgb24`, `rgba`, and `yuv420p` naming, plane counts, frame-size math, yuv420p even-dimension validation, and plane splitting. `SampleFormat` covers packed `s16` naming, packed plane counts, sample-byte sizing, and payload-size calculation. `VideoFrame` and `AudioFrame` now validate exact plane counts and plane lengths against those shared models, the rawvideo decoder reuses the shared pixel-plane splitter, the pcm_s16le decoder emits shared `SampleFormat::S16` frames, and the rawvideo demuxer/muxer uses the shared pixel format type alias instead of its own duplicate enum. The ledger records both components as implemented but not complete because full FFmpeg descriptor coverage, oracle differential tests, FATE, and fuzz coverage are still pending.

`avformat-mov-demuxer` now has an initial packet extraction path. It validates ISOBMFF/MOV/MP4 box bounds, parses `ftyp`, `moov/mvhd`, `trak/tkhd`, `mdia/mdhd` language metadata, and `mdia/hdlr` handler type plus handler-name metadata, extracts common movie-level and track-level `udta/meta/ilst` metadata values from `data` atoms for UTF-8 and UTF-16 text fields, classic `gnre` genre indexes, one-byte integer/boolean metadata atoms, iTunes-style freeform `----` atoms, `covr` cover-art payloads, and track/disc number pairs, registers a hand-written MOV/MP4 `ftyp`/extension/MIME probe descriptor, records generic `stsd` codec parameters, parses version-0 AudioSampleEntry fields and child boxes for known audio sample-entry tags, parses VisualSampleEntry fields and child boxes for known video sample entries and raw entries with full visual payloads including structured `avcC` version/profile/level/NAL-length-size/SPS/PPS data, structured `hvcC` profile/timing/NAL-length-size/NAL-array data, `pasp` pixel aspect ratio, and `nclx`/`nclc`/`rICC`/`prof` `colr` color information, explicitly rejects fragmented `mvex`/`moof` layouts, edit-list `edts` boxes, multiple populated tracks, multiple `stsd` sample entries, sample description indexes other than 1, malformed AudioSampleEntry and VisualSampleEntry child boxes, malformed metadata atoms, malformed text encodings, malformed handler names, malformed integer/boolean metadata payloads, malformed freeform metadata payloads, malformed cover-art payloads, malformed `avcC`/`hvcC`/`pasp`/`colr` payloads, parses simple `stsd`, `stts`, `ctts`, `stsc`, `stsz`, `stss`, and `stco`/`co64` sample tables for one populated track, handles multi-chunk `stsc` entry transitions and multiple `mdat` ranges, and emits packets with PTS/DTS/duration, composition-offset PTS, sync-sample key flags, and MOV side data. `avformat-avi-probe` now has a hand-written descriptor for the offset-8 RIFF `AVI ` form tag, `.avi` extension, and common AVI MIME types. `ffprobe-rs` now has initial local MOV/MP4 and constrained AVI `-show_format`/`-show_streams`/`-show_packets` execution paths that register AVI and MOV descriptors in the shared Rust `ProbeRegistry`, can force those Rust demuxer paths with `-f avi`/`-f mov`/`-f mp4`, open `AviDemuxer` or `MovDemuxer`, and render default or JSON summaries with local input byte length as format `size`, zero `nb_programs` and `nb_stream_groups` counters for the current constrained demuxer model, MOV movie-level format tags, plus initial stream `codec_name`, `codec_long_name`, MOV `profile` and `level` from parsed `avcC`/`hvcC`, MOV `avcC` `is_avc` and `nal_length_size`, `codec_type` derived from MOV handler type where available, `codec_tag_string`, numeric `codec_tag`, parsed display dimensions plus constrained `coded_width`/`coded_height`, MOV audio `sample_rate`, `channels`, and `bits_per_sample` from parsed AudioSampleEntry fields, `field_order=unknown` for video streams because field-order metadata is not parsed yet, MOV `mdhd` `language` and `hdlr` `handler_name` stream tags, AVI `bits_per_raw_sample` from parsed BITMAPINFOHEADER bit counts, MOV raw VisualSampleEntry depth as `bits_per_raw_sample`, MOV non-empty sample-entry `extradata_size`, `time_base` from parsed MOV media timescale or AVI stream scale/rate, zero-origin `start_pts`/`start_time` for non-empty constrained streams, `r_frame_rate`, `avg_frame_rate`, `duration_ts`/`duration` from parsed MOV media duration or AVI stream length/time base, `nb_frames` from parsed MOV sample counts or AVI stream lengths, constrained `nb_read_frames` in default and JSON output when `-count_frames` is requested, `nb_read_packets` in default and JSON output when `-count_packets` is requested, and MOV visual sample-entry `sample_aspect_ratio`, `display_aspect_ratio`, `color_range`, `color_space`, `color_transfer`, and `color_primaries` fields where the Rust demuxer has enough metadata. `ffmpeg-rs` now has constrained local MOV/MP4, AVI, PCM s16le RIFF/WAVE, raw `pcm_s16le`, explicit `rawvideo`, `yuv4mpegpipe`, and explicit image2 single-file/numbered-sequence command execution paths for stdout `-f null -` and `-f framecrc -`, plus raw `pcm_s16le` packet-copy to local `-f s16le` and `-f wav` files, rawvideo packet-copy to local `-f rawvideo` files, raw yuv420p packet-copy to local `-f yuv4mpegpipe` files, raw rgb24 packet-copy to local `-f avi` files, and image2 packet-copy to local `-f image2` file or numbered-pattern outputs with image2 `-start_number` on input and output groups, using Rust demuxers and Rust muxers while rejecting unsupported inputs, outputs, muxers, missing raw/image stream parameters, malformed AVI headers, malformed raw PCM packet boundaries, malformed rawvideo frame boundaries, non-yuv420p yuv4mpegpipe file outputs, non-rgb24 AVI file outputs, malformed YUV4MPEG2 stream/frame boundaries, empty image2 payloads, non-contiguous image2 sequences, and file-output overwrites.

The `fftools_option_parser` fuzz target also now generates and round-trips output-scoped `-hash` options with a valid hash-output fixture, and accepts compound loglevel directives in its global-option invariant checks.

## Last Successful Commands

- Current `fate-runner` differential-mapping coverage slice:
  - `cargo test -p fate-runner --target-dir target-codex` (28 tests passed before rustfmt)
  - `cargo test -p fate-runner --target-dir target-fate-runner-diff-test` (28 tests passed after rustfmt)
  - `cargo run --target-dir target-fate-runner-diff-bin -p fate-runner -- mappings --mappings tests/differential/mappings.txt`
  - `cargo run --target-dir target-fate-runner-diff-bin -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-fate-runner-diff-bin -p fate-runner -- run --dry-run --mappings tests/differential/mappings.txt --oracle-ffmpeg Cargo.toml --component avutil-channel-layout`
  - `cargo clippy -p fate-runner --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` oracle-harness slice after adding the fate-runner regression test:
  - `cargo test -p fate-runner --target-dir target-codex` (26 tests passed)
  - `cargo test -p avutil --test channel_layout_oracle` (ignored test compiled, 1 ignored)
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo clippy -p fate-runner --all-targets -- -D warnings`
  - `cargo clippy -p avutil --all-targets -- -D warnings`

- Current `avutil-channel-layout` `ffmpeg -layouts` oracle-harness slice:
  - `cargo test -p avutil --test channel_layout_oracle` (ignored test compiled, 1 ignored)
  - `cargo run --target-dir target-codex -p fate-runner -- mappings --mappings tests/differential/mappings.txt`
  - `cargo test -p avutil channel_layout` (33 tests passed; ignored oracle integration test compiled)
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo test -p fate-runner`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy -p fate-runner --all-targets -- -D warnings`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` whitespace/full-consumption parser slice:
  - `cargo test -p avutil channel_layout` (33 tests passed)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` case-sensitive parser-string slice:
  - `cargo test -p avutil channel_layout` (33 tests passed)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` explicit ambisonic strtol-order parser slice:
  - `cargo test -p avutil channel_layout` (33 tests passed)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` byte-entry parser slice:
  - `cargo test -p avutil channel_layout` (31 tests passed)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` raw-channel-mask retype slice:
  - `cargo test -p avutil channel_layout` (30 tests passed)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` ambisonic/canonical-target retype slice:
  - `cargo test -p avutil channel_layout` (29 tests passed)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` lossy native/unspecified-target retype slice:
  - `cargo test -p avutil channel_layout` (27 tests passed)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` custom-target retype slice:
  - `cargo test -p avutil channel_layout` (23 tests passed through Cargo's default target directory after correcting the new test's expected custom-unknown description)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`

- Current `avutil-channel-layout` ambisonic retype slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'canonical_order|masked_description|av_channel_layout_ambisonic_order|AV_CHANNEL_ORDER_AMBISONIC' -Context 0,80` (source-check only)
  - `cargo test -p avutil channel_layout` (22 tests passed through Cargo's default target directory)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (CRLF warnings only)

- Current `avutil-channel-layout` explicit ambisonic lookup slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_layout_channel_from_index|av_channel_layout_index_from_channel|av_channel_layout_index_from_string|av_channel_layout_channel_from_string' -Context 0,90` (source-check only)
  - `cargo test -p avutil channel_layout` (22 tests passed through Cargo's default target directory)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`

- Current `avutil-channel-layout` escaped channel-list tokenizer slice:
  - `Select-String -Path $env:TEMP\ffmpeg-opt-8.1.1.c -Pattern 'get_key|av_opt_get_key_value' -Context 0,80` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-avstring-8.1.1.c -Pattern 'av_get_token' -Context 0,60` (source-check only)
  - `cargo test -p avutil channel_layout` (22 tests passed through Cargo's default target directory)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current `avutil-channel-layout` ChannelLayoutSpec custom variant slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'parse_channel_list|canonical_order|AV_CHANNEL_ORDER_CUSTOM' -Context 0,80` (source-check only)
  - `cargo test -p avutil channel_layout` (21 tests passed through Cargo's default target directory)
  - `cargo test -p avformat audio` (13 tests passed through Cargo's default target directory)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy -p avformat --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current `avutil-channel-layout` custom channel-list parser slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'parse_channel_list|av_opt_get_key_value|canonical_order' -Context 0,90` (source-check only)
  - `cargo test -p avutil channel_layout` (20 tests passed through Cargo's default target directory)
  - `cargo fmt --all`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current `avutil-channel-layout` native channel-list parser slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'parse_channel_list|av_channel_layout_from_string|av_channel_layout_retype' -Context 0,90` (source-check only)
  - `cargo test -p avutil channel_layout` (19 tests passed through Cargo's default target directory)
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current `avutil-channel-layout` numeric-mask parser slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_layout_from_string|av_channel_layout_from_mask|strtoull|parse_channel_list' -Context 0,90` (source-check only)
  - `cargo test -p avutil channel_layout` (18 tests passed through Cargo's default target directory)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-channel-layout` count-suffix parser slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_layout_from_string|strtol| channels|strcmp(end' -Context 0,80` (source-check only)
  - `cargo test -p avutil channel_layout` (18 tests passed through Cargo's default target directory)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-channel-layout` unspecified-layout slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.h -Pattern 'AV_CHANNEL_ORDER_UNSPEC|av_channel_layout_compare|av_channel_layout_check' -Context 0,12` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_layout_default|av_channel_layout_describe_bprint|av_channel_layout_check|av_channel_layout_compare|av_channel_layout_subset' -Context 0,80` (source-check only)
  - `cargo fmt --all -- --check`
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo test -p avutil audio_frame --target-dir target-codex`
  - `cargo test -p avformat audio` (rerun through the default target directory after Windows Application Control blocked the custom target-dir executable)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy -p avformat --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo run --target-dir target-codex -p fate-runner -- list`
  - `git diff --check` (passed with CRLF conversion warnings only)

- Current `avutil-channel-layout` default-count slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'channel_layout_map|av_channel_layout_default|AV_CHANNEL_LAYOUT_22POINT2|AV_CHANNEL_LAYOUT_HEXADECAGONAL' -Context 0,120` (source-check only)
  - `cargo fmt --all`
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- list`
  - `git diff --check`

- Current `avutil-channel-layout` lookup slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_layout_index_from_channel|av_channel_layout_channel_from_index|av_channel_layout_index_from_string|av_channel_layout_channel_from_string' -Context 6,35` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.h -Pattern 'av_channel_layout_index_from_channel|av_channel_layout_channel_from_index|av_channel_layout_index_from_string|av_channel_layout_channel_from_string' -Context 4,12` (source-check only)
  - `cargo fmt --all`
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- list`
  - `git diff --check`

- Current `avutil-channel-layout` subset-mask slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_layout_subset' -Context 0,25` (source-check only)
  - `cargo fmt --all`
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- list`
  - `git diff --check`

- Current `avutil-channel-layout` comparison slice:
  - `Get-Content $env:TEMP\ffmpeg-channel-layout-8.1.1.c | Select-String -Pattern 'av_channel_layout_compare|canonical_order|has_channel_names|av_channel_layout_check|av_channel_layout_subset' -Context 5,35` (source-check only)
  - `cargo fmt --all`
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `git diff --check`
  - `cargo run --target-dir target-codex -p fate-runner -- list`

- Current `avutil-channel-layout` custom ambisonic slice:
  - `Get-Content $env:TEMP\ffmpeg-channel-layout-8.1.1.c | Select-Object -Skip 484 -First 105` (source-check only)
  - `Get-Content $env:TEMP\ffmpeg-channel-layout-8.1.1.c | Select-Object -Skip 540 -First 100` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.h -Pattern 'ambisonic_order|AV_CHANNEL_LAYOUT_AMBISONIC_FIRST_ORDER|AV_CHAN_AMBISONIC|AV_CHANNEL_ORDER_AMBISONIC' -Context 3,8` (source-check only)
  - `cargo fmt --all`
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `git diff --check`
  - `cargo run --target-dir target-codex -p fate-runner -- list`

- Current `avutil-channel-layout` custom-map slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_layout_retype|try_describe_ambisonic|av_channel_layout_describe|av_channel_layout_compare|has_channel_names|canonical_order|AV_CHANNEL_ORDER_NATIVE|AV_CHANNEL_ORDER_CUSTOM|AV_CHANNEL_ORDER_AMBISONIC' -Context 4,12` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.h -Pattern 'AVChannelLayout|AV_CHANNEL_ORDER_NATIVE|AV_CHANNEL_ORDER_CUSTOM|AV_CHANNEL_ORDER_AMBISONIC|av_channel_layout_retype|AV_CHANNEL_LAYOUT_RETYPE' -Context 3,10` (source-check only)
  - `Get-Content $env:TEMP\ffmpeg-channel-layout-8.1.1.c | Select-Object -Skip 520 -First 160` (source-check only)
  - `Get-Content $env:TEMP\ffmpeg-channel-layout-8.1.1.c | Select-Object -Skip 456 -First 80` (source-check only)
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `git diff --check`

- Previous `avutil-channel-layout` custom-map slice:
  - `Get-Content $env:TEMP\ffmpeg-channel-layout-8.1.1.c | Select-Object -Skip 560 -First 95` (source-check only)
  - `Get-Content $env:TEMP\ffmpeg-channel-layout-8.1.1.c | Select-Object -Skip 650 -First 145` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.h -Pattern 'AVChannelCustom|AVChannelLayout|AV_CHANNEL_ORDER_CUSTOM|AV_CHANNEL_ORDER_AMBISONIC|AV_CHANNEL_LAYOUT_MASK|u\.map|nb_channels' -Context 3,8` (source-check only)
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `git diff --check`

- Previous `avutil-channel-layout` ChannelId slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.h -Pattern 'AVChannel|AV_CHAN_UNUSED|AV_CHAN_UNKNOWN|AV_CHAN_AMBISONIC|AV_CHANNEL_ORDER|av_channel_name|av_channel_from_string|av_channel_description' -Context 3,7` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'av_channel_name|av_channel_from_string|av_channel_description|AMBI|Unused|unknown|ambisonic|channel_names' -Context 4,10` (source-check only)
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex` (passed after re-exporting `ChannelId`)
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `git diff --check`

- Previous `avutil-channel-layout` slice:
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.h -Pattern 'SURROUND_DIRECT|SIDE_SURROUND|TOP_SURROUND|AMBISONIC|AV_CHAN_UNUSED|AV_CHAN_UNKNOWN|AV_CHAN_NB|AV_CH_SURROUND_DIRECT|AV_CH_SIDE_SURROUND|AV_CH_TOP_SURROUND' -Context 4,5` (source-check only)
  - `Select-String -Path $env:TEMP\ffmpeg-channel-layout-8.1.1.c -Pattern 'SURROUND_DIRECT|SIDE_SURROUND|TOP_SURROUND|AMBISONIC|"SDL"|"SDR"|"SSL"|"SSR"|"TTL"|"TTR"' -Context 4,5` (source-check only)
  - `cargo test -p avutil channel_layout --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-channel-layout`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo test -p avformat audio --target-dir target-codex`
  - `git diff --check`

- Current `avutil-sample-format` table-string slice:
  - `cargo test -p avutil samplefmt`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-sample-format`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `git diff --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`

- Current `avutil-sample-format` owned allocation slice:
  - `cargo fmt --all`
  - `cargo test -p avutil samplefmt`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-sample-format`
  - `cargo fmt --all -- --check`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-sample-format` fill-array layout slice:
  - `cargo fmt --all`
  - `cargo test -p avutil samplefmt`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings` (passed after removing needless explicit lifetimes)
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-sample-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-sample-format` copy-helper slice:
  - `cargo fmt --all`
  - `cargo test -p avutil samplefmt`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-sample-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-sample-format` silence-fill slice:
  - `cargo fmt --all`
  - `cargo test -p avutil samplefmt`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-sample-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-sample-format` sample-buffer-layout slice:
  - `cargo fmt --all`
  - `cargo test -p avutil samplefmt` (passed through the default target cache after `target-codex` was blocked by Windows Application Control)
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-sample-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/samplefmt.c -OutFile $env:TEMP\ffmpeg-samplefmt-8.1.1.c` (after escalation)

- Current `avutil-bitwriter` truncate/reset slice:
  - `cargo fmt --all`
  - `cargo test -p avutil bitwriter --target-dir target-codex`
  - `cargo test -p avutil bitreader --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-bitwriter`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-timebase` signed stable-add slice:
  - `cargo fmt --all`
  - `cargo test -p avutil timebase --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-timebase`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-timebase` rescale-delta slice:
  - `cargo fmt --all`
  - `cargo test -p avutil timebase --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-timebase`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-timebase` stable-add slice:
  - `cargo fmt --all`
  - `cargo test -p avutil timebase --target-dir target-codex`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-timebase`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-timebase` compare helper slice:
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/mathematics.h -OutFile $env:TEMP\ffmpeg-mathematics-8.1.1.h` (after escalation)
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/mathematics.c -OutFile $env:TEMP\ffmpeg-mathematics-8.1.1.c` (after escalation)
  - `cargo fmt --all`
  - `cargo test -p avutil timebase --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-timebase`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `git diff --check`

- Current `avutil-rational` int-float slice:
  - `cargo fmt --all`
  - `cargo test -p avutil rational --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-rational`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-rational` comparison/nearest slice:
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/rational.h -OutFile $env:TEMP\ffmpeg-rational-8.1.1.h`
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/rational.c -OutFile $env:TEMP\ffmpeg-rational-8.1.1.c`
  - `cargo fmt --all`
  - `cargo test -p avutil rational --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p avutil --target-dir target-codex -- -D warnings`
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-rational`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `avutil-error` string-table slice:
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/error.h -OutFile $env:TEMP\ffmpeg-error-8.1.1.h`
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/error.c -OutFile $env:TEMP\ffmpeg-error-8.1.1.c`
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo test -p avutil error --target-dir target-codex`
  - `cargo clippy -p avutil --target-dir target-codex -- -D warnings`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-error`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `fate-runner` oracle-env mapping slice:
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo test -p fate-runner --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- mappings --mappings tests/differential/mappings.txt --check-prereqs --oracle-ffmpeg Cargo.toml`
  - `cargo run --target-dir target-codex -p fate-runner -- run --dry-run --mappings tests/differential/mappings.txt --oracle-ffmpeg Cargo.toml --component fftools-ffmpeg-rawvideo-file-output --component avformat-rawvideo-demuxer --component avformat-rawvideo-muxer`
  - `cargo clippy -p fate-runner --target-dir target-codex -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `fate-runner` repeated-component slice:
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo test -p fate-runner parses_run_options --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --dry-run --component avformat-rawvideo-demuxer --component avformat-rawvideo-muxer --component avformat-rawvideo-demuxer`
  - `cargo test -p fate-runner --target-dir target-codex`
  - `cargo clippy -p fate-runner --target-dir target-codex -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current rawvideo oracle harness slice:
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo test -p fftools --test rawvideo_oracle --target-dir target-codex`
  - `cargo clippy -p fftools --test rawvideo_oracle --target-dir target-codex -- -D warnings`
  - `cargo test -p fate-runner rawvideo_oracle --target-dir target-codex`
  - `cargo test -p fate-runner --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo clippy -p fate-runner --target-dir target-codex -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current MSB-aligned planar YUV444/GBR `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec msb --target-dir target-codex`
  - `cargo test -p avcodec high_bit_depth_planar_yuv --target-dir target-codex`
  - `cargo test -p avformat msb --target-dir target-codex`
  - `cargo test -p avformat high_bit_depth_planar_yuv --target-dir target-codex`
  - `cargo test -p fftools --lib gbrp10msble --target-dir target-codex`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-frame --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer --dry-run`
  - `git diff --check`

- Current packed 32-bit integer RGB/RGBA `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avformat rgb96 --target-dir target-codex`
  - `cargo test -p avformat muxer_computes_rgbf_frame_sizes --target-dir target-codex`
  - `cargo test -p avformat muxer_computes_rgbaf_frame_sizes --target-dir target-codex`
  - `cargo test -p fftools --lib runs_rawvideo_rgba128le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo check -p avcodec --tests --target-dir target-codex`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-frame`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `git diff --check`

- Current packed floating RGBA `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rgba_float --target-dir target-codex`
  - `cargo test -p avformat rgbaf --target-dir target-codex`
  - `cargo test -p fftools --lib runs_rawvideo_rgbaf32le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-frame`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `git diff --check`

- Current packed floating RGB `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rgb_float --target-dir target-codex`
  - `cargo test -p avformat rgbf --target-dir target-codex`
  - `cargo test -p fftools --lib runs_rawvideo_rgbf32le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `git diff --check`

- Current planar floating GBR `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec gbrpf --target-dir target-codex`
  - `cargo test -p avformat gbrpf --target-dir target-codex`
  - `cargo test -p fftools --lib runs_rawvideo_gbrpf32le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `git diff --check`

- Current packed UYYVYY411 `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-avutil-byteio-peek-test`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-avutil-logging-color-test`
  - `cargo test -p avcodec uyyvyy411 --target-dir target-avcodec-ya16-test`
  - `cargo test -p avformat uyyvyy411 --target-dir target-avcodec-gray32-test`
  - `cargo test -p avformat computes_frame_sizes_for_supported_pixel_formats --target-dir target-avcodec-gray32-test`
  - `cargo test -p avformat rejects_invalid_geometry_frame_rate_and_truncated_frames --target-dir target-avcodec-rgba64-test`
  - `cargo test -p avformat muxer_rejects_invalid_geometry_rate_stream_and_frame_size --target-dir target-avcodec-ya16-test`
  - `cargo test -p fftools --lib runs_rawvideo_uyyvyy411_to_null_stdout --target-dir target-fftools-ya16-test`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-avcodec-gray32-test -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `git diff --check` (passed with CRLF warnings only)

- Current Bayer CFA `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil --target-dir target-codex`
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-avutil-byteio-peek-test`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-avutil-byteio-peek-test`
  - `cargo test -p avcodec decodes_bayer_packets_to_single_plane_frames --target-dir target-codex`
  - `cargo test -p avformat bayer --target-dir target-avcodec-rgba64-test`
  - `cargo test -p fftools --lib runs_rawvideo_bayer_bggr8_to_null_stdout --target-dir target-fftools-rgba64-test`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `git diff --check` (passed with CRLF warnings only)

- Current packed X/V YUV 4:4:4 `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-avutil-byteio-peek-test`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-avutil-byteio-peek-test`
  - `cargo test -p avcodec decodes_xv_packed_yuv_packets_to_single_plane_frames --target-dir target-codex`
  - `cargo test -p avformat xv --target-dir target-avcodec-rgba64-test`
  - `cargo test -p fftools --lib runs_rawvideo_xv30le_to_null_stdout --target-dir target-fftools-rgba64-test`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `git diff --check` (passed with CRLF warnings only)

- Current packed floating gray+alpha YAF `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec decodes_yaf_packets_to_single_plane_frames --target-dir target-codex`
  - `cargo test -p avformat yaf --target-dir target-codex`
  - `cargo test -p avformat computes_frame_sizes_for_supported_pixel_formats --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_yaf16le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current packed 8-bit YUV/YUVA `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_vuya_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current packed X2RGB10/X2BGR10 `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_x2rgb10le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current packed XYZ12 `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_xyz12le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current packed AYUV64 `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_ayuv64le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current high-bit packed YUV 4:2:2 `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_y210le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current semi-planar NV YUV 4:2:2/4:4:4 `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_nv20le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current high-bit-depth planar YUVA `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo fmt --all`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_yuva420p10le_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `yuva420p` / `yuva422p` / `yuva444p` planar YUVA `avutil-pixel-format` / rawvideo slice:
  - `cargo check -p avutil -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `cargo test -p avutil pixel --target-dir target-codex`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes --target-dir target-codex`
  - `cargo test -p avcodec rawvideo --target-dir target-codex`
  - `cargo test -p avformat rawvideo --target-dir target-codex`
  - `cargo test -p fftools runs_rawvideo_yuva420p_to_null_stdout --target-dir target-codex`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo test --workspace --lib --all-features --target-dir target-codex`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `git diff --check`

- Current `yuv440p10*` / `yuv440p12*` planar YUV 4:4:0 `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo check -p avutil --target-dir target-codex`
  - `cargo check -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `$env:CARGO_TARGET_DIR='target-avutil-byteio-peek-test'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-avutil-byteio-peek-test'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test -p fftools runs_rawvideo_yuv440p10le_to_null_stdout`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test --workspace --lib --all-features`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-avutil-byteio-peek-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --changed`

- Current 14-bit and 16-bit planar YUV `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `cargo fmt --all -- --check`
  - `cargo check -p avutil --target-dir target-codex`
  - `cargo check -p avcodec -p avformat -p fftools --target-dir target-codex`
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex`
  - `$env:CARGO_TARGET_DIR='target-avutil-byteio-peek-test'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-avutil-byteio-peek-test'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test -p fftools runs_rawvideo_yuv420p14le_to_null_stdout`
  - `cargo clippy --workspace --all-targets --all-features --target-dir target-codex -- -D warnings`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --target-dir target-codex --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run`
  - `$env:CARGO_TARGET_DIR='target-avutil-byteio-peek-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test --workspace --lib --all-features`
  - `git diff --check` (CRLF warnings only)

- Current paletted `pal8` `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `cargo test --target-dir target-codex -p avutil pixel --no-run`
  - `cargo check --target-dir target-codex -p avutil -p avcodec -p avformat -p fftools --all-targets`
  - `cargo check --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo clippy --target-dir target-codex -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo clippy --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `cargo test --target-dir target-codex -p avutil frames_report_tightly_packed_line_sizes --no-run`
  - `cargo test --target-dir target-codex -p avcodec rawvideo --no-run`
  - `cargo test --target-dir target-codex -p avformat rawvideo --no-run`
  - `cargo test --target-dir target-codex -p fftools --lib runs_rawvideo_pal8_to_null_stdout --no-run`
  - `cargo test --target-dir target-avutil-byteio-peek-test -p avutil pixel`
  - `cargo test --target-dir target-avutil-byteio-peek-test -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test --target-dir target-avcodec-rgba64-test -p avcodec rawvideo`
  - `cargo test --target-dir target-avcodec-ya16-test -p avcodec rawvideo`
  - `cargo test --target-dir target-fftools-gray32-test -p avformat rawvideo`
  - `cargo test --target-dir target-fftools-gray32-test -p fftools --lib runs_rawvideo_pal8_to_null_stdout`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo fmt --all -- --check`
  - `git diff --check` (CRLF warnings only)
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo clippy --target-dir target-codex --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --target-dir target-fftools-gray32-test --workspace --lib --all-features`

- Current deprecated full-range planar YUVJ `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `cargo test --target-dir target-codex -p avutil pixel`
  - `cargo test --target-dir target-codex -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test --target-dir target-codex -p avcodec rawvideo`
  - `cargo test --target-dir target-codex -p avformat rawvideo`
  - `cargo test --target-dir target-codex -p fftools --lib runs_rawvideo_yuvj420p_to_null_stdout`
  - `cargo check --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo clippy --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `cargo clippy --target-dir target-codex -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo clippy --target-dir target-codex --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --target-dir target-codex --workspace --lib --all-features`
  - `git diff --check` (CRLF warnings only)

- Current 1bpp monochrome bitstream `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `cargo test --target-dir target-codex -p avutil pixel`
  - `cargo test --target-dir target-codex -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test --target-dir target-codex -p avcodec rawvideo`
  - `cargo test --target-dir target-codex -p avformat rawvideo`
  - `cargo test --target-dir target-codex -p fftools --lib runs_rawvideo_monow_to_null_stdout`
  - `cargo check --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `cargo clippy --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `cargo clippy --target-dir target-codex -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo clippy --target-dir target-codex --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check`

- Current packed 4bpp RGB bitstream `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `cargo test --target-dir target-codex -p avutil pixel`
  - `cargo test --target-dir target-codex -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test --target-dir target-codex -p avcodec rawvideo`
  - `cargo test --target-dir target-codex -p avformat rawvideo`
  - `cargo test --target-dir target-codex -p fftools --lib runs_rawvideo_rgb4_to_null_stdout`
  - `cargo check --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `cargo clippy --target-dir target-codex --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `cargo clippy --target-dir target-codex -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed`
  - `cargo clippy --target-dir target-codex --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check`

- Current byte-packed low-bit-depth RGB `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_rgb8_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`

- Current semi-planar YUV 4:2:0 `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `cargo test -p avutil pixel`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test -p avcodec rawvideo`
  - `cargo test -p avformat rawvideo`
  - `cargo test -p fftools --lib runs_rawvideo_nv12_to_null_stdout`
  - `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo run -p fate-runner -- run --changed`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check`

- Current packed YUV 4:2:2 `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_yuyv422_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check` (passed with CRLF warnings only)

- Current packed 16-bit RGB/BGR `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_rgb565le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check` (passed with CRLF warnings only)

- Current high-bit-depth grayscale `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_gray10le_alias_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
  - `git diff --check`

- Current planar GBRA `avutil-pixel-format` / rawvideo slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_gbrapf32le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
  - `git diff --check`

- Current `gbrp16le`/`gbrp16be` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_gbrp16le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current `gbrp14le`/`gbrp14be` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `cargo test -p fftools --lib runs_rawvideo_gbrp14le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
  - `git diff --check` (passed with CRLF warnings only)

- Current `gbrp12le`/`gbrp12be` slice:
  - `cargo fmt --all`
  - `cargo test -p avutil pixel`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test -p avcodec rawvideo`
  - `cargo test -p avformat rawvideo`
  - `cargo test -p fftools --lib runs_rawvideo_gbrp12le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check` (passed with CRLF warnings only)
  - `cargo run -p fate-runner -- run --changed`

- Current `gbrp10le`/`gbrp10be` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel --no-run`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil -p avcodec -p avformat -p fftools --lib --no-run`
  - `cargo test -p avutil pixel`
  - `cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test -p avcodec rawvideo`
  - `cargo test -p avformat rawvideo`
  - `cargo test -p fftools --lib runs_rawvideo_gbrp10le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check` (clean apart from existing CRLF normalization warnings)
  - `cargo run -p fate-runner -- run --changed`

- Current `gbrp9le`/`gbrp9be` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_gbrp9le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`

- Current `gbrp` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `cargo test -p avcodec rawvideo`
  - `cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_gbrp_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `cargo run -p fate-runner -- run --component avcodec-rawvideo`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check`

- Current `grayf16le`/`grayf16be`/`grayf32le`/`grayf32be` slice:
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/pixdesc.c -OutFile C:\tmp\ffmpeg-pixdesc-8.1.1.c` (approved network retry)
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/pixfmt.h -OutFile C:\tmp\ffmpeg-pixfmt-8.1.1.h`
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-avcodec-gray32-test'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test -p fftools --lib runs_rawvideo_grayf32le_to_null_stdout`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; .\target-codex\debug\fate-runner.exe run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; .\target-codex\debug\fate-runner.exe run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-codex'; .\target-codex\debug\fate-runner.exe run --changed`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features`
  - `git diff --check` (CRLF warnings only)

- Current `gray32le`/`gray32be` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-avcodec-gray32-test'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; cargo test -p fftools --lib runs_rawvideo_gray32le_to_null_stdout`
  - `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-avcodec-gray32-test'; .\target-codex\debug\fate-runner.exe run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; .\target-codex\debug\fate-runner.exe run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-fftools-gray32-test'; .\target-codex\debug\fate-runner.exe run --changed`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check`

- Current `ya16le`/`ya16be` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-avcodec-ya16-test'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-ya16-test'; cargo test -p fftools --lib runs_rawvideo_ya16le_to_null_stdout`
  - `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-avcodec-ya16-test'; .\target-codex\debug\fate-runner.exe run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-ya16-test'; .\target-codex\debug\fate-runner.exe run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-fftools-ya16-test'; .\target-codex\debug\fate-runner.exe run --changed`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `git diff --check` (passed; CRLF warnings only)

- Current `rgba64le`/`rgba64be`/`bgra64le`/`bgra64be` slice:
  - `cargo fmt --all`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
  - `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; cargo test -p avcodec rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
  - `$env:CARGO_TARGET_DIR='target-fftools-rgba64-test'; cargo test -p fftools --lib runs_rawvideo_rgba64le_to_null_stdout`
  - `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
  - `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
  - `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; .\target-codex\debug\fate-runner.exe run --component avcodec-rawvideo`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
  - `$env:CARGO_TARGET_DIR='target-fftools-rgba64-test'; .\target-codex\debug\fate-runner.exe run --component fftools-ffmpeg-rawvideo-framecrc-null`
  - `$env:CARGO_TARGET_DIR='target-fftools-rgba64-test'; .\target-codex\debug\fate-runner.exe run --changed`
  - `cargo fmt --all -- --check`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p swscale --lib`
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avdevice --lib`
  - `git diff --check`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_ya8_to_null_stdout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`
- `git diff --check` (passed with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_rgb48le_to_null_stdout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`
- `git diff --check` (passed with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_gray16le_to_null_stdout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`
- `git diff --check` (passed with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_yuv411p_to_null_stdout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
- `cargo fmt --all -- --check`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`
- `git diff --check` (passed with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_yuv444p_to_null_stdout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
- `cargo fmt --all -- --check`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`
- `git diff --check` (passed with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frames_report_tightly_packed_line_sizes`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_yuv422p_to_null_stdout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers`
- `cargo fmt --all -- --check`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avcodec_basic_decoders --bin avformat_rawvideo --bin avformat_basic_muxers -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-demuxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avformat-rawvideo-muxer`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`
- `git diff --check` (passed with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `git diff --check` (passed with CRLF warnings only)
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil pixel`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avcodec rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avformat rawvideo`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_bgra_to_null_stdout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avformat_rawvideo --bin avformat_basic_muxers --bin avcodec_basic_decoders`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models --bin avformat_rawvideo --bin avformat_basic_muxers --bin avcodec_basic_decoders -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil -p avcodec -p avformat -p fftools --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check` (passed with CRLF warnings only)
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-pixel-format`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib`
- `cargo fmt --all -- --check` after state/docs/mapping updates
- `git diff --check` after state/docs/mapping updates (passed with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil channel_layout`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component avutil-channel-layout`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `git diff --check` (passed with CRLF warnings only)
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil dict`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_metadata_options`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_metadata_options -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check` (exited 0 with CRLF warnings only)
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component avutil-dict`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil options`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_metadata_options`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_metadata_options -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check` (exited 0 with CRLF warnings only)
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component avutil-options`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil bitreader`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil bitwriter`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_bitreader`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_bitreader -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component avutil-bitreader`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component avutil-bitwriter`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`
- `git diff --check`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil byteio`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_byteio`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_byteio -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component avutil-byteio`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`
- `git diff --check`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avutil hash`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avformat hash_muxer`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib runs_s16le_to_hash_stdout_with_sha160_option`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib ffmpeg`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p avutil -p avformat -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo check --manifest-path fuzz\Cargo.toml --bins`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --manifest-path fuzz\Cargo.toml --bins -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component avutil-hash`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib --no-run`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`
- `git diff --check`

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib cli_logging`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p fftools --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component fftools-option-parser`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib --no-run`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`
- `git diff --check` (exited 0 with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib cli_logging`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component fftools-option-parser`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib --no-run`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`
- `git diff --check` (exited 0 with CRLF warnings only)

- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-terminal-test'; cargo test -p avutil logging` (passed before the clippy branch cleanup; the rerun in this target was blocked by Windows Application Control after rebuild)
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test -p avutil logging`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-terminal-test'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-terminal-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-terminal-test'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-logging`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-terminal-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test --workspace --all-features --lib --no-run`
- `git diff --check` (exited 0 with CRLF warnings only)
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib cli_logging`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p fftools --lib`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --component fftools-option-parser`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test --workspace --all-features --lib`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-env-test'; cargo test -p avutil logging` (passed when rerun outside sandbox after Application Control blocked the first sandboxed launch)
- `$env:CARGO_TARGET_DIR='target-avutil-logging-env-test'; cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-env-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-logging`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-env-test'; cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `git diff --check` (exited 0 with CRLF warnings only)
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test -p fftools --lib cli_logging`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test -p fftools --lib`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo clippy -p fftools --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo run -p fate-runner -- run --component fftools-option-parser`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test --workspace --all-features --lib`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test -p fftools --lib cli_logging`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test -p fate-runner`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test -p fftools --lib`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo clippy -p fftools -p fate-runner --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo run -p fate-runner -- run --component fftools-option-parser`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test --workspace --all-features --lib`
- `git diff --check`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-color-test'; cargo test -p avutil logging`
- `cargo fmt --all -- --check`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-color-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-color-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-color-test'; cargo run -p fate-runner -- run --component avutil-logging`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-color-test'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo test -p avutil logging`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --component avutil-logging`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo test -p avutil logging`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --component avutil-logging`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo test -p avutil logging`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --component avutil-logging`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo test -p avutil logging`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --component avutil-logging`
- `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-options-rational-test'; cargo test -p avutil options`
- `$env:CARGO_TARGET_DIR='target-avutil-options-rational-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_metadata_options`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_metadata_options -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-options-rational-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-options-rational-test'; cargo run -p fate-runner -- run --component avutil-options`
- `$env:CARGO_TARGET_DIR='target-avutil-options-rational-test'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test -p avutil timebase`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --component avutil-timebase`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test -p avutil rational`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --component avutil-rational`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test -p avutil error`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --component avutil-error`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test -p avutil packet_opaque`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --changed`
- `git diff --check` (passed with CRLF warnings only)
- `cargo fmt --all`
- `cargo test --target-dir target-avutil-timebase-test -p avutil iamf -- --nocapture`
- `cargo test --target-dir target-avutil-timebase-test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `cargo test --target-dir target-avutil-timebase-test -p fftools --lib`
- `cargo test --target-dir target-avutil-timebase-test -p fftools --bins --no-run`
- `git diff --check` (passed with CRLF warnings only)
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick`
- `cargo fmt --all`
- `cargo test --target-dir target-avutil-timebase-test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `.\target\debug\fate-runner.exe run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `cargo test --target-dir target-avutil-timebase-test -p fftools --lib`
- `cargo test --target-dir target-avutil-timebase-test -p fftools --bins --no-run`
- `git diff --check`
- `cargo fmt --all`
- `cargo test --target-dir target-avutil-timebase-test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `cargo test --target-dir target-avutil-timebase-test -p fftools --lib --no-run`
- `git diff --check`
- `cargo fmt --all`
- `cargo test --target-dir target-avutil-timebase-test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick`
- `git diff --check`
- `cargo fmt --all`
- `cargo test --target-dir target-avutil-timebase-test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick`
- `git diff --check`
- `cargo fmt --all`
- `cargo test --target-dir target-avutil-timebase-test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick`
- `git diff --check`
- `cargo fmt --all`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo test -p avutil packet` (passed 65 focused packet tests before the later test-only clippy cleanup rebuilt the harness and WAC began blocking launches)
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-packet --dry-run`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo test -p avutil packet --no-run`
- `cargo test --target-dir target-avutil-opaque-ref-test -p avutil packet`
- `cargo test --target-dir target-avutil-timebase-test -p avutil packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\fate-runner.exe run --changed`
- `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick`
- `git diff --check`
- `cargo fmt --all`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil packet`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-packet --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p xtask -- quick`
- `cargo fmt --all`
- `cargo test -p avutil packet`
- `cargo test -p avutil packet --no-run`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-packet --dry-run`
- `cargo run -p fate-runner -- run --component avutil-packet`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo run -p xtask -- quick`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil packet`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil packet`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-packet`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-codex'; .\target\debug\xtask.exe quick`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil packet`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-packet`
- `cargo fmt --all`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil packet`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-packet`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `$env:CARGO_TARGET_DIR='target-codex'; .\target\debug\xtask.exe quick`
- `cargo test -p avutil packet --no-run`
- `cargo clippy -p avutil --all-targets -- -D warnings`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-packet --dry-run`
- `cargo fmt --all`
- `cargo test -p avutil packet`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-packet`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p xtask -- quick`
- `cargo fmt --all`
- `cargo test -p avutil byteio`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_byteio`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_byteio -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-byteio`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p xtask -- quick`
- `cargo fmt --all`
- `cargo test -p avutil bitreader`
- `cargo test -p avutil bitwriter`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_bitreader`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_bitreader -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-bitreader`
- `cargo run -p fate-runner -- run --component avutil-bitwriter`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p xtask -- quick`
- `git diff --check`
- `cargo test -p xtask lockfile_guard_rejects_forbidden_transitive_packages`
- `cargo test -p fate-runner changed_selection_maps_dependency_manifests_to_runtime_guard`
- `cargo test -p fate-runner`
- `cargo test -p xtask`
- `cargo run -p xtask -- guard-runtime`
- `cargo fmt --all -- --check`
- `cargo clippy -p xtask -p fate-runner --all-targets -- -D warnings`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo run -p xtask -- quick`
- `git diff --check`
- `cargo test -p xtask`
- `cargo run -p xtask -- guard-runtime`
- `cargo test -p fate-runner default_mappings_cover_runtime_guard_selection`
- `cargo clippy -p xtask -p fate-runner --all-targets -- -D warnings`
- `cargo run -p fate-runner -- run --component repo-runtime-guard`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `cargo run -p xtask -- quick`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil frame_side_data_parses_view_id_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_view_id_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_video_hint_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_video_hint_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_ambient_viewing_environment_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_ambient_viewing_environment_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_detection_bboxes_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_detection_bboxes_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_film_grain_params_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_film_grain_params_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_video_enc_params_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_video_enc_params_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_regions_of_interest_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_regions_of_interest_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_dynamic_hdr_plus_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_dynamic_hdr_plus_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_s12m_timecode_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_s12m_timecode_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_icc_profile_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_icc_profile_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_content_light_metadata_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_spherical_mapping_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_video_hint_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_alignment_rejects_zero_alignment_and_overflow`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil video_frame_make_writable_detaches_readonly_plane_storage`
- `cargo test -p avutil audio_frame_make_writable_detaches_readonly_plane_storage`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_sei_unregistered_payload`
- `cargo test -p avutil frame_side_data_rejects_malformed_sei_unregistered_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_rejects_malformed_exif_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_parses_exif_linked_ifds`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_decodes_exif_entry_values`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_common_exif_tags`
- `cargo test -p avutil frame_side_data_interprets_exif_gps_processing_error_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_sensitivity_tags`
- `cargo test -p avutil frame_side_data_interprets_exif_capture_setting_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_gps_processing_error_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_gps_destination_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_gps_motion_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_gps_acquisition_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_gps_altitude_time_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_root_bits_per_sample_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_root_document_page_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_root_subfile_type_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_root_colorimetry_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_root_image_layout_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_rendering_scene_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_capture_setting_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_offset_time_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_camera_characterization_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_sensitivity_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_exposure_tags`
- `cargo test -p avutil frame_side_data_interprets_exif_apex_exposure_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame_side_data_interprets_exif_interoperability_related_image_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo test -p avutil frame_side_data_rejects_exif_descriptive_ascii_shapes`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo run -p fate-runner -- run --component avutil-frame`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo run -p fate-runner -- run --changed --dry-run`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo test -p avutil frame_side_data_interprets_exif_root_orientation_resolution_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo run -p fate-runner -- run --component avutil-frame`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo run -p fate-runner -- run --changed --dry-run`
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_camera_identity_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_copyright_tag`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_predictor_tag`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_host_computer_tag`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_document_page_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_subfile_type_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_bits_per_sample_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_data_mut_detaches_shared_readonly_payload`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_version_timing_comment_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_common_exif_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_exposure_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_altitude_time_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_offset_time_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_descriptive_tags`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_common_exif_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_version_timing_comment_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame_side_data_interprets_exif_offset_time_tags`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_camera_characterization_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_sensitivity_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gamma_composite_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_camera_lens_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_version_timing_comment_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_optics_subject_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_rendering_scene_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_capture_setting_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_acquisition_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_destination_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_motion_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_altitude_time_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo fmt --all -- --check`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo test -p avutil --lib --no-run`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo test -p avutil --lib --no-run`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo test -p avutil --lib --no-run`
- `cargo test -p avutil frame`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame --dry-run`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `git diff --check`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib --no-run`
- `cargo run -p fate-runner -- run --component avutil-frame --dry-run`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo fmt --all`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-buffer`
- `git diff --check`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-buffer`
- `git diff --check`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --component avutil-buffer`
- `git diff --check`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-buffer`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `cargo fmt --all -- --check`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `git diff --check`
- `cargo run -p fate-runner -- run --component avutil-buffer`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo test -p fate-runner default_mappings_cover_current_avutil_smoke_selections`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `git diff --check`
- `cargo run -p fate-runner -- run --component avutil-buffer`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo test -p avutil buffer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo test -p fate-runner default_mappings_cover_current_avutil_smoke_selections`
- `cargo run -p fate-runner -- run --component avutil-buffer`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avformat_mov -- -D warnings`
- `cargo test -p avformat mov::tests`
- `cargo run -p fate-runner -- run --component avformat-mov-demuxer`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avformat_mov -- -D warnings`
- `cargo test -p avformat mov::tests`
- `cargo run -p fate-runner -- run --component avformat-mov-demuxer`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- run --component avformat-mov-demuxer`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all -- --check`
- `cargo test -p avformat mov::tests`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo run -p fate-runner -- run --component avformat-mov-demuxer`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo run -p fate-runner -- run --component avformat-mov-demuxer`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo check -p fftools`
- `git diff --check`
- `cargo test --workspace --all-features --exclude fftools --no-run`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo test --workspace --all-features`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- run --component avformat-mov-demuxer`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo check -p fftools`
- `cargo test -p avformat --lib --no-run`
- `cargo test --workspace --all-features --exclude fftools --no-run`
- `git diff --check`
- `git diff --check`
- `cargo fmt --all -- --check`
- `cargo test -p fate-runner`
- `cargo run -p fate-runner -- mappings`
- `cargo run -p fate-runner -- run --component avformat-mov-demuxer`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo test -p avformat mov::tests`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo test -p fftools --lib`
- `cargo run -p fate-runner -- list`
- `cargo run -p fate-runner -- run --samples . --oracle-ffmpeg Cargo.toml --component fate-runner`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo test -p avformat mov::tests`
- `cargo test -p fftools --lib ffmpeg::tests::streamhash_type_maps_derive_from_container_metadata`
- `cargo test -p fftools --lib ffprobe`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo test -p fftools --lib`
- `cargo run -p fate-runner -- list`
- `cargo run -p fate-runner -- run --samples . --oracle-ffmpeg Cargo.toml --component fate-runner`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test -p fftools --lib ffprobe`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo test -p fftools --lib`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test -p fftools --lib ffprobe`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avformat_mov`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `cargo test -p fftools --lib`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `cargo check --manifest-path fuzz\Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bins -- -D warnings`
- `cargo fmt --all --manifest-path fuzz\Cargo.toml -- --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `cargo check --manifest-path fuzz\Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bins -- -D warnings`
- `cargo fmt --all --manifest-path fuzz\Cargo.toml -- --check`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz\Cargo.toml -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p fftools ffmpeg`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo check --manifest-path fuzz\Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bins -- -D warnings`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz\Cargo.toml`
- `cargo test -p avformat streamhash_muxer`
- `cargo test -p fftools ffmpeg`
- `cargo check --manifest-path fuzz\Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz\Cargo.toml -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat framehash_muxer`
- `cargo test -p fftools ffmpeg`
- `cargo check --manifest-path fuzz\Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bins -- -D warnings`
- `cargo test -p fftools ffmpeg`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `cargo test -p fftools --lib option_parser`
- `cargo test -p fftools ffmpeg`
- `cargo test -p avformat hash_muxer`
- `cargo check --manifest-path fuzz\Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz\Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil hash`
- `cargo test -p avformat hash_muxer`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test -p avformat --lib`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avformat hash_muxer`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avformat --lib`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil hash`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil options`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo test -p avutil bitreader`
- `cargo test -p avutil bitwriter`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo test -p avutil byteio`
- `cargo test -p avformat mov::tests::parses_hevc_sample_entry_codec_parameters`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil logging`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo test -p avutil packet`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil timebase`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil rational`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil --lib`
- `cargo test --workspace --all-features --exclude fftools`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil error`
- `cargo test -p avformat avio`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools loglevel`
- `cargo test -p fftools log_level`
- `cargo test -p fftools`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p fftools`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil logging`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avutil`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avformat avi::tests`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avformat wav`
- `cargo test -p avformat pcm::tests`
- `cargo test -p avformat rawvideo`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avcodec rawvideo`
- `cargo test -p avcodec pcm`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avformat null_muxer`
- `cargo test -p avformat hash_muxer`
- `cargo test -p avformat framecrc_muxer`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avformat probe`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avutil dict`
- `cargo test -p avutil options`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avformat image2`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p fftools option_parser`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo test -p avformat mov::tests`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avformat avi`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avformat pcm::tests`
- `cargo test -p avformat rawvideo`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo test -p avformat wav`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo test -p avformat video`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo fmt --all --manifest-path fuzz/Cargo.toml`
- `cargo check --manifest-path fuzz/Cargo.toml --bins`
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fate-runner`
- `cargo run -p fate-runner -- run --dry-run --component fate-runner`
- `cargo run -p fate-runner -- run --dry-run --samples . --oracle-ffmpeg Cargo.toml --component fate-runner`
- `cargo run -p fate-runner -- run --dry-run --changed`
- `cargo run -p fate-runner -- run --samples . --oracle-ffmpeg Cargo.toml --component fate-runner`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fate-runner`
- `cargo run -p fate-runner -- mappings`
- `cargo run -p fate-runner -- mappings --check-prereqs --samples . --oracle-ffmpeg Cargo.toml`
- `cargo run -p fate-runner -- run --dry-run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat video`
- `cargo test -p avformat rawvideo`
- `cargo test -p avformat avi::tests::muxer`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat audio`
- `cargo test -p avformat pcm::tests`
- `cargo test -p avformat wav`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat pcm::tests`
- `cargo test -p avformat wav`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil channel_layout`
- `cargo test -p avutil frame`
- `cargo test -p avcodec pcm`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avutil pixel`
- `cargo test -p avutil samplefmt`
- `cargo test -p avutil frame`
- `cargo test -p avcodec rawvideo`
- `cargo test -p avcodec pcm`
- `cargo test -p avformat rawvideo`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests::rejects_invalid_full_boxes_and_ftyp_payloads`
- `cargo test -p avformat mov::tests`
- `cargo test -p fftools ffprobe`
- `cargo test -p fftools ffmpeg::tests::runs_mov_to`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat avi::tests`
- `cargo test -p avformat probe`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `git init`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo run -p fate-runner -- list`
- `cargo test -p avformat pcm`
- `cargo fmt --all`
- `cargo test -p avformat pcm::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p avformat rawvideo`
- `cargo fmt --all`
- `cargo test -p avformat rawvideo`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p avformat image2`
- `cargo fmt --all`
- `cargo test -p avformat image2`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo fmt --all`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo run -p oracle -- --help`
- `cargo run -p xtask -- quick`
- `cargo fmt --all`
- `cargo test -p avutil byteio`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test --workspace --all-features`
- `cargo test -p avutil bitreader`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avutil bitwriter`
- `cargo test -p avutil dict`
- `cargo test -p avutil options`
- `cargo test -p avutil hash`
- `cargo test -p fftools option_parser`
- `cargo test -p fftools --test version hide_banner`
- `cargo test -p fftools io_plan`
- `cargo test -p avformat avio`
- `cargo test -p avformat probe`
- `cargo test -p avformat null_muxer`
- `cargo test -p avformat hash_muxer`
- `cargo test -p avformat framecrc_muxer`
- `cargo test -p avcodec rawvideo`
- `cargo test -p avcodec pcm`
- `cargo test -p avformat wav`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat wav`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat pcm::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat rawvideo`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat yuv4mpegpipe`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat image2`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat avi`
- `cargo test --workspace --all-features`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p avformat avi::tests`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat avi::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat probe`
- `cargo test -p avformat mov::tests`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools option_parser`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools option_parser`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools option_parser`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools option_parser`
- `cargo fmt --all`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffmpeg`
- `cargo test -p fftools`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p avformat mov::tests`
- `cargo test -p fftools ffprobe`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo test -p fate-runner`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fate-runner`
- `cargo run -p fate-runner -- run --component fate-runner`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `git diff --check`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all`
- `cargo test -p fate-runner`
- `cargo run -p fate-runner -- run --samples . --oracle-ffmpeg Cargo.toml --component fate-runner`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo run -p fate-runner -- list`
- `git diff --check`
- `cargo fmt --all -- --check`
- `cargo test -p fate-runner`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\\fuzz_targets\\fftools_option_parser.rs`
- `cargo test -p fftools --lib option_parser`
- `cargo test -p fftools --lib loglevel`
- `cargo test -p fftools --lib log_level`
- `cargo check --manifest-path fuzz\\Cargo.toml --bin fftools_option_parser`
- `cargo clippy --manifest-path fuzz\\Cargo.toml --bin fftools_option_parser -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo fmt --all -- --check`
- `cargo test -p fate-runner default_mappings_cover_current_fftools_smoke_selections`
- `cargo run -p fate-runner -- mappings`
- `cargo run -p fate-runner -- mappings --check-prereqs --samples . --oracle-ffmpeg Cargo.toml`
- `cargo run -p fate-runner -- run --component fftools-option-parser`
- `cargo run -p fate-runner -- run --component fftools-basic-io`
- `cargo run -p fate-runner -- run --component fftools-ffprobe-mov-show-format`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `cargo fmt --all -- --check`
- `cargo test -p fate-runner default_mappings_cover_current_fftools_smoke_selections`
- `cargo run -p fate-runner -- mappings`
- `cargo run -p fate-runner -- mappings --check-prereqs --samples . --oracle-ffmpeg Cargo.toml`
- `cargo run -p fate-runner -- run --component fftools-ffmpeg-mov-framecrc-null`
- `cargo run -p fate-runner -- run --component fftools-version`
- `cargo run -p fate-runner -- run --component fftools-hide-banner`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame_side_data_interprets_common_exif_tags`
- `cargo test -p avutil frame_side_data_interprets_exif_exposure_tags`
- `cargo test -p avutil frame_side_data_interprets_exif_descriptive_tags`
- `cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_altitude_time_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_common_exif_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_acquisition_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_motion_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_destination_tags`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avutil-frame`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed --dry-run`
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil empty_frame_unref_clears_data_and_releases_references`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`
- `cargo fmt --all`
- `rustfmt fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo test -p avutil frame_ref_from_shares_references_and_replaces_destination`
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models`
- `cargo test -p avutil frame`
- `cargo fmt --all -- --check`
- `rustfmt --check fuzz\fuzz_targets\avutil_core_models.rs`
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo run -p fate-runner -- run --component avutil-frame`
- `cargo run -p fate-runner -- run --changed --dry-run`
- `cargo run -p fate-runner -- run --changed`
- `git diff --check`

## Last Failing Commands

- Current `fate-runner` differential-mapping coverage slice:
  - `cargo run --target-dir target-codex -p fate-runner -- mappings --mappings tests/differential/mappings.txt` was blocked by Windows Application Control on `target-codex\debug\fate-runner.exe`.
  - `cargo run -p fate-runner -- mappings --mappings tests/differential/mappings.txt` was blocked by Windows Application Control on `target\debug\fate-runner.exe`.
  - After rustfmt rebuilt the test binary, `cargo test -p fate-runner --target-dir target-codex` was blocked by Windows Application Control on `target-codex\debug\deps\fate_runner-*.exe`. The same focused test passed through `target-fate-runner-diff-test`, and the CLI dry-runs passed through `target-fate-runner-diff-bin`.

- Current `avutil-channel-layout` oracle-harness slice after adding the fate-runner regression test:
  - `cargo test -p fate-runner` through the default `target` directory is blocked by Windows Application Control on `target\debug\deps\fate_runner-*.exe` after the rebuilt test executable is created. `cargo test -p fate-runner --target-dir target-codex` passes.
  - `$env:CARGO_TARGET_DIR='target-codex'; cargo run --target-dir target-codex -p fate-runner -- run --changed` passes the `fate-runner` self-test but is blocked by Windows Application Control on `target-codex\debug\deps\channel_layout_oracle-*.exe`. The same avutil component mapping passes through the default target directory.

- Current `avutil-channel-layout` `ffmpeg -layouts` oracle-harness slice: the first `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run` failed because `crates/avutil/tests/channel_layout_oracle.rs` had no changed-path selection rule. A `fate-runner` path rule now maps that test to `avutil-channel-layout`, and the dry-run plus real changed-path run pass. The ignored oracle test has not been executed because no pinned FFmpeg binary exists locally.

- Current `avutil-channel-layout` whitespace/full-consumption parser slice: no remaining failing commands. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-channel-layout` case-sensitive parser-string slice: no remaining failing commands. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-channel-layout` explicit ambisonic strtol-order parser slice: no remaining failing commands. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-channel-layout` byte-entry parser slice: no failing commands. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-channel-layout` raw-channel-mask retype slice: no failing commands. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-channel-layout` ambisonic/canonical-target retype slice: no failing commands. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-channel-layout` lossy native/unspecified-target retype slice: no failing commands. Focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run/execution, formatting, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-channel-layout` custom-target retype slice:
  - First `cargo test -p avutil channel_layout` run failed because the new unit test expected a count-only unspecified description for `ChannelLayoutSpec::unspecified(2).to_custom_layout()`. The implementation correctly produced a custom map with two `UNK` entries, so the test expectation was fixed to `2 channels (UNK+UNK)` and the same focused test then passed.

- Current `avutil-channel-layout` count-suffix parser slice:
  - `cargo test -p avutil channel_layout --target-dir target-codex` failed before running tests because Windows Application Control blocked the generated `target-codex\debug\deps\avutil-*.exe` test binary with OS error 4551.
  - The same focused avutil channel-layout test passed with `cargo test -p avutil channel_layout` through Cargo's default `target` directory, so this is recorded as an environment/artifact policy issue, not a Rust test failure.

- Current `avutil-channel-layout` unspecified-layout slice:
  - `cargo test -p avformat audio --target-dir target-codex` failed to execute the generated `target-codex\debug\deps\avformat-*.exe` test binary because Windows Application Control blocked that file before test execution. The same failure reproduced once with escalation and once with a fresh `target-codex-avformat` directory.
  - The focused avformat audio test passed with `cargo test -p avformat audio` using Cargo's default `target` directory, and the changed-path FATE-runner mapping also passed using its mapped default-target cargo commands. This is recorded as an environment/artifact policy issue, not a Rust test failure.

- Previous `avutil-channel-layout` default-count slice:
  - No command failures were observed while expanding `ChannelLayout::default_for_count` to source-order modeled defaults; focused channel-layout unit tests, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, FATE listing, and diff checks passed.
  - At that point, unmodeled default counts still returned `None` because the Rust model had no `AV_CHANNEL_ORDER_UNSPEC` representation. Full parser grammar, broad retyping, implicit ambisonic layout semantics, oracle inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-channel-layout` lookup slice:
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex` initially failed because deterministic native lookup assertions were inserted before the fuzz fixture's `layout` variable was declared. The assertions were moved to the existing generated-layout section, and the same command then passed.
  - Full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC` layout semantics, `ffmpeg -layouts` oracle inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-channel-layout` subset-mask slice:
  - No command failures were observed while adding current-subset native/custom subset-mask helpers; focused channel-layout unit tests, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, FATE listing, and diff checks passed.
  - Full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC` layout semantics, `ffmpeg -layouts` oracle inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-channel-layout` comparison slice:
  - No command failures were observed while adding current-subset native/custom layout equivalence helpers; focused channel-layout unit tests, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, and diff checks passed.
  - Full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC` layout semantics, `ffmpeg -layouts` oracle inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-channel-layout` custom ambisonic slice:
  - No command failures were observed while adding bounded custom-map ambisonic order detection/description; focused channel-layout unit tests, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, and diff checks passed.
  - Full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC` layout semantics, layout comparison, `ffmpeg -layouts` oracle inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-channel-layout` slice:
  - No command failures were observed while adding source-shaped custom-to-native canonicalization helpers; focused channel-layout unit tests, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, and downstream `avformat` audio tests passed so far.
  - Full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic retyping, `AV_CHANNEL_ORDER_AMBISONIC` layout semantics, layout comparison, `ffmpeg -layouts` oracle inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Previous `avutil-channel-layout` custom-map slice:
  - No command failures were observed while adding `ChannelCustom` and `CustomChannelLayout`; focused channel-layout unit tests, formatting, fuzz-package build/clippy, avutil clippy, local FATE, changed-path FATE, downstream `avformat` audio tests, and diff checks passed.
  - Full `av_channel_layout_from_string()` grammar, native/custom/ambisonic retyping, `AV_CHANNEL_ORDER_AMBISONIC` layout semantics, layout comparison, `ffmpeg -layouts` oracle inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Previous `avutil-channel-layout` ChannelId slice:
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex` initially failed because the fuzz target imported `avutil::ChannelId` before `crates/avutil/src/lib.rs` re-exported it. `ChannelId` is now re-exported and the rerun passes.
  - `Invoke-WebRequest -Uri https://raw.githubusercontent.com/FFmpeg/FFmpeg/n8.1.1/libavutil/channel_layout.c -OutFile $env:TEMP\ffmpeg-channel-layout-8.1.1.c` initially failed inside the sandbox with `Unable to connect to the remote server`; the same pinned-source fetch passed outside the sandbox through the approved `Invoke-WebRequest` prefix.
  - No remaining code/test assertion failures have been observed while adding the `ChannelId` raw ID helper; focused channel-layout unit tests, local FATE mapping, fuzz-package build/clippy, changed-path smoke run, `avformat` audio regression, format checks, and `git diff --check` passed.
  - Full `ffmpeg -layouts` oracle inventory comparison, remaining FFmpeg default-layout count behavior beyond the modeled first-map entries for counts 1, 2, 3, 4, 5, 6, 7, and 8, custom/native/ambisonic order semantics, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-sample-format` table-string slice: no command failures have been observed so far. Pinned sample-format oracle vectors, upstream FATE parity, broader sample-data conversion routines, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-sample-format` fill-array layout slice:
  - `cargo clippy -p avutil --all-targets --target-dir target-codex -- -D warnings` initially failed on `clippy::needless_lifetimes` for the convenience `split_buffer` and `split_buffer_mut` methods. The signatures now use elided lifetimes, and the rerun passes.
  - Pinned sample-format oracle vectors, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, sample-data conversion routines, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-sample-format` copy-helper slice: no command failures have been observed so far. Pinned sample-format oracle vectors, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-sample-format` silence-fill slice: no command failures have been observed so far. Pinned sample-format oracle vectors, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-sample-format` sample-buffer-layout slice:
  - The first sandboxed `Invoke-WebRequest` attempt for pinned `libavutil/samplefmt.c` failed with `Unable to connect to the remote server`; the download succeeded after escalation.
  - `cargo test -p avutil samplefmt --target-dir target-codex` compiled successfully, then Windows Application Control blocked the freshly built `avutil` unit-test executable with `os error 4551`; rerunning the same focused filter through the default target cache passed.
  - `cargo check --manifest-path fuzz\Cargo.toml --target-dir target-codex` initially failed because the new fuzz alignment selector inferred `i32`, so `usize::from(buffer_alignment)` was invalid. The selector is now typed as `usize`, and the fuzz package check passes.
  - Pinned sample-format oracle vectors, upstream FATE parity, and actual fuzz execution remain blockers rather than completion claims.

- Current `avutil-bitwriter` truncate/reset slice: no Rust assertion, clippy, local FATE, formatting, or diff-hygiene failures were observed. The implementation intentionally records missing pinned PutBitContext oracle vectors, upstream FATE parity, and actual fuzz execution as blockers instead of claiming completion.

- Current `avutil-timebase` signed stable-add slice: no Rust assertion, clippy, local FATE, formatting, or diff-hygiene failures were observed. The implementation intentionally records missing pinned oracle vectors, upstream FATE parity, actual fuzz execution, and out-of-range C behavior calibration as blockers instead of claiming completion.

- Current `avutil-timebase` rescale-delta slice: no Rust assertion, clippy, local FATE, or formatting failures were observed. The implementation intentionally records missing pinned oracle vectors, upstream FATE parity, actual fuzz execution, and negative `av_add_stable` calibration as blockers instead of claiming completion.

- Current `avutil-timebase` stable-add slice: no Rust assertion, clippy, local FATE, or formatting failures were observed. The implementation intentionally records missing pinned oracle vectors, upstream FATE parity, actual fuzz execution, `av_rescale_delta`, and negative-increment calibration as blockers instead of claiming completion.

- Current `avutil-timebase` compare helper slice:
  - The first sandboxed `Invoke-WebRequest` attempts for pinned `libavutil/mathematics.h` and `libavutil/mathematics.c` failed with `Unable to connect to the remote server`; both downloads succeeded after escalation.
  - The first focused `cargo test -p avutil timebase --target-dir target-codex` failed because the large timestamp comparison vector expected `Less` while the exact cross-product was `Greater`; the test vector was corrected to exercise a true large-value `Less` case. No remaining Rust assertion, clippy, local FATE, or diff-hygiene failures remain for this slice.

- Current `avutil-rational` int-float slice: no Rust assertion, clippy, local FATE, or diff-hygiene failures were observed.

- Current `avutil-rational` comparison/nearest slice:
  - The first sandboxed `Invoke-WebRequest` attempts for pinned `libavutil/rational.h` and `libavutil/rational.c` failed with `Unable to connect to the remote server`; both downloads succeeded after escalation. No Rust test, clippy, local FATE, or diff-hygiene failures remain for this slice.

- Current `avutil-error` string-table slice:
  - The first focused `cargo test -p avutil error --target-dir target-codex` failed because the new test expected a distinct `Input and output changed` description. Source checking and the failure showed `AVERROR_INPUT_CHANGED | AVERROR_OUTPUT_CHANGED` has the same raw value as `AVERROR_INPUT_CHANGED`, and FFmpeg's first table match returns `Input changed`; the test and fuzz invariant were corrected and the focused test passed.

- Current `fate-runner` oracle-env mapping slice:
  - `cargo run --target-dir target-codex -p fate-runner -- mappings --mappings tests/differential/mappings.txt --check-prereqs` failed as expected because the differential mappings reference `{oracle_ffmpeg}` and require `--oracle-ffmpeg <path>` before injecting `FFMPEG_ORACLE`. This confirms the differential mapping file does not silently skip the pinned-oracle prerequisite.

- Current `fate-runner` repeated-component slice: no assertion failure remains. The prior repeated-`--component` rejection is resolved by accepting multiple explicit component IDs and deduplicating duplicate IDs before executing mappings.

- Current rawvideo oracle harness slice:
  - `cargo test -p fftools --test rawvideo_oracle --target-dir target-codex -- --ignored` failed as expected before parity comparison because no pinned FFmpeg oracle binary is available at `FFMPEG_ORACLE` or `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)`.
  - `cargo run --target-dir target-codex -p fate-runner -- run --changed --dry-run` initially failed because `crates/fftools/tests/rawvideo_oracle.rs` had no changed-path component selection rule; adding the rawvideo oracle harness path rule fixed it, and the same dry-run plus actual changed-path local smoke run now pass.

- Current MSB-aligned planar YUV444/GBR `avutil-pixel-format` / rawvideo slice: no Rust assertion failure remains. A single FATE dry-run command with repeated `--component` flags failed because the runner currently accepts only one explicit component per invocation; the same component mappings dry-ran successfully one at a time. Remaining blockers are oracle differentials, upstream FATE media parity, full conversion behavior, hardware formats, and actual fuzz execution.

- Current packed 32-bit integer RGB/RGBA `avutil-pixel-format` / rawvideo slice: no Rust assertion failure remains. `cargo test -p avcodec rgb96 --target-dir target-codex`, retries through `target-avcodec-rgb96-test`, `target-avcodec-rgba64-test`, and `target-avcodec-rgb96-release-test`, were blocked before execution by Windows Application Control (`os error 4551`); the same rawvideo decoder coverage passed through `cargo run --target-dir target-codex -p fate-runner -- run --component avcodec-rawvideo`. `cargo run --target-dir target-codex -p fate-runner -- run --component fftools-ffmpeg-rawvideo-framecrc-null` remains blocked before executing the default-target `fftools` lib test executable, while the focused target-codex `fftools` test passed. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed floating RGBA `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through `target-codex`; format check, workspace clippy, fuzz-package check/clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed floating RGB `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through `target-codex`; format check, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current planar floating GBR `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through `target-codex`; format check, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed X/V YUV 4:4:4 `avutil-pixel-format` / rawvideo slice: no Rust code/test assertion failure remains. Initial focused avutil and avformat runs through `target-codex` and a focused avformat run through `target` were blocked before execution by Windows Application Control (`os error 4551`); the same avutil tests passed through `target-avutil-byteio-peek-test` and the same avformat tests passed through `target-avcodec-rgba64-test`. Initial focused fftools runs through `target-fftools-gray32-test` and `target-codex` were similarly blocked before the lib harness ran; `cargo test -p fftools --lib runs_rawvideo_xv30le_to_null_stdout --target-dir target-fftools-rgba64-test` passed. A broad `cargo test --workspace --lib --all-features --target-dir target-avcodec-rgba64-test` completed the avcodec library tests, including the new XV/V30X rawvideo test, then was blocked on an unrelated `avdevice` test executable. `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; cargo run --target-dir target-codex -p fate-runner -- run --changed` was blocked on the first avutil mapping executable. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete.

- Current packed floating gray+alpha YAF `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete.

- Current packed X2RGB10/X2BGR10 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete.

- Current packed XYZ12 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete.

- Current packed AYUV64 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete.

- Current high-bit packed YUV 4:2:2 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete.

- Current semi-planar NV YUV 4:2:2/4:4:4 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete.

- Current high-bit-depth planar YUVA `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, and local changed-path FATE mappings passed. Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution remain incomplete; no `yuva420p12*` variants were added because pinned FFmpeg 8.1.1 does not define them.

- Current `yuva420p` / `yuva422p` / `yuva444p` planar YUVA `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. A transient fuzz-package build failure from an incomplete invariant update was fixed before validation. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, high-bit YUVA variants, and actual fuzz execution remain incomplete.

- Current `yuv440p10*` / `yuv440p12*` planar YUV 4:4:0 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace clippy, workspace library tests, directly affected local FATE component mappings, broad local `fate-runner run --changed`, formatting, and final diff hygiene passed. Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, and actual fuzz execution remain incomplete.

- Current 14-bit and 16-bit planar YUV `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. The broad local FATE `run --changed` command selected the expected changed components but was blocked twice by Windows Application Control before child test executables ran: first on the default-target `avcodec` rawvideo test executable, then on the alternate-target `avformat` WAV test executable. The dry-run changed selection passed, and the directly affected local FATE component mappings for `avutil-pixel-format`, `avcodec-rawvideo`, `avformat-rawvideo-demuxer`, `avformat-rawvideo-muxer`, and `fftools-ffmpeg-rawvideo-framecrc-null` passed through accepted target directories. Focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace clippy, workspace library tests, formatting, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continued to report the usual `could not canonicalize path C:\Users\trevo` warning. Oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, and actual fuzz execution remain incomplete.

- Current paletted `pal8` `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Freshly built test executables were blocked by Windows Application Control for `cargo test --target-dir target-codex -p avutil pixel`, `cargo test --target-dir target-avcodec-gray32-test -p avcodec rawvideo`, and default-target avformat rawvideo FATE child tests launched by `fate-runner`; the same focused tests and mappings passed through already accepted target directories (`target-avutil-byteio-peek-test`, `target-avcodec-rgba64-test`, `target-avcodec-ya16-test`, and `target-fftools-gray32-test`). A WSL source-inspection fallback also failed because no Ubuntu distro is installed, so pinned-source checking used the cached FFmpeg 8.1.1 snippets under `C:\tmp`. No Rust assertion failure remains; full palette side-plane/context propagation, oracle differentials, upstream FATE media coverage, and actual fuzz execution remain incomplete.

- Current deprecated full-range planar YUVJ `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. The first focused `fftools` test failed because the new test used a nonexistent `FfmpegOutput::muxer()` helper; switching to the existing `output_format()` accessor fixed it. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, local FATE-runner component mappings, local `run --changed`, workspace clippy, workspace library tests, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current 1bpp monochrome bitstream `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. The first touched fuzz-target clippy run failed on `manual_is_multiple_of` for the new fuzz helper; replacing the helper with `is_multiple_of` then failed affected-crate clippy because the method is newer than the repo's MSRV 1.75. The final helper uses `width.div_ceil(8)`, and focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current packed 4bpp RGB bitstream `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures and no fresh Windows Application Control blocks during validation. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current byte-packed low-bit-depth RGB `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. Default-target `cargo test -p avutil pixel`, `cargo test -p avutil frames_report_tightly_packed_line_sizes`, and `cargo test -p avcodec rawvideo` were blocked before execution by Windows Application Control at freshly rebuilt unit-test executables; the same focused tests passed through `target-codex`. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, and local `run --changed` passed. Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current semi-planar YUV 4:2:0 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures. The first `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel` and retry through `target-avutil-timebase-test` were blocked before execution by Windows Application Control at the rebuilt `avutil` unit-test executable; the same focused test passed through the default Cargo target cache. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current packed YUV 4:2:2 `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures and no fresh Windows Application Control blocks during validation. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current packed 16-bit RGB/BGR `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures and no fresh Windows Application Control blocks during validation. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current high-bit-depth grayscale `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures and no fresh Windows Application Control blocks during validation. The first `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel` failed because the expected `PixelFormat::ALL.len()` assertion still used the previous inventory count; the expected count was updated from 62 to 70 and the focused test passed on retry. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current planar GBRA `avutil-pixel-format` / rawvideo slice: no remaining code/test assertion failures and no fresh Windows Application Control blocks during validation. The initial `rg` searches could not run because `rg` is not available in this PowerShell session, so the descriptor/code searches were repeated with `Select-String`. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only; Cargo commands continue to report the usual `could not canonicalize path C:\Users\trevo` warning.

- Current `avutil-pixel-format` / rawvideo `gbrp16le`/`gbrp16be` slice: no remaining code/test assertion failures and no fresh Windows Application Control blocks during the focused validation. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `gbrp14le`/`gbrp14be` slice: no remaining code/test assertion failures. Default-target `cargo test -p avutil pixel` and `cargo test -p avutil frames_report_tightly_packed_line_sizes` were blocked before execution by Windows Application Control at the freshly built `avutil` unit-test executable; the same tests passed through `target-codex`. `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_gbrp14le_to_null_stdout` was blocked before execution at the `fftools` unit-test executable; the same focused test passed from the default target cache. Default-target `cargo run -p fate-runner -- run --changed` was blocked before execution at the default-target `avutil` unit-test executable; `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed` passed. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `gbrp12le`/`gbrp12be` slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `gbrp10le`/`gbrp10be` slice: no remaining code/test assertion failures. The first `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel` and retry were blocked by Windows Application Control at the freshly built `target-codex` avutil unit-test executable. The same focused avutil tests passed from the default Cargo target directory, and focused avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, and `git diff --check` passed. `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `gbrp9le`/`gbrp9be` slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected-crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` passed. `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `gbrp` slice: no remaining code/test assertion failures in the focused gate. Windows Application Control blocked some freshly built executables in specific target caches: `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo` at the `avcodec` test executable, default-target `cargo test -p fftools --lib runs_rawvideo_gbrp_to_null_stdout` at the `fftools` test executable, `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo` at the `avcodec` test executable, `cargo run -p fate-runner -- run --changed` at the default-target `fftools` test executable after earlier avutil mappings had passed, `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features` at the `avcodec` test executable, and default-target `cargo test --workspace --all-features` at the `fate-runner` bin-test executable after `avcodec`, zero-test crates, `avformat`, and `avutil` had passed. The corresponding focused avcodec, fftools, and local FATE component mappings passed through target caches accepted by Windows Application Control. Workspace clippy, formatting, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `grayf16le`/`grayf16be`/`grayf32le`/`grayf32be` slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only. The first sandboxed `Invoke-WebRequest` for pinned FFmpeg source failed with restricted network access; the approved retry succeeded and does not block the slice.

- Current `avutil-pixel-format` / rawvideo `gray32le`/`gray32be` slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `ya16le`/`ya16be` slice: no remaining code/test assertion failures. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and `git diff --check` passed; `git diff --check` reported CRLF warnings only. The known Windows Application Control limitation on a single broad workspace library-test invocation remains documented from previous slices and was not used as the gate for this narrow slice.

- Current `avutil-pixel-format` / rawvideo `rgba64le`/`rgba64be`/`bgra64le`/`bgra64be` slice: no remaining code/test assertion failures. Windows Application Control blocked several freshly built test executables in specific target directories: `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo`, `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib runs_rawvideo_rgba64le_to_null_stdout`, `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --component avcodec-rawvideo`, `$env:CARGO_TARGET_DIR='target-avcodec-rgba64-test'; cargo run -p fate-runner -- run --component avcodec-rawvideo`, `$env:CARGO_TARGET_DIR='target-fftools-rgba64-test'; cargo test --workspace --all-features --lib` at `avdevice`, default-target `cargo test --workspace --all-features --lib` at `swscale`, and `$env:CARGO_TARGET_DIR='target-codex'; cargo test --workspace --all-features --lib` at `avcodec`. The affected focused tests and FATE mappings passed by running the already-built `target-codex` fate-runner with accepted target directories, and `avdevice`/`swscale` zero-test crates pass individually from `target-codex`; workspace-wide `--lib` in one target directory remains blocked by policy rather than Rust test failures. `git diff --check` passed with CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `ya8` slice: no remaining failing validation commands. The first `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_ya8_to_null_stdout` run passed the library test but then failed when Cargo tried to execute the generated `ffmpeg-rs` binary test harness and Windows Application Control blocked that executable. Rerunning the actual changed test with `--lib` passed, and focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `rgb48le`/`rgb48be`/`bgr48le`/`bgr48be` slice: no remaining failing validation commands. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `gray16le`/`gray16be` slice: no remaining failing validation commands. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo packed 32-bit RGB padding slice: no remaining failing validation commands. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `yuv440p` slice: no remaining failing validation commands. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `yuv410p` slice: no remaining failing validation commands. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `yuv411p` slice: the first `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools runs_rawvideo_yuv411p_to_null_stdout` run failed to compile because the new test asserted a nonexistent `FfmpegOutput::input_format()` helper; removing that stray assertion fixed it. No remaining failing validation commands. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo `yuv444p` slice: no remaining failing validation commands. Focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and workspace library tests passed.

- Current `avutil-pixel-format` / rawvideo `yuv422p` slice: the first `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avcodec rawvideo` run failed to compile the new rawvideo decoder test because the `FrameData` match did not cover `FrameData::Empty`; adding the missing match arm fixed it. No remaining failing validation commands. Focused unit tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` descriptor metadata slice: no remaining failing validation. Focused pixel tests, fuzz-target check/clippy, `avutil` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-pixel-format` / rawvideo packed-RGB slice: `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo test -p avformat rawvideo` compiled but Windows Application Control blocked the freshly built avformat unit-test executable with `os error 4551`; rerunning the same focused test through `target-codex` passed. The first `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p fate-runner -- run --changed` failed because changed `avformat_basic_muxers` coverage selected `avformat-wav-muxer`, `avformat-pcm-s16le-muxer`, and `avformat-yuv4mpegpipe-muxer` without local mappings. Added local smoke mappings for those muxers, plus rawvideo demuxer/muxer mappings, updated the ledger/docs, and reran `run --changed` successfully. No code-related failing validation remains; `git diff --check` reported CRLF warnings only.

- Current `avutil-hash` / SHA-160 hash-muxer slice: initial `$env:CARGO_TARGET_DIR='target-fftools-cli-color-test'; cargo run -p fate-runner -- run --changed` failed because the changed packet-muxer fuzz target selected `avformat-null-muxer`, `avformat-hash-muxer`, `avformat-framecrc-muxer`, `avformat-framehash-muxer`, and `avformat-streamhash-muxer`, but those ledger components had no local mappings. Added local avformat packet-muxer unit mappings and reran the same command successfully. No code-related failing validation remains; `git diff --check` reported CRLF warnings only.

- Current `fftools-option-parser`/CLI repeat-summary logging slice: `$env:CARGO_TARGET_DIR='target-fftools-cli-repeat-test'; cargo test -p fftools --lib cli_logging` compiled but Windows Application Control blocked the freshly built `fftools` unit-test executable before tests ran with `os error 4551`; rerunning the same focused test through the previously allowed `target-fftools-cli-color-test` target passed. No code-related failing validation remains. Full `fftools` library tests, `fftools` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library compile check, full workspace library tests, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `fftools-option-parser`/CLI terminal-color logging slice: no remaining failing commands. Focused `cli_logging` tests, full `fftools` library tests, `fftools` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library compile check, full workspace library tests, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.

- Current `avutil-logging` terminal color slice: `$env:CARGO_TARGET_DIR='target-avutil-logging-terminal-test'; cargo clippy -p avutil --all-targets -- -D warnings` initially failed on a duplicated `Always` branch in `LogColorMode::from_ffmpeg_env_vars_and_stderr`; folding forced-color and terminal detection into a single branch fixed clippy. `$env:CARGO_TARGET_DIR='target-avutil-logging-terminal-test'; cargo test -p avutil logging` then compiled but Windows Application Control blocked the rebuilt focused test executable; rerunning the same focused test through `target-avutil-timebase-test` passed. `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo run -p fate-runner -- run --component avutil-logging` was blocked launching the rebuilt `fate-runner.exe`, and `cargo run -p fate-runner -- run --component avutil-logging` was blocked by the child default-target `avutil` test executable; running the already-built default `.\target\debug\fate-runner.exe` with `CARGO_TARGET_DIR=target-avutil-timebase-test` passed for both component and changed mappings. `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; cargo test --workspace --all-features --lib` compiled but Windows Application Control blocked the rebuilt `avcodec` lib-test executable before tests ran; the same workspace lib suite passed compilation with `--no-run`. No code-related failing validation remains.
- Current `fftools-option-parser`/CLI forced-color logging slice: no remaining failing commands. Focused `cli_logging` tests, full `fftools` library tests, `fftools` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and workspace library tests all passed.
- Current `avutil-logging` forced-color env slice: `$env:CARGO_TARGET_DIR='target-avutil-logging-env-test'; cargo test -p avutil logging` initially compiled but Windows Application Control blocked launching the fresh `avutil` test binary in that target dir; the same command passed when rerun outside the sandbox. `$env:CARGO_TARGET_DIR='target-avutil-logging-env-test'; cargo run -p fate-runner -- run --component avutil-logging` was also blocked by Application Control even outside the sandbox; rerunning the same mapping through the default target cache passed. No remaining failing validation commands; `git diff --check` reported CRLF warnings only.
- Current `fftools-option-parser`/CLI timestamp logging slice: no remaining failing commands. Focused `cli_logging` tests, full `fftools` library tests, `fftools` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and workspace library tests all passed.
- Current `fftools-option-parser`/CLI logging slice: `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test -p fate-runner changed_selection default_mappings_cover_current_fftools_smoke_selections` failed because Cargo accepts only one test-name filter. The `fate-runner` unit suite was rerun as `$env:CARGO_TARGET_DIR='target-fftools-cli-logging-test'; cargo test -p fate-runner` and passed. No remaining failing validation commands for the final slice; `git diff --check` reported CRLF warnings only.
- Current `avutil-logging` color-formatting slice: no remaining failing commands. Formatting, focused logging tests, fuzz-target check/clippy, avutil clippy, workspace clippy, local FATE-runner logging/changed mappings, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.
- Current `avutil-logging` system-time timestamp slice: `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo test -p avutil logging` initially failed `logging::tests::log_timestamps_convert_system_time_to_unix_micros` because Windows `SystemTime` did not preserve `UNIX_EPOCH - 1ns` as a distinct pre-epoch public value; the test now checks that sub-microsecond floor behavior through the internal duration conversion helper and uses representable `SystemTime` values for public conversion. `cargo fmt --manifest-path fuzz\Cargo.toml --all -- --check` also failed with a rustfmt stack overflow on the very large fuzz package even with `RUST_MIN_STACK=33554432`; the workspace format check passed, and the touched fuzz target passed check and clippy. No remaining failing validation commands for the final slice; `git diff --check` reported CRLF warnings only.
- Current `avutil-logging` global logger slice: no remaining failing commands. Formatting, focused logging tests, fuzz-target check/clippy, avutil clippy, workspace clippy, local FATE-runner logging/changed mappings, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.
- Current `avutil-logging` callback slice: no remaining failing commands. Formatting, focused logging tests, fuzz-target check/clippy, avutil clippy, workspace clippy, local FATE-runner logging/changed mappings, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.
- Current `avutil-logging` timestamp-formatting slice: `$env:CARGO_TARGET_DIR='target-avutil-logging-repeat-test'; cargo test -p avutil logging` initially failed the new `logging::tests::repeated_comparison_respects_printed_timestamp_flags` assertion because the repeat-comparison flag check required both `PRINT_TIME` and `PRINT_DATETIME` instead of either timestamp flag. Adding `LogFlags::intersects` and using it for timestamp-aware repeat comparison fixed the bug; all rerun validation passed. `git diff --check` reported CRLF warnings only.
- Current `avutil-logging` repeat-compression slice: no remaining failing commands. Formatting, focused logging tests, fuzz-target check/clippy, avutil clippy, workspace clippy, local FATE-runner logging/changed mappings, and `git diff --check` all passed; `git diff --check` reported CRLF warnings only.
- Current `avutil-options` rational slice: `cargo fmt --all -- --check` initially failed only because rustfmt wanted to wrap the new `parse_rational` tuple expression; `cargo fmt --all` fixed it, and the rerun passed. No remaining failing commands in the final validation.
- Latest packet IAMF parameter side-data parser slice: `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick` passed the runtime guard, all 389 `avutil` tests, all 119 `fftools` library tests, and the 0-test `ffmpeg-rs` bin harness, then failed only because Windows Application Control blocked launching the freshly rebuilt `target-avutil-timebase-test\debug\deps\ffprobe_rs-9dad61cbb3658cf1.exe` with `os error 4551`. Fallback validation passed with `cargo test --target-dir target-avutil-timebase-test -p fftools --lib` and `cargo test --target-dir target-avutil-timebase-test -p fftools --bins --no-run`.
- Latest packet encryption side-data parser slice: transient test compile failures were fixed before commit. `cargo test --target-dir target-avutil-timebase-test -p avutil packet_side_data_parses_encryption --no-run` initially failed on an untyped empty-slice assertion in the new init-info test, then on borrowing a temporary zero-count byte array; replacing the assertion with `is_empty()` and binding the byte array fixed both issues. No remaining failing commands in the final validation; `xtask quick` passed in this slice.
- Latest packet string-metadata side-data parser slice: `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick` passed the runtime guard, all 381 `avutil` tests, and all 119 `fftools` library tests, then failed only when Windows Application Control blocked launching the freshly rebuilt `target-avutil-timebase-test\debug\deps\ffmpeg_rs-083f1a0bc8974a7d.exe` bin test harness with `os error 4551`. `cargo test --target-dir target-avutil-timebase-test -p fftools --lib` and `cargo test --target-dir target-avutil-timebase-test -p fftools --bins --no-run` both pass.
- Latest packet NEW_EXTRADATA/H263-MB-info side-data parser slice: transient compile/lint failures were fixed before commit. `cargo test --target-dir target-avutil-timebase-test -p avutil packet --no-run` initially failed because the H.263 parser needed a local `read_u16_le` helper; `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings` then rejected a manual modulo multiple check; replacing it with `usize::is_multiple_of` made fuzz clippy pass but `cargo clippy --workspace --all-targets --all-features -- -D warnings` rejected that API as newer than the Rust 1.75 MSRV, so the final code uses `chunks_exact(...).remainder()` and all focused/workspace clippy checks pass.
- Latest packet NEW_EXTRADATA/H263-MB-info side-data parser slice: `.\target\debug\fate-runner.exe run --component avutil-packet` failed only because its child `cargo test` used the default target directory and Windows Application Control blocked launching `target\debug\deps\avutil-c501250f1d03cde5.exe` with `os error 4551`; rerunning the same mapping with `CARGO_TARGET_DIR=target-avutil-timebase-test` passed.
- Latest packet NEW_EXTRADATA/H263-MB-info side-data parser slice: `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick` passed the runtime guard, all 379 `avutil` tests, and all 119 `fftools` library tests, then failed only when Windows Application Control blocked launching the freshly rebuilt `target-avutil-timebase-test\debug\deps\ffprobe_rs-9dad61cbb3658cf1.exe` bin test harness with `os error 4551`. `cargo test --target-dir target-avutil-timebase-test -p fftools --lib` and `cargo test --target-dir target-avutil-timebase-test -p fftools --bins --no-run` both pass.
- Latest packet palette side-data parser slice: `$env:CARGO_TARGET_DIR='target-avutil-timebase-test'; .\target\debug\xtask.exe quick` passed the runtime guard and all 376 `avutil` tests, then failed only when Windows Application Control blocked launching the freshly rebuilt `target-avutil-timebase-test\debug\deps\fftools-649f05dff8861706.exe` with `os error 4551`. `cargo test --target-dir target-avutil-timebase-test -p fftools --lib --no-run` builds that executable successfully, and focused packet tests, fuzz-target check/clippy, avutil clippy, formatting, workspace clippy, FATE-runner packet/changed smoke mappings, and `git diff --check` pass.
- No remaining failing commands in the latest packet Dynamic HDR10+ side-data parser slice. Focused packet tests, fuzz-target check/clippy, avutil clippy, formatting, workspace clippy, FATE-runner packet/changed smoke mappings, and `xtask quick` pass.
- No remaining failing commands in the latest packet ambient viewing environment side-data parser slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- Transient in the packet stereo3d side-data parser slice: `cargo test --target-dir target-avutil-timebase-test -p avutil packet` initially failed to compile because a malformed-payload test table left integer literals ambiguous for `to_ne_bytes`; suffixing the literals as `i32` fixed the issue and the focused packet tests passed on rerun.
- Current packet audio-service-type side-data parser slice: default/debug, `target-codex`, `target-avutil-audio-service-test`, and release test harness launches were blocked by Windows Application Control with `os error 4551` after a rebuild. The existing `target-avutil-opaque-ref-test` and `target-avutil-timebase-test` target directories both launched the rebuilt focused packet test successfully, and the packet FATE mapping, changed FATE mappings, and `xtask quick` pass when `CARGO_TARGET_DIR=target-avutil-timebase-test` is used. `cargo clippy -p avutil --all-targets -- -D warnings` initially failed on a useless `.into_iter()` in the new test loop; removing that test-only conversion fixed the clippy failure.
- No remaining failing commands in the current packet display-matrix side-data parser slice. Focused packet tests, fuzz-target check/clippy, avutil clippy, formatting, workspace clippy, FATE-runner packet/changed smoke mappings, `xtask quick`, and `git diff --check` pass.
- No remaining failing commands in the current packet LCEVC side-data parser slice. `git diff --check` exited successfully with CRLF line-ending warnings only, and `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p xtask -- quick` passed in this turn.
- Current spherical packet side-data slice: `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p xtask -- quick` was run twice. Both runs passed `xtask guard-runtime` and the rebuilt `avutil` unit suite (355 tests), then failed only because Windows Application Control blocked launching the unchanged rebuilt `target-codex\debug\deps\fftools-649f05dff8861706.exe` with `os error 4551`. `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p fftools --lib --no-run` builds that test executable successfully, and focused packet tests, fuzz-target check/clippy, avutil clippy, workspace clippy, FATE-runner packet/changed smoke mappings, formatting, and `git diff --check` pass.
- Transient in this WebVTT packet side-data slice: `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil packet` and then `cargo test -p avutil packet` both compiled the focused test binary but initially hit Windows Application Control on `avutil-c501250f1d03cde5.exe` with `os error 4551`. `cargo test -p avutil packet --no-run` built successfully, and subsequent `cargo run -p fate-runner -- run --component avutil-packet`, direct `cargo test -p avutil packet`, changed FATE mappings, and `cargo run -p xtask -- quick` all passed.
- `$env:CARGO_TARGET_DIR='target-codex'; .\target\debug\xtask.exe quick` ran the runtime guard and passed the rebuilt `avutil` tests, then failed when Windows Application Control blocked the rebuilt `target-codex\debug\deps\fftools-649f05dff8861706.exe` with `os error 4551`; `cargo test -p fftools --lib` in the default target directory failed with the same policy block. `cargo test -p avutil -p fftools --lib --no-run` still builds both test executables successfully, and the focused packet, avutil clippy, fuzz-target check/clippy, local FATE mapping, changed FATE mappings, formatting, diff-check, and workspace clippy gates pass.
- No remaining failing commands in the latest packet subtitle-position side-data parser slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest packet JP-dual-mono/MPEG-TS-stream-id side-data parser slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest packet parameter-change side-data parser slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest packet skip-samples/frame-cropping side-data parser slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest typed packet side-data kind slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo test -p avutil packet` compiled the focused test binary but Windows Application Control blocked launching `target\debug\deps\avutil-c501250f1d03cde5.exe` with `os error 4551`; rerunning with `CARGO_TARGET_DIR=target-avutil-opaque-ref-test`, with `CARGO_TARGET_DIR=target-avutil-timebase-test`, and outside the sandbox produced the same OS policy block. Rerunning with `CARGO_TARGET_DIR=target-codex` passed all 18 focused packet tests, and the local `fate-runner` mapping also passed through `target-codex`.
- Cleanup of the generated `target-avutil-opaque-ref-test` and `target-avutil-timebase-test` directories was requested after verifying both paths resolved under the workspace, but the recursive delete approval was rejected; the directories remain untracked.
- `$env:CARGO_TARGET_DIR='target-codex'; cargo run -p xtask -- quick` failed because Windows Application Control blocked the freshly built `target-codex\debug\xtask.exe`. `cargo run -p xtask -- quick` launched the default `target\debug\xtask.exe` but failed when its child `cargo test` used the default target directory and hit the blocked `avutil` test executable. Running the already-built default `.\target\debug\xtask.exe quick` with `CARGO_TARGET_DIR=target-codex` passed.
- No remaining failing commands in the latest Exp-Golomb bit I/O slice.
- No remaining failing commands in the latest manifest/lockfile runtime-guard slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo test -p fate-runner default_mappings_cover_runtime_guard_selection unmapped_relevant_paths_report_crate_files_but_ignore_docs` failed because Cargo accepts only one test-name filter. It was replaced by `cargo test -p fate-runner`, which passed the full runner unit suite.
- `cargo test -p xtask` initially failed in the runtime guard slice because `std::process::Command::new("ffmpeg")` matched both a fully qualified shell-out pattern and the shorter `Command::new("ffmpeg")` substring. The redundant fully qualified patterns were removed, and `cargo test -p xtask`, `cargo run -p xtask -- guard-runtime`, focused `fate-runner` coverage, changed-smoke execution, `xtask quick`, formatting, and clippy now pass.
- No remaining failing commands in the latest View ID fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only. Two exploratory `Select-String` reads failed before the edit, one from incorrect multi-path PowerShell syntax and one timeout while printing a long compatibility line; targeted reads and all validation commands passed.
- No remaining failing commands in the latest video hint fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest ambient viewing fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only. An initial `rustfmt fuzz\fuzz_targets\avutil_core_models.rs` attempt overflowed rustfmt's stack while the new checks lived inside the already-large `exercise_fixtures` function; moving the ambient checks into a focused helper made `rustfmt`, `rustfmt --check`, fuzz-target check/clippy, workspace clippy, and FATE-runner smoke checks pass.
- No remaining failing commands in the latest detection bounding boxes fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest film grain fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest video encoding parameters fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest ROI fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest Dynamic HDR10+ fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest S12M timecode fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest ICC profile tag-table fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest content-light native-layout fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest spherical projection-table fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest video-hint empty-constant fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest frame alignment overflow fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest direct typed frame `make_writable` fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest SEI unregistered fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest malformed EXIF payload fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF linked-IFD fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF typed-entry value-semantics fuzz-coverage slice.
- No remaining failing commands in the latest EXIF root/GPS raw-value fuzz-coverage slice.
- No remaining failing commands in the latest EXIF sensitivity/capture-setting raw-value fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF raw-value preservation fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models` initially failed in this slice because the fuzz target tried to construct `FrameExifRational` with private fields. The assertions now compare public `numerator()`/`denominator()` accessors, and the fuzz-target check passes.
- No remaining failing commands in the latest EXIF common GPS coordinate fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS processing/error fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS destination fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS motion fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS acquisition fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS altitude/time/date fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root BitsPerSample fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root document/page fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root subfile-type fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root colorimetry fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root image-layout fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF rendering/scene fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF capture-setting fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF exposure/APEX fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF interoperability related-image fuzz-coverage slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue; cargo test -p avutil frame_side_data_interprets_exif_root_resolution_tags` initially failed because the malformed YResolution count case used count 2 without enough backing value bytes, causing `FrameExif::parse` to reject the fixture before `common_tags()` validation; the invalid case now uses count 0 and the focused test passes.
- No remaining failing commands in the latest EXIF descriptive ASCII-shape slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_orientation_resolution_tags` compiled successfully but Windows Application Control blocked launching `target-codex\debug\deps\avutil-...exe` with `os error 4551`; rerunning the focused test through the default target directory passed. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root Make/Model slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root Copyright slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root Predictor slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root HostComputer slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF root document/page slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_root_document_page_tags` initially failed because the malformed `DocumentName` fixture changed the ASCII entry to LONG, which made the TIFF envelope parser reject the inline bytes as an out-of-line range before the common-tag layer could reject the wrong type; the fixture now mutates that invalid entry to inline-compatible UNDEFINED, and the focused test passes.
- No remaining failing commands in the latest EXIF root subfile-type slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF SubjectArea zero-dimension validation slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF calendar-date validation slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF dimension-validation slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF SubSecTime digit-validation slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF date/time range slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS direction-range slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gps_destination_tags` initially failed in this EXIF GPS direction-range slice because the malformed destination-bearing fixture changed `91/2` to `360/2`, which is still a valid 180-degree value; the fixture now mutates the numerator to `720`, making the value exactly 360 degrees, and the focused test passes.
- No remaining failing commands in the latest EXIF GPS coordinate/time range slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF version/component fixed-shape slice.
- No remaining failing commands in the latest EXIF GPS reference fixed-count side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPSDateStamp fixed-count side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo test -p avutil frame_side_data_interprets_exif_gps_altitude_time_tags` built successfully in the default target directory but Windows Application Control blocked launching `target\debug\deps\avutil-...exe` with `os error 4551`; rerunning the same focused test with `$env:CARGO_TARGET_DIR='target-codex'` passed.
- No remaining failing commands in the latest EXIF DateTime fixed-count side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF offset-time side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_offset_time_tags` built successfully but Windows Application Control blocked the freshly built `target-codex\debug\deps\avutil-...exe` with `os error 4551`; rerunning the same focused test through the normal `target\debug` path passed.
- No remaining failing commands in the latest EXIF camera-characterization side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_camera_characterization_tags` initially failed because the intentional malformed-CFAPattern fixture changed the TIFF type to SHORT while leaving count 8, making the envelope parser reject the out-of-line value range before the common-tag layer could reject the type; the test now changes that malformed SHORT entry to count 4 so the envelope stays valid and the common-tag validation owns the error.
- No remaining failing commands in the latest EXIF sensitivity side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil frame_side_data_interprets_exif_gamma_composite_tags` initially failed in this EXIF gamma/composite slice because the malformed exposure-times type fixture changed the type to LONG while leaving count 12, making the TIFF envelope parser reject the out-of-line range before the common-tag layer could reject the type; the test now mutates that invalid LONG entry to count 1 and the focused test passes.
- No remaining failing commands in the latest EXIF gamma/composite side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF camera/lens side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF version/timing/comment side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models` initially failed once with `rustc` `STATUS_ACCESS_VIOLATION` while checking unchanged `fftools`; rerunning the same command passed.
- No remaining failing commands in the latest EXIF optics/subject side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF rendering-scene side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF capture-settings side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS acquisition side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS destination side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF GPS motion/map-datum side-data slice.
- No remaining failing commands in the latest EXIF GPS altitude/time/date side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo test -p avutil frame_side_data_interprets_exif_gps_altitude_time_tags` compiled the test binary but Windows Application Control blocked launching `target\debug\deps\avutil-c501250f1d03cde5.exe` with `os error 4551`; the same focused test passed with `$env:CARGO_TARGET_DIR='target-codex'`.
- `$env:CARGO_TARGET_DIR='C:\tmp\ffmpegrust-target'; cargo test -p avutil frame_side_data_interprets_exif_gps_altitude_time_tags` failed before build with `Access is denied. (os error 5)` creating a temporary target-dir path under `C:\tmp`; the workspace-local `target-codex` target directory worked.
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models` initially failed because `FrameExifGpsAltitudeRef` was not re-exported from `avutil`; `crates/avutil/src/lib.rs` now exports it and the fuzz-target check/clippy gates pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially timed out after five minutes while checking workspace crates; rerunning with a longer timeout passed without diagnostics.
- `cargo run -p fate-runner -- run --component avutil-frame` failed only because the local mapping's child `cargo test` used the default target directory and hit Windows Application Control; rerunning with `$env:CARGO_TARGET_DIR='target-codex'` passed.
- No remaining failing commands in the latest EXIF descriptive-tag side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo test -p avutil frame_side_data_interprets_exif_exposure_tags` initially failed in this EXIF exposure-tag slice because the negative semantic-count fixture changed the count on a LONG tag, making the TIFF parser correctly treat it as an out-of-line value before the common-tag layer could reject it; the test now mutates an inline SHORT dimension count and the focused test passes.
- No remaining failing commands in the latest EXIF exposure-tag side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest EXIF common-tag side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- `cargo test -p avutil frame_side_data_decodes_exif_entry_values` initially failed in this EXIF typed-entry slice because a big-endian fixture vector was borrowed from a temporary while entries were still referenced; the fixture now lives in a local binding and the focused test passes.
- No remaining failing commands in the latest EXIF typed-entry side-data slice. `git diff --check` exited successfully with CRLF line-ending warnings only.
- No remaining failing commands in the latest Dolby Vision side-data slice. One parallel validation attempt timed out while `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo run -p fate-runner -- run --changed --dry-run` waited on Cargo's artifact lock; both commands passed when rerun sequentially.
- No remaining failing commands in the latest detection-bboxes side-data slice.
- No remaining failing commands in the latest video-encoding-params side-data slice.
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings` initially failed in this slice because the new video encoding parameters invalid-payload helper used an OR pattern where clippy preferred a range. The helper now uses `-1..=2` and the focused fuzz clippy gate passes.
- No remaining failing commands in the latest regions-of-interest side-data slice.
- No remaining failing commands in the latest dynamic HDR10+ side-data slice.
- `cargo test -p avutil frame` initially failed in this ICC side-data slice because two test fixtures borrowed vectors immutably while mutating slices and one fixture was immutable before a slice mutation; the fixtures now store lengths in locals or avoid the unnecessary mutation, and the focused frame suite passes.
- No remaining failing commands in the latest ICC side-data slice.
- No remaining failing commands in the latest content-light side-data slice.
- No remaining failing commands in the latest spherical side-data slice.
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models` initially failed in this slice because the new mastering-display fuzz invariant passed the generated payload `Vec<u8>` where the helper expected a byte slice; the helper calls now borrow the payload and the fuzz target builds.
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings` initially failed in this slice because the invalid-length invariant used a manual `% ... != 0` multiple check; the code now uses `chunks_exact(...).remainder()` and passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed in this slice because the replacement `usize::is_multiple_of` API is newer than the repo's Rust 1.75 MSRV; the code now uses the same `chunks_exact(...).remainder()` shape and passes.
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings` initially failed in this slice because the downmix fixture used the approximate `1/sqrt(2)` literal; it now uses `std::f64::consts::FRAC_1_SQRT_2` and passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed in this slice for the same approximate downmix test literal in `crates\avutil\src\frame.rs`; the focused and workspace clippy checks now pass.
- `cargo test -p avutil frame` compiled the rebuilt avutil unit-test binary but Windows Application Control blocked launching `target\debug\deps\avutil-c501250f1d03cde5.exe` with `os error 4551` on two attempts during the previous typed-kind slice. The later side-data replacement slice rebuilt and executed the same focused filter successfully with 16 passing tests, so this is historical rather than a current blocker for `avutil-frame`.
- `cargo fmt --manifest-path fuzz\Cargo.toml -- --check` reports formatting diffs in unrelated pre-existing `fuzz\fuzz_targets\avformat_mov.rs` sections. This slice intentionally reverted those mechanical MOV formatting changes to keep the commit scoped; `cargo fmt --all -- --check` passes for the workspace, and the touched `avutil_core_models` fuzz target passes check and clippy.
- `cargo test --workspace --all-features` passed the preceding workspace crates but failed when launching the unchanged `target\debug\deps\fftools-649f05dff8861706.exe`; Windows Application Control blocked the executable with `os error 4551`. The affected slice was still validated with `cargo test --workspace --all-features --exclude fftools`, focused `avutil-frame` tests, local FATE-runner mappings, workspace clippy, and fuzz-target check/clippy.
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models` initially failed after adding readonly fuzz invariants because `release_storage` was moved by an earlier callback-release assertion; the assertion now clones the expected fixture and the fuzz target builds.
- `cargo test -p avutil buffer` initially failed after adding custom pool callbacks because a drop-time spare-list lock result outlived the upgraded pool reference in `BufferStorage::drop`; the lock result is now scoped with a terminating semicolon and the focused buffer suite passes.
- `cargo test -p avutil buffer` initially failed after adding callback-owned buffers because `BufferStorage` still had a derived `Debug` implementation alongside the manual `Debug` needed to hide callback internals; the stale derive was removed and the focused buffer suite passes.
- `cargo test -p avutil buffer` initially failed after adding automatic buffer-pool return because one method still indexed the new `Arc<BufferStorage>` wrapper and the drop-time spare-list lock temporary outlived the upgraded pool reference; `padding_slice()` now indexes the wrapped bytes and the lock result is scoped before `Drop` exits.
- `cargo check --manifest-path fuzz\Cargo.toml --bin avutil_core_models` initially failed after adding buffer-pool fuzz invariants because a mutable slice index borrowed the buffer immutably in the same expression; the index is now stored before `make_mut()` and the fuzz target builds.
- `cargo fmt --all -- --check` initially failed after adding padded buffer helpers because `checked_storage_len` needed rustfmt wrapping; `cargo fmt --all` fixed the formatting.
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avutil_core_models -- -D warnings` initially failed on a manual `% 2 == 0` branch selector in the buffer fuzz path; the harness now uses `.is_multiple_of(2)` and the focused fuzz clippy pass succeeds.
- `cargo test -p avutil frame` initially failed in this side-data inventory slice because `FrameSideDataKind::KNOWN` still used the older semantic grouping order for several side-data kinds; the enum and `KNOWN` list now match the FFmpeg 8.1.1 `libavutil/frame.h` declaration order and the focused test passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed in this linked-IFD slice because `append_linked_ifd_chain` had too many arguments; the linked pointer fields are now grouped in a private helper struct and clippy passes.
- No remaining failing commands in the latest slice. The previous no-runnable-FATE-mapping gap for current `fftools-option-parser`, `fftools-basic-io`, `fftools-ffmpeg-*`, and `fftools-ffprobe-*` source selections is now covered by local smoke mappings.
- `cargo clippy --manifest-path fuzz\Cargo.toml --bin avformat_mov -- -D warnings` initially failed after the MOV sample-entry seed slice because of a needless borrow in the packet assertion path and too many arguments in a local MOV fixture helper; `SampleEntrySpec` now groups the sample-entry fixture fields and the fuzz-target clippy pass succeeds.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed after the `tx3g` sample payload slice because `.is_multiple_of(2)` exceeded the workspace MSRV; the UTF-16 odd-byte check now uses a Rust 1.75-compatible modulo expression and clippy passes.
- `cargo test --workspace --all-features --exclude fftools`, `cargo test -p avformat --lib`, `cargo test -p avformat mov::tests`, and `cargo run -p fate-runner -- run --changed` failed after the `dec3` slice because Windows Application Control blocked the rebuilt `target\debug\deps\avformat-a0c2eebe89aa5944.exe` with `os error 4551`; focused MOV tests and the local MOV fate-runner mapping passed before the later rebuild, and `cargo test -p avformat --lib --no-run`, `cargo test --workspace --all-features --exclude fftools --no-run`, `cargo check -p fftools`, and clippy pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on too many arguments in the new `dec3` test helper; the helper now has a local test-only allow and clippy passes.
- `cargo test -p fftools --lib` previously failed because Windows Application Control blocked the rebuilt `target\debug\deps\fftools-649f05dff8861706.exe` with `os error 4551`; `cargo check -p fftools` continues to pass, while executable-launching workspace tests remain subject to the same Windows policy.
- `cargo run -p fate-runner -- run --changed` selected `avformat-mov-demuxer` but failed with the expected no-runnable-FATE-mapping error because upstream media mappings for that component are still not configured; the local `fate-runner` self-test mapping passes.
- `cargo run -p fate-runner -- run --changed` selected the changed MOV and fftools ledger components but failed with the expected no-runnable-FATE-mapping error because upstream media mappings for those components are still not configured; the local `fate-runner` self-test mapping passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a needless borrow in the new ffprobe audio sample-entry v2 test helper; the helper now passes the slice directly and clippy passes.
- `git status --short` before initialization failed because the directory was not a Git repository.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a test literal grouping in `byteio.rs`; the literal was normalized and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a loop-counter pattern and boolean assert style in `bitreader.rs`; both were corrected and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed on a redundant closure in `probe.rs`; it was replaced with the function reference and clippy passed on rerun.
- `rg "read_u64|write_u64" crates/avutil/src crates/avformat/src` failed because ripgrep is not installed in this shell; PowerShell-native file inspection was used instead.
- `cargo test -p avformat mov` initially failed because the malformed `ftyp` fixture truncated the next top-level box before reaching the intended validation; the fixture was narrowed and `cargo test -p avformat mov::tests` passed.
- `cargo test -p avformat mov::tests` was initially blocked by Windows Application Control for the generated test executable; rerunning with the approved `cargo test` prefix passed.
- `cargo test -p avformat mov::tests` initially failed after adding a custom `stsd` fixture because the fixture computed an `mdat` payload offset from the `moov` payload length instead of the boxed `moov` length; the fixture was corrected and the MOV suite passed on rerun.
- `cargo test -p avformat probe mov::tests` failed because `cargo test` accepts only one test-name filter; the probe and MOV filters were rerun as separate commands and passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed after adding offset-signature probe support because of simplify-map-or and explicit-auto-deref suggestions in `probe.rs`; both were corrected and clippy passed on rerun.
- `cargo fmt --all` initially failed after adding the malformed `colr` fixture because the byte literal used an invalid `\1` escape; the fixture was corrected to `\x01` and formatting passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed after adding the `avcC` fixture helper because of `vec_init_then_push`; the helper now uses `vec![..]` and clippy passed on rerun.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed after adding structured `hvcC` parsing because `MovSampleEntryDetails::Video` became a large enum variant; the video details are now boxed and clippy passed on rerun.
- `cargo test -p fftools ffprobe` failed after rebuilding `ffprobe-rs` because Windows Application Control blocked the test-spawned binary; rerunning outside the sandbox did not change the policy result.
- `cargo test -p fftools --test version ffprobe_rs_prints_version_banner` failed for the same Windows Application Control block on the rebuilt `ffprobe-rs` binary.
- `cargo test -p fftools` then failed when Windows Application Control blocked the separate `ffprobe_mov` integration-test executable; the coverage was moved into the `fftools` unit-test binary, which runs successfully in this environment.
- Parallel `cargo test -p fftools ffmpeg` and `cargo test -p fftools option_parser` failed once with Windows file-lock/linker errors against rebuilt test executables; both commands passed when rerun sequentially.
- `cargo test -p fftools option_parser` later ran the relevant option-parser unit tests successfully but then failed when Windows Application Control blocked the unrelated `ffprobe_rs-...exe option_parser` bin test harness with `os error 4551`; `cargo test -p fftools --lib option_parser` validates the focused option-parser coverage without launching that binary.
- `cargo test -p fftools ffmpeg` initially failed after adding the rawvideo output muxer variant because the raw PCM input branch did not explicitly handle `OutputMuxer::RawVideo`; the unsupported arm was added and the focused test passed on rerun.
- `cargo test --workspace --all-features` initially failed after tightening MOV `mdhd` parsing because two `ffmpeg` MOV fixtures still omitted the language/predefined trailer fields; the fixtures were updated and the focused MOV ffmpeg tests plus full workspace suite passed on rerun.
- `cargo test --workspace --all-features` and `cargo test -p fftools --lib` are currently blocked when launching the unchanged `target\debug\deps\fftools-649f05dff8861706.exe`; rerunning `cargo test -p fftools --lib` outside the sandbox produced the same Windows Application Control `os error 4551`. `cargo test --workspace --all-features --exclude fftools` and `cargo test -p avutil --lib` pass for this slice.
- `cargo test --workspace --all-features --exclude fftools` once failed when Windows Application Control blocked a freshly rebuilt `target\debug\deps\avformat-a0c2eebe89aa5944.exe`; rerunning `cargo test -p avformat --lib` and then `cargo test --workspace --all-features --exclude fftools` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` initially failed after adding channel-layout assertions because `ChannelLayout` was imported at `avcodec::pcm` module scope but only used by tests; the import was moved into the test module and clippy passed on rerun.
- `cargo run -p fate-runner -- run --changed` selected `fate-runner` from the current git diff and exited with the intended no-runnable-FATE-mapping error because FATE samples and component command mappings are not configured yet.
- `cargo fuzz --version` failed because the cargo-fuzz subcommand is not installed in this environment.
- `cargo check --manifest-path fuzz/Cargo.toml --bins` initially failed under the restricted sandbox because Cargo needed crates.io index access for `libfuzzer-sys`; rerunning with approved network access resolved and built the cached/downloaded dependency set.
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings` initially failed on a `len() == 0` assertion in `avutil_byteio`; the invariant now compares `ByteWriter::is_empty()` to `as_slice().is_empty()`, and clippy passes.
- `cargo test -p avutil dict options` failed because Cargo accepts only one test-name filter; the dict and options filters were rerun as separate commands and passed.
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings` initially failed on `len() == 0` assertions in `avutil_metadata_options`; the invariants now compare exposed slice emptiness and lengths, and clippy passes.
- `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings` initially failed on a manual `% 4 == 0` multiple check in `avformat_packet_muxers`; it now uses `.is_multiple_of(4)` and clippy passes.
- `cargo check --manifest-path fuzz/Cargo.toml --bins` initially failed after adding `avutil_core_models` because a nested `cursor.next()` call borrowed the fuzz cursor mutably twice; the length byte is now read into a local before payload generation and the fuzz package builds.
- `cargo test -p avutil frame` and `cargo check --manifest-path fuzz/Cargo.toml --bins` initially failed after adding frame line sizes because expected plane sizes were iterated by reference and compared as `&usize`; the comparisons now dereference expected sizes and both commands pass.
- `cargo test -p fftools ffprobe` ran the relevant in-process ffprobe unit tests successfully, then failed when Windows Application Control blocked the rebuilt `ffprobe-rs` bin test harness with `os error 4551`; `cargo test -p fftools --lib ffprobe` and `cargo test -p fftools --lib` pass for this slice.

## Current Focus Component

`fate-runner` is the active infrastructure focus for this turn. The concrete change makes `tests/differential/` files relevant to changed-path selection and adds unit coverage that parses the live differential mapping file against current ledger IDs, including rawvideo oracle mappings and the channel-layout `ffmpeg -layouts` oracle mapping. It deliberately does not claim upstream FATE parity because no sample-based FATE mappings exist yet.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds an ignored oracle inventory test for `ffmpeg -layouts`, comparing the pinned oracle's channel names/descriptions and standard-layout decompositions to the current Rust inventories, and wires that test into the differential mappings plus changed-path component selection. It deliberately does not claim `differential_pass` because the pinned oracle binary is absent locally.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change aligns the parser's numeric-mask, count-suffix, and described-list branches with the pinned FFmpeg `strtoull`/`strtol` shape: leading C whitespace and leading `+` are accepted where the C parsers accept them, and trailing junk after masks, count suffixes, or described lists is no longer hidden by whole-expression trimming. It deliberately does not claim byte-preserving FFmpeg custom-name parity, oracle-vector parity, full `ffmpeg -layouts` inventory coverage, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change aligns FFmpeg-facing channel/layout string parsing with pinned `strcmp` behavior: native channel IDs and `UNK`/`UNSD` require exact uppercase names, `AMBI`/`USR` parser prefixes stay uppercase-only, exact layout names use `ChannelLayout::from_name`, and `ChannelLayoutSpec::parse` stops falling through to the more permissive `ChannelLayout::parse` helper. It deliberately does not claim byte-preserving FFmpeg custom-name parity, oracle-vector parity, full `ffmpeg -layouts` inventory coverage, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change strengthens the explicit `ambisonic <order>[+extra]` layout parser branch so order parsing follows FFmpeg's bounded `strtol` shape: signed-zero `ambisonic -0` is order 0, `ambisonic -0+stereo` and no-conversion `ambisonic +stereo` become zeroth-order ambisonics with stereo extras, and negative nonzero orders remain rejected. It deliberately does not claim byte-preserving FFmpeg custom-name parity, oracle-vector parity, full `ffmpeg -layouts` inventory coverage, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds explicit byte-entry parser helpers for channel layouts: valid UTF-8 byte strings use the same bounded parser as `&str`, non-UTF-8 byte strings return typed `InvalidData`, and NUL-containing byte strings continue to return typed `InvalidArgument`. It deliberately does not claim byte-preserving FFmpeg custom-name parity, oracle-vector parity, full `ffmpeg -layouts` inventory coverage, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change aligns custom-map native-mask and ambisonic-extra retyping with FFmpeg's raw-channel `masked_description` rule: raw IDs `0..62` can form masks even when they are not modeled named native channels, while bit 63, out-of-range user IDs, duplicates, and descending order reject. It deliberately does not claim byte-level/non-UTF-8 parser parity, oracle-vector parity, full `ffmpeg -layouts` inventory coverage, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds bounded direct target-AMBISONIC retyping and bounded CANONICAL retyping on `ChannelLayoutSpec`, both returning `ChannelLayoutRetypeResult` so lossy direct target conversions stay observable. It deliberately does not claim byte-level/non-UTF-8 parser parity, broader raw-channel/custom/ambisonic order retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds bounded lossy target-NATIVE and target-UNSPEC retype results on `ChannelLayoutSpec` with an explicit lossy flag, while preserving strict lossless wrapper methods. It deliberately does not claim target-AMBISONIC lossy retyping, `CANONICAL` flag retyping, byte-level/non-UTF-8 parser parity, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds bounded lossless target-NATIVE and target-UNSPEC conversion helpers on `ChannelLayoutSpec`: already-native/unspecified variants are no-ops, nameless strictly ordered native custom maps reduce to native specs, and nameless all-UNKNOWN custom maps reduce to count-only unspecified specs. It deliberately does not claim lossy retyping, byte-level/non-UTF-8 parser parity, broader canonical/custom/ambisonic order retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds a bounded lossless target-CUSTOM conversion helper on `ChannelLayoutSpec`, using existing channel-index order for native, arbitrary native-mask, and explicit ambisonic layouts, cloning custom maps unchanged, and representing unspecified layouts as UNKNOWN custom maps. It deliberately does not claim byte-level/non-UTF-8 parser parity, broader lossy/custom/native/ambisonic/unspecified retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds lossless canonical ambisonic retyping for nameless custom channel-list maps whose leading AMBI channels form a complete standard-order prefix and whose extra channels are native IDs in strict mask-bit order. It deliberately does not claim byte-level/non-UTF-8 parser parity, broad lossy/custom/native/unspecified retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change completes the current explicit ambisonic lookup surface by adding `index_from_channel`, `index_from_string`, and `channel_from_string` for `AmbisonicChannelLayout` and delegating those through `ChannelLayoutSpec`; native-mask and count-only unspecified specs now expose the same lookup API shape. It deliberately does not claim byte-level/non-UTF-8 parser parity, broad retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds explicit `AmbisonicChannelLayout` storage for bounded `AV_CHANNEL_ORDER_AMBISONIC`-like parser results with no extras or native-mask extras, while retaining Custom-backed AMBI0..AMBI<N> maps for named/custom extras. It deliberately does not claim full escaped token parsing, broad retyping, full `AV_CHANNEL_ORDER_AMBISONIC` comparison/index-string parity, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change makes bounded custom channel lists first-class in `ChannelLayoutSpec`: custom specs can now carry parsed `AV_CHANNEL_ORDER_CUSTOM`-style maps, while canonical nameless native lists still reduce to native or exact-mask specs and all-unknown nameless maps reduce to count-only unspecified specs. It deliberately does not claim full escaped token parsing, explicit `AV_CHANNEL_ORDER_AMBISONIC`, broad retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds `CustomChannelLayout::parse_channel_list` for the bounded FFmpeg `parse_channel_list` helper shape, parsing `CH[@name]+...` strings into custom-order maps without FFmpeg runtime linkage. It deliberately does not yet add a first-class `ChannelLayoutSpec` custom-order result, full escaping/quoting parity, implicit ambisonic layout parsing, broad retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change extends `ChannelLayoutSpec::parse` to preserve arbitrary nameless native channel lists as exact native masks when they do not canonicalize to a modeled named layout, covering forms such as `FL`, `FL+FC`, and `2 channels (FL+FC)`. It deliberately leaves custom `@name` map syntax, implicit ambisonic layout parsing, broad retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution unclaimed.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds `NativeChannelMaskLayout` and threads it through `ChannelLayoutSpec::parse`, so FFmpeg-valid arbitrary nonzero native masks are preserved by exact bitmask instead of being rejected when they do not map to a modeled named layout. It deliberately leaves full custom-map syntax, implicit ambisonic layout parsing, broad retyping, oracle inventory parity, upstream FATE parity, and actual fuzz execution unclaimed.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds bounded `ChannelLayoutSpec::parse` coverage for source-checked count suffixes in `av_channel_layout_from_string()`: native layout names and current native channel expressions, `Nc` native defaults only, `NC`/`N channels` count-only unspecified layouts, and `N channels (<native channel-list>)` count validation. It does not claim numeric-mask parsing, full custom-map parsing, implicit ambisonic layout parsing, broad retyping, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds a bounded Rust representation for FFmpeg's count-only `AV_CHANNEL_ORDER_UNSPEC` layouts through `UnspecifiedChannelLayout` and `ChannelLayoutSpec`, then threads that spec through `AudioFrame` and avformat audio stream parameters. It covers default-count fallback, validation, description, comparison, subset, and index-lookup behavior for count-only layouts without claiming full string parsing, retyping, implicit ambisonic layout semantics, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

Previous `avutil-channel-layout` default-count focus: expanded default layout derivation to follow `av_channel_layout_default` for the modeled source-order `channel_layout_map` subset, returning mono, stereo, 2.1, 4.0, 5.0, 5.1, 6.1, 7.1, 5.1.4, 7.1.4, 9.1.4, 9.1.6, and 22.2 for counts 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, and 24. That slice did not claim full parsing, retyping, implicit ambisonic layout semantics, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

Previous `avutil-channel-layout` lookup focus: added source-shaped current native/custom channel lookup helpers by index, raw channel ID, and string. It follows `av_channel_layout_channel_from_index`, `av_channel_layout_index_from_channel`, `av_channel_layout_index_from_string`, and `av_channel_layout_channel_from_string` for the currently modeled native/custom surfaces: native indices use mask-bit order, native strings resolve through canonical channel names, and custom strings support first-match raw ID, `CH@name`, and `@name` lookup through the existing bounded custom-map subset. It does not claim full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

Previous `avutil-channel-layout` subset focus: added source-shaped current-subset native mask extraction helpers for native layouts and custom maps. It follows `av_channel_layout_subset` for the currently modeled native/custom surfaces by intersecting native layout masks directly and by checking requested native channel IDs for presence in custom maps while ignoring names, duplicate entries, and non-native IDs. It does not claim full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

Previous `avutil-channel-layout` equivalence focus: added source-shaped current-subset layout equivalence helpers for native/native, native/custom, and custom/custom layout pairs. It follows `av_channel_layout_compare` for the then-modeled native/custom surfaces by requiring equal channel counts and equal channel IDs at each index, ignoring custom channel names. It does not claim full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic/unspecified retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds source-shaped custom-map ambisonic order detection and description for complete standard-order `AMBI0..AMBI<N>` prefixes, including optional trailing native/custom channels and typed invalid-argument errors for missing, incomplete, out-of-order, or ACN/index-mismatched ambisonic maps. It does not claim full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, layout comparison, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds source-shaped canonical native-mask detection for `CustomChannelLayout`, including strict native raw-ID ordering, name-sensitive lossless native reduction, native-name description reduction for modeled layouts, and typed errors for duplicate, out-of-order, or non-native custom entries. It does not claim full `av_channel_layout_from_string()` grammar, broad native/custom/ambisonic retyping, `AV_CHANNEL_ORDER_AMBISONIC`, layout comparison, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds a source-shaped `AVChannelCustom` map primitive through `ChannelCustom` and `CustomChannelLayout`: positive counts, default `UNK` entries, `NONE` rejection, 15-byte custom names, duplicate raw IDs, first-match lookup, `CH@name`/`@name` custom-name lookup, and custom-order descriptions. It does not claim full `av_channel_layout_from_string()` grammar, native/custom/ambisonic retyping, `AV_CHANNEL_ORDER_AMBISONIC`, layout comparison, oracle inventory parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds a separate source-shaped `ChannelId` raw ID helper for `AV_CHAN_NONE`, native channels, `AV_CHAN_UNUSED`, `AV_CHAN_UNKNOWN`, ambisonic ACN IDs, and user raw IDs, with exact name/description tests and fuzz-fixture coverage. It does not claim `AVChannelCustom`, `AV_CHANNEL_ORDER_CUSTOM`, `AV_CHANNEL_ORDER_AMBISONIC`, ambisonic order inference, retyping, or layout comparison parity.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds source-checked standalone FFmpeg native channel IDs `SDL`/surround-direct-left, `SDR`/surround-direct-right, `SSL`/side-surround-left, `SSR`/side-surround-right, `TTL`/top-surround-left, and `TTR`/top-surround-right with exact native mask bits, unit tests, and shared fuzz-fixture coverage. It does not claim new named layouts for those channel pairs and keeps unsupported custom expressions rejected. It also does not claim full `ffmpeg -layouts` inventory parity, `AV_CHAN_UNUSED`/`AV_CHAN_UNKNOWN`/ambisonic IDs, custom/native/ambisonic order semantics, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the active infrastructure focus for this turn. The concrete change adds source-checked `DL`/downmix-left, `DR`/downmix-right, `BIL`/binaural-left, `BIR`/binaural-right, `binaural`, and `downmix` from pinned FFmpeg 8.1.1 `channel_layout_map`, preserving their `AV_CH_LAYOUT_BINAURAL` and `AV_CH_LAYOUT_STEREO_DOWNMIX` mappings, extending exact name/mask/parser tests, and expanding the shared `avutil_core_models` fuzz generator. It does not claim full `ffmpeg -layouts` inventory parity, `22.2` inventory, custom/ambisonic orders, upstream FATE parity, or actual fuzz execution.

`avutil-sample-format` is the active infrastructure focus for this turn. The concrete change adds FFmpeg-shaped `av_get_sample_fmt_string` table formatting helpers for the modeled native sample formats, including the fixed header and fixed-width name/depth row strings needed by future Rust-native `-sample_fmts` inventory output. It does not claim pinned libavutil differential parity, upstream FATE parity, broader sample-data conversion routines, or actual fuzz execution.

`avutil-sample-format` is the active infrastructure focus for this turn. The concrete change adds FFmpeg-shaped `av_samples_alloc` / `av_samples_alloc_array_and_samples` owned allocation helpers for contiguous Rust audio buffers, including `BufferRef` ownership, reuse of fill-array plane ranges, requested-versus-effective sample count reporting, source-shaped silence initialization for requested samples, deterministic zeroed padding/tail bytes, mutable plane access, and typed invalid-input errors. It does not claim pinned libavutil differential parity, upstream FATE parity, broader sample-data conversion routines, or actual fuzz execution.

`avutil-sample-format` is the active infrastructure focus for this turn. The concrete change adds FFmpeg-shaped `av_samples_fill_arrays` layout helpers for contiguous Rust audio buffers, including packed one-plane layout, planar line-size-spaced channel ranges, immutable and mutable safe splitting, extra trailing-buffer tolerance, and typed short-buffer errors. It does not claim pinned libavutil differential parity, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, or actual fuzz execution.

`avutil-bitwriter` is the active infrastructure focus for this turn. The concrete change adds checked bit-level truncation and clear/reset support to the bounded MSB-first writer, with unit and fuzz-harness invariant coverage for no-mutation failure behavior and masked partial-byte tails. It does not claim pinned PutBitContext differential parity, upstream FATE parity, or actual fuzz execution.

`avutil-timebase` is the active infrastructure focus for this turn. The latest change extends source-checked `av_add_stable` behavior from positive increments to bounded signed increments: exact negative tick increments subtract, fractional negative increments keep the current timestamp unchanged, and exact-result overflow becomes a typed Rust error. It does not claim pinned differential parity, upstream FATE parity, actual fuzz execution, or out-of-range C edge parity yet.

`avutil-timebase` was the active infrastructure focus for the prior turn. That change added source-checked `av_rescale_delta`-style stateful timestamp conversion on top of the existing FFmpeg timebase constants, rescale helpers, `av_compare_ts`, `av_compare_mod`, and `av_add_stable`. It did not claim pinned differential parity, upstream FATE parity, actual fuzz execution, or exact out-of-range C behavior parity.

`avutil-rational` is the active infrastructure focus for this turn. The latest change adds source-checked `av_q2intfloat`-style conversion through `Rational::to_int_float_bits`, with local unit tests and fuzz-harness invariants for finite IEEE bit vectors, special-value results, negative denominators, and invalid raw negation-overflow inputs. The component remains `implemented`, not `complete`, because pinned FFmpeg differential vectors, upstream FATE parity, and actual fuzz execution are still incomplete.

`avutil-rational` is the active infrastructure focus for this turn. The latest change adds source-checked `av_cmp_q`-style sentinel comparison plus `av_nearer_q`/`av_find_nearest_q_idx`-style nearest-candidate selection over Rust slices, with local unit tests, fuzz-harness build invariants, and local FATE-runner smoke coverage. The component remains `implemented`, not `complete`, because pinned FFmpeg differential vectors, upstream FATE parity, broader rational edge coverage, and actual fuzz execution are still incomplete.

`avutil-error` is the active infrastructure focus for this turn. The latest change adds pinned FFmpeg-defined error-string table helpers and generic unknown-code formatting, with local unit and fuzz-harness invariant coverage. The component remains `implemented`, not `complete`, because platform `AVERROR(errno)` parity, pinned oracle differential vectors, upstream FATE parity, and actual fuzz execution are still incomplete.

`fate-runner` was the active infrastructure focus for this turn. The latest change makes differential mapping rows able to inject mapping-scoped environment variables with placeholder resolution, so rawvideo oracle tests can receive a validated `FFMPEG_ORACLE` path from `--oracle-ffmpeg` instead of manual shell setup. The component remains `scaffolded` because upstream FFmpeg FATE media sample execution and real sample-backed mappings are still not present.

`fftools-ffmpeg-rawvideo-file-output`, `avformat-rawvideo-demuxer`, and `avformat-rawvideo-muxer` are the current focus for turning rawvideo local coverage into oracle-backed differential coverage. The latest concrete change adds ignored `rgb24` and `gbrp10msble` rawvideo file-output oracle tests. The component status remains `implemented`, not `differential_pass` or `complete`, until the pinned FFmpeg 8.1.1 oracle binary is installed and those tests pass.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's MSB-aligned planar 10/12-bit YUV444 and GBR names `yuv444p10msble`, `yuv444p10msbbe`, `yuv444p12msble`, `yuv444p12msbbe`, `gbrp10msble`, `gbrp10msbbe`, `gbrp12msble`, and `gbrp12msbbe` as three full-resolution two-byte-per-sample payload planes with 10 or 12 valid high bits and 30 or 36 logical bpp. It does not claim conversion support, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed 32-bit integer RGB/RGBA names `rgb96le`, `rgb96be`, `rgba128le`, and `rgba128be` as one-payload-plane RGB-class integer formats with no chroma subsampling, 32-bit components, 96/128 logical bpp, twelve/sixteen-byte-per-pixel rawvideo packet sizing, and alpha metadata only on `rgba128*`. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed floating RGBA names `rgbaf16le`, `rgbaf16be`, `rgbaf32le`, and `rgbaf32be` as one-payload-plane RGB-class float formats with alpha metadata, no chroma subsampling, 16/32-bit component descriptors, 64/128 logical bpp, and eight/sixteen-byte-per-pixel rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed floating RGB names `rgbf16le`, `rgbf16be`, `rgbf32le`, and `rgbf32be` as one-payload-plane RGB-class float formats with no alpha, no chroma subsampling, 16/32-bit component descriptors, 48/96 logical bpp, and six/twelve-byte-per-pixel rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's planar floating GBR names `gbrpf16le`, `gbrpf16be`, `gbrpf32le`, and `gbrpf32be` as three-payload-plane RGB-class planar float formats with no alpha, no chroma subsampling, 16/32-bit component descriptors, 48/96 logical bpp, and two/four-byte-per-sample rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed UYYVYY411 name `uyyvyy411` as one packed YUV 4:1:1 payload plane with three 8-bit exposed components, no alpha, log2 chroma `(2,0)`, 12 logical bpp, width-divisible-by-4 validation, and 6-byte-per-4-pixel rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's Bayer CFA names `bayer_bggr8`, `bayer_rggb8`, `bayer_gbrg8`, `bayer_grbg8`, `bayer_bggr16le`, `bayer_bggr16be`, `bayer_rggb16le`, `bayer_rggb16be`, `bayer_gbrg16le`, `bayer_gbrg16be`, `bayer_grbg16le`, and `bayer_grbg16be` as one-payload-plane Bayer formats with RGB-class descriptor metadata, no alpha, no chroma subsampling, 8/16 logical bpp, and one/two-byte-per-pixel rawvideo packet sizing. It does not claim Bayer conversion/demosaic support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed X/V YUV 4:4:4 names `xv30le`, `xv30be`, `xv36le`, `xv36be`, `xv48le`, `xv48be`, `v30xle`, and `v30xbe` as one-payload-plane YUV-class formats with three exposed components, no alpha, no chroma subsampling, 30/36/48 logical bpp, and four/six/eight-byte-per-pixel rawvideo packet sizing that preserves the undefined X lane. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed floating gray+alpha names `yaf16le`, `yaf16be`, `yaf32le`, and `yaf32be` as one-payload-plane grayscale-class formats with two components, alpha and float descriptor metadata, no chroma subsampling, 32/64 logical bpp, and four/eight-byte-per-pixel rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed 8-bit YUV/YUVA 4:4:4 names `vuya`, `vuyx`, `ayuv`, `uyva`, and `vyu444` as one-payload-plane YUV-class formats. `vuya`, `ayuv`, and `uyva` expose four 8-bit components with alpha and four stored bytes per pixel; `vuyx` exposes three components with no alpha, 24 logical bpp, and four stored bytes per pixel; `vyu444` exposes three components, no alpha, and three stored bytes per pixel. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed X2RGB10/X2BGR10 names `x2rgb10le`, `x2rgb10be`, `x2bgr10le`, and `x2bgr10be` as one-payload-plane RGB-class formats with pinned 30-bpp descriptor metadata, three 10-bit exposed color components, endian-specific names, no alpha, no chroma subsampling, and four-byte-per-pixel rawvideo packet sizing that preserves the unused two-bit X lane. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed XYZ12 names `xyz12le` and `xyz12be` as one-payload-plane formats with a distinct XYZ class, pinned 36-bpp descriptor metadata, three 12-bit components stored in six bytes per pixel, endian-specific names, no alpha, and no chroma subsampling. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed AYUV64 names `ayuv64le` and `ayuv64be` as one-payload-plane formats with four 16-bit components, alpha metadata, pinned 64-bpp descriptor metadata, endian-specific names, no chroma subsampling, and eight-byte-per-pixel rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's high-bit packed YUV 4:2:2 names `y210le`, `y210be`, `y212le`, `y212be`, `y216le`, and `y216be` as one-payload-plane formats with pinned descriptor bpp, endian-specific names, even-width validation, and four-byte-per-pixel rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's semi-planar NV YUV 4:2:2 and 4:4:4 names `nv16`, `nv20le`, `nv20be`, `nv24`, and `nv42` as two payload-plane formats with pinned descriptor bpp, UV/VU ordering metadata through distinct names, even-width validation for the 4:2:2 formats, and matching rawvideo packet sizing. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's high-bit-depth planar YUVA family except nonexistent `yuva420p12*` names, using four two-byte-per-sample planes with alpha metadata, pinned descriptor bpp, matching 4:2:0/4:2:2/4:4:4 chroma geometry, and validation for invalid subsampled dimensions. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's 8-bit planar YUVA names `yuva420p`, `yuva422p`, and `yuva444p` as four 8-bit planes with full-resolution alpha, pinned descriptor bpp, log2 chroma `(1,1)`, `(1,0)`, or `(0,0)`, and matching chroma-geometry validation. It does not claim conversion support, high-bit YUVA variants, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's high-bit-depth planar YUV 4:4:0 names `yuv440p10le`, `yuv440p10be`, `yuv440p12le`, and `yuv440p12be` as three two-byte-per-sample planes with pinned descriptor bpp, log2 chroma `(0,1)`, and height-divisible-by-2 validation. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE media parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's 14-bit and 16-bit planar YUV families for 4:2:0, 4:2:2, and 4:4:4 as three two-byte-per-sample planes with pinned descriptor bpp and chroma geometry. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's paletted `pal8` format to the shared pixel model as a one byte-per-pixel raw packet index plane, with `AVPALETTE_COUNT`/`AVPALETTE_SIZE` constants and a paletted descriptor flag. It does not claim full `AVFrame.data[1]` palette side-plane/context propagation, palette interpretation, conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's deprecated full-range planar YUVJ names `yuvj420p`, `yuvj422p`, `yuvj411p`, `yuvj440p`, and `yuvj444p` as distinct `PixelFormat` variants that reuse the corresponding YUV planar geometry. It does not claim color-range conversion, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's 1bpp monochrome bitstream `monow` and `monob` formats to the shared pixel model as one row-padded grayscale bitstream plane with `ceil(width / 8)` bytes per row, one descriptor bit per pixel, and no polarity conversion. It does not claim conversion support, full `AVPixFmtDescriptor` bitstream flag/component-offset parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed 4bpp RGB bitstream `rgb4` and `bgr4` formats to the shared pixel model as one row-padded plane with `ceil(width / 2)` bytes per row, four descriptor bits per pixel, and scalar max component depth 2. It does not claim conversion support, full `AVPixFmtDescriptor` bitstream flag/component-layout parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's byte-packed low-bit-depth RGB `rgb8`, `bgr8`, `rgb4_byte`, and `bgr4_byte` formats to the shared pixel model as one byte-per-pixel packed RGB planes with scalar max component depths of 3 or 2 bits. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's semi-planar YUV 4:2:0 `nv12` and `nv21` formats to the shared pixel model as one full-resolution luma plane plus one full-width half-height interleaved chroma plane with even-width and even-height validation. It does not claim conversion support, full `AVPixFmtDescriptor` component-layout parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed YUV 4:2:2 `yuyv422`, `uyvy422`, and `yvyu422` formats to the shared pixel model as one even-width, two-byte-per-pixel packed YUV plane. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed 16-bit RGB/BGR `rgb565*`, `rgb555*`, `bgr565*`, `bgr555*`, `rgb444*`, and `bgr444*` family to the shared pixel model as one two-byte-per-pixel packed plane. It does not claim conversion support, full per-component `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's packed high-bit-depth grayscale family to the shared pixel model, modeling `gray9le`/`be`, `gray10le`/`be`, `gray12le`/`be`, and `gray14le`/`be` plus `y9*`/`y10*`/`y12*`/`y14*` aliases as one two-byte-per-sample grayscale plane with 9/10/12/14 descriptor bits per pixel. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds FFmpeg's planar GBRA family to the shared pixel model, modeling integer and floating variants as four full-resolution planes with alpha metadata, no chroma subsampling, and one-, two-, or four-byte sample storage. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds planar 16-bit RGB `gbrp16le` and `gbrp16be` to the shared pixel model, modeling them as three full-resolution GBR planes with 16-bit descriptor components, 48 descriptor bits per pixel, and two raw storage bytes per sample in each plane. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds planar 14-bit RGB `gbrp14le` and `gbrp14be` to the shared pixel model, modeling them as three full-resolution GBR planes with 14-bit descriptor components, 42 descriptor bits per pixel, and two raw storage bytes per sample in each plane. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds planar 12-bit RGB `gbrp12le` and `gbrp12be` to the shared pixel model, modeling them as three full-resolution GBR planes with 12-bit descriptor components, 36 descriptor bits per pixel, and two raw storage bytes per sample in each plane. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` remains the current focus, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The latest concrete change adds planar 10-bit RGB `gbrp10le` and `gbrp10be` to the shared pixel model, modeling them as three full-resolution GBR planes with 10-bit descriptor components, 30 descriptor bits per pixel, and two raw storage bytes per sample in each plane. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding planar 9-bit RGB `gbrp9le` and `gbrp9be` to the current shared pixel model, modeling them as three full-resolution GBR planes with 9-bit descriptor components and two raw storage bytes per sample in each plane. It does not claim conversion support, full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding planar 8-bit RGB `gbrp` to the current shared pixel model, modeling it as three full-resolution 8-bit planes with planar RGB metadata and no alpha/float/chroma-subsampling semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding packed floating grayscale formats `grayf16le`, `grayf16be`, `grayf32le`, and `grayf32be` to the current shared pixel model, accepting `yf32le` and `yf32be` aliases for the 32-bit pair, and treating them as single-plane 2-byte or 4-byte floating gray storage with endian-specific raw storage names but no conversion semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding packed 16-bit gray+alpha formats `ya16le` and `ya16be` to the current shared pixel model and treating them as single-plane 4-byte-per-pixel formats with endian-specific raw storage names and alpha metadata but no conversion semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding packed 64-bit RGBA/BGRA formats `rgba64le`, `rgba64be`, `bgra64le`, and `bgra64be` to the current shared pixel model and treating them as single-plane 8-byte-per-pixel formats with endian-specific raw storage names and alpha metadata but no conversion semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding packed gray+alpha `ya8` to the current shared pixel model, accepting `gray8a` and `y400a` as aliases, and treating the format as single-plane 2-byte-per-pixel raw storage with alpha metadata but no conversion semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding packed 48-bit RGB/BGR formats `rgb48le`, `rgb48be`, `bgr48le`, and `bgr48be` to the current shared pixel model and treating them as single-plane 6-byte-per-pixel formats with endian-specific raw storage names but no conversion semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding packed 16-bit grayscale formats `gray16le` and `gray16be` to the current shared pixel model and treating them as single-plane 2-byte-per-pixel formats with endian-specific raw storage names but no conversion semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding packed 32-bit no-alpha RGB padding formats `0rgb`, `rgb0`, `0bgr`, and `bgr0` to the current shared pixel model and treating them as single-plane 4-byte-per-pixel formats without alpha semantics. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding planar 8-bit `yuv440p` to the current shared pixel model and using descriptor chroma metadata for height-divisible-by-2 4:4:0 sizing, frame line sizes, packet splitting, and rawvideo packetization. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding planar 8-bit `yuv410p` to the current shared pixel model and using descriptor chroma metadata for width-and-height-divisible-by-4 4:1:0 sizing, frame line sizes, packet splitting, and rawvideo packetization. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding planar 8-bit `yuv411p` to the current shared pixel model and using descriptor chroma metadata for width-divisible-by-4 4:1:1 sizing, frame line sizes, packet splitting, and rawvideo packetization. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding planar 8-bit `yuv444p` to the current shared pixel model and using descriptor chroma metadata for full-resolution three-plane sizing, frame line sizes, packet splitting, and rawvideo packetization. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, constrained `ffmpeg-rs` input parsing, and fuzz-harness invariant coverage. The concrete change is adding planar 8-bit `yuv422p` to the current shared pixel model and using descriptor chroma metadata for width-only subsampling validation, plane sizing, frame line sizes, packet splitting, and rawvideo packetization. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice. The concrete change is descriptor-style metadata for the already modeled pixel formats: class, component count, bits per component, average bits per pixel, packed bytes per pixel, alpha flag, plane count, and log2 chroma subsampling, with unit and fuzz-harness invariant coverage. It does not claim full `AVPixFmtDescriptor` parity, complete `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, hardware formats, or actual fuzz execution.

`avutil-pixel-format` is the current focus for this slice, with linked rawvideo decoder, demuxer, muxer, and constrained `ffmpeg-rs` input parsing coverage. The concrete change is expanding the current packed RGB-family subset to include `bgr24`, `bgra`, `argb`, and `abgr`, adding shared inventory and packed/alpha metadata, and build-checking the touched fuzz harnesses against the expanded inventory. It does not claim full `AVPixFmtDescriptor` parity, full `ffmpeg -pix_fmts` inventory, pixel conversion, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-channel-layout` is the current focus for this slice. The concrete change is native mask helpers for the modeled common channels, canonical layout masks and channel-expression serialization, narrow named/expression parsing for the six modeled layouts, and unit/fuzz-harness coverage for successful round trips plus invalid expression rejection. It does not claim full `AVChannelLayout` parsing, custom/ambisonic/downmix layouts, pinned `ffmpeg -layouts` differential parity, upstream FATE parity, or actual fuzz execution.

`avutil-dict` is the current focus for this slice. The concrete change is escaped dictionary pair serialization/parsing with configurable separator sets, selected set/match mode application, successful-entry preservation on later malformed tokens, separator/token validation, local unit tests, and fuzz-harness invariant coverage. It does not claim pinned `AVDictionary` differential parity, upstream FATE parity, or actual fuzz execution.

`avutil-options` is the current focus for this slice. The concrete change is parent-mediated child option access and mutation with typed/string setters, child range lookup, mutable child option-set access, root/child namespace independence, and no-mutation error coverage. It does not claim full `AVOption` API parity, CLI option-ordering parity, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`avutil-bitreader` and `avutil-bitwriter` are the current focus for this slice. The concrete change is checked reader bit-position seeking plus writer aligned-byte appends, with local unit and fuzz-harness invariant coverage. It does not claim pinned GetBitContext/PutBitContext differential parity, upstream FATE media parity, or actual fuzz execution.

`avutil-byteio` is the current focus for this slice. The concrete change is checked reader repositioning plus writer patch/truncate support for bounded byte streams, with local unit and fuzz-harness invariant coverage. It does not claim pinned AVIO/GetByte differential parity, upstream FATE media parity, or actual fuzz execution.

`avutil-hash` is the current focus for this slice, with linked `avformat-hash-muxer` and `fftools-ffmpeg-hash-output` coverage. The concrete change is SHA-1/SHA-160 digest support across the shared hash primitive, hash muxer state, CLI `-hash` algorithm parsing, and packet-muxer fuzz invariants. It does not claim exact FFmpeg hash muxer output semantics, pinned oracle parity, upstream FATE parity, or actual fuzz execution.

`fftools-option-parser` is the current focus for this slice. The concrete change is repeat-summary compression for process-level CLI diagnostics by routing the formatter through the shared `avutil::Logger`, with tests proving repeated diagnostics compress only when the parsed `repeat` flag is active. It does not claim media-progress stderr, byte-identical upstream repeat/timestamp/color formatting, pinned oracle parity, or upstream FATE parity.

`fftools-option-parser` is the current focus for this slice. The concrete change is deterministic terminal/non-terminal color coverage for process-level `ffmpeg-rs`/`ffprobe-rs` error stderr, proving that the runtime shared color resolver applies terminal-derived ANSI coloring and preserves forced no-color precedence. It does not claim media-progress stderr, repeat-summary stderr, byte-identical upstream timestamp/color formatting, pinned oracle parity, or upstream FATE parity.

`avutil-logging` is the current focus for this slice. The concrete change is terminal-aware color resolution for `LogColorMode::from_ffmpeg_env`, preserving FFmpeg forced no-color/color environment precedence and enabling color for terminal stderr when no force variable is set. It does not claim byte-identical upstream color policy or line formatting, C ABI callback parity, local-time parity, CLI media-progress/repeat stderr parity, pinned oracle parity, or upstream FATE parity yet.

`fftools-option-parser` is the current focus for this slice. The concrete change is forced-color environment integration for process-level `ffmpeg-rs`/`ffprobe-rs` error stderr using the existing shared `avutil::LogFormatOptions` resolver. It does not claim terminal color auto-detection, media-progress stderr, repeat-summary stderr, byte-identical upstream formatting, pinned oracle parity, or upstream FATE parity.

`avutil-logging` is the current focus for this slice. The concrete change is forced color environment-variable resolution for `AV_LOG_FORCE_NOCOLOR`/`AV_LOG_FORCE_COLOR`, plus unit and fuzz-harness invariant coverage. It does not claim terminal color auto-detection, byte-identical upstream color formatting, C ABI callback parity, local-time parity, CLI stderr parity, pinned oracle parity, or upstream FATE parity yet.

`fftools-option-parser` remains the current focus for this slice. The concrete change is timestamp-flag integration for process-level `ffmpeg-rs`/`ffprobe-rs` error stderr using the existing shared loglevel parser model and `avutil::LogTimestamp`/`LogRecord` formatter. It does not claim media-progress stderr, repeat-summary stderr, byte-identical upstream timestamp formatting, local-time parity, pinned oracle parity, or upstream FATE parity.

`fftools-option-parser` is the current focus for this slice. The concrete change is CLI stderr integration for process-level `ffmpeg-rs`/`ffprobe-rs` errors using the existing shared loglevel parser model and `avutil::LogRecord` formatter. It does not claim full FFmpeg stderr/progress behavior, timestamp/repeat flag parity, byte-identical upstream formatting, pinned oracle parity, or upstream FATE parity.

`avutil-logging` is the current focus for this slice. The concrete change is explicit opt-in ANSI color formatting through `LogFormatOptions`/`LogColorMode`, with record, logger, global-helper, repeat-summary, and fuzz-harness coverage. It does not claim byte-identical upstream color formatting, terminal/env color auto-detection, byte-identical `av_log`, CLI stderr, C ABI callback parity, local-time parity, pinned oracle parity, or upstream FATE parity yet.

`avutil-options` is the current focus for this slice. The concrete change is rational option kind/value support with positive-denominator range validation, parsed `num/den` and integer text support, and unit/fuzz-harness invariant coverage. It does not claim full `AVOption` API parity, CLI option-ordering parity, pinned oracle parity, or upstream FATE parity yet.

`avutil-dict` is the current focus for this slice. The concrete change is ordered exact and prefix matching iterators over the existing metadata entries, with local unit and fuzz-harness invariant coverage. It does not claim pinned `AVDictionary` differential parity or upstream FATE parity yet.

`avutil-byteio` is the current focus for this slice. The concrete change is non-advancing byte lookahead for exact slices and endian-aware signed/unsigned integer widths, with local unit and fuzz-harness invariant coverage. It does not claim pinned AVIO/GetByte compatibility vectors or upstream FATE parity yet.

`avutil-timebase` is the current focus for this slice. The concrete change adds source-checked `av_add_stable`-style nonnegative timestamp increments on top of the existing FFmpeg timebase constants, rescale helpers, `av_compare_ts`, and `av_compare_mod`. It does not claim pinned differential parity, upstream FATE parity, `av_rescale_delta`, or negative-increment parity yet.

`avutil-rational` is the current focus for this slice. The concrete change is limited rational reduction and double conversion parity scaffolding: `reduce_i64`, `from_f64_limited`, and `to_f64`, with unit and fuzz-harness invariant coverage. It does not claim exact pinned `av_reduce`/`av_d2q` differential parity yet.

`avutil-error` is the current focus for this slice. The concrete change is FFmpeg-style error-code metadata for documented tag-based `AVERROR_*` constants and constructor/accessor coverage; it does not claim full platform `AVERROR(errno)` or `av_strerror` parity.

`avutil-channel-layout` is the current focus for this slice. The concrete change is a source-checked expansion of the custom channel-list tokenizer: `CustomChannelLayout::parse_channel_list` and `ChannelLayoutSpec::parse` now cover the bounded FFmpeg `av_opt_get_key_value`/`av_get_token` behavior for whitespace, implicit keys, quoted token text, escaped separators, repeated `@` inside values, and trailing `+` separators.

This slice does not mark channel layout handling complete. The broader goal remains blocked on missing pinned `ffmpeg -layouts` inventory comparison, remaining FFmpeg default-layout count table modeling beyond the currently modeled first-map entries for counts 1, 2, 3, 4, 5, 6, 7, and 8, special `AV_CHAN_UNUSED`/`AV_CHAN_UNKNOWN`/ambisonic IDs, custom/native/ambisonic order semantics, upstream FATE media mappings/samples, actual local fuzz execution, and many incomplete FFmpeg surfaces.

## Next 3 Concrete Actions

1. Configure or build the pinned FFmpeg 8.1.1 oracle binary, then run the rawvideo and channel-layout rows from `tests/differential/mappings.txt` through `fate-runner`.
2. If no oracle is available, continue `fate-runner` by adding the next FATE infrastructure slice: sample-root discovery/reporting or a real sample-backed mapping scaffold that remains skipped/blocked until samples exist.
3. Continue `avutil-channel-layout` only after the oracle/FATE infrastructure path is exhausted, focusing on deeper `AV_CHANNEL_ORDER_AMBISONIC` semantics or byte-preserving non-UTF-8 custom-name parity.

## Known Blockers

- `fate-runner` remains `scaffolded` because upstream FATE sample-based media mappings are not configured and no local FATE samples path exists. The current update only strengthens changed-path and mapping-file validation for local/differential runner wiring.

- Latest `avutil-channel-layout` oracle coverage now has an ignored `ffmpeg -layouts` differential harness and mapping, but real execution is blocked because no pinned FFmpeg 8.1.1 binary exists at `FFMPEG_ORACLE` or `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)`. The harness compiles by default and fails explicitly when run without the oracle instead of silently passing.

- Latest `avutil-channel-layout` parser-facing whitespace coverage now matches the source-checked leading-whitespace/full-consumption shape for numeric masks, `Nc`/`NC`, `N channels`, and described channel lists, while preserving channel-list tokenizer whitespace trimming. Remaining blockers are byte-preserving non-UTF-8 custom-name parity if required by the selected API surface, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, deeper `AV_CHANNEL_ORDER_AMBISONIC` semantics beyond the current bounded lookup/retype surface, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` parser-facing string coverage now rejects case-mismatched channel/layout forms such as `fl+fr`, `unk+unk`, `ambi0`, `usr0`, `STEREO`, and `ambisonic 1+STEREO`, while preserving exact FFmpeg uppercase channel IDs and exact lowercase standard layout names. Remaining blockers are byte-preserving non-UTF-8 custom-name parity if required by the selected API surface, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, deeper `AV_CHANNEL_ORDER_AMBISONIC` semantics beyond the current bounded lookup/retype surface, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` explicit ambisonic parser coverage now follows the bounded FFmpeg `strtol` order-prefix shape for signed-zero and no-conversion extras: `ambisonic -0`, `ambisonic +stereo`, and `ambisonic -0+stereo` resolve to zeroth-order ambisonic layouts while negative nonzero orders reject. Remaining blockers are byte-preserving non-UTF-8 custom-name parity if required by the selected API surface, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, deeper `AV_CHANNEL_ORDER_AMBISONIC` semantics beyond the current bounded lookup/retype surface, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` byte-entry parser coverage makes the Rust boundary explicit for valid UTF-8 byte inputs and typed non-UTF-8 rejection. Remaining blockers are byte-preserving non-UTF-8 custom-name parity if required by the selected API surface, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, full `AV_CHANNEL_ORDER_AMBISONIC` order semantics, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` retype coverage now includes FFmpeg-shaped `masked_description` raw-ID handling for native-mask and ambisonic-extra reductions: nameless raw IDs `0..62` such as `USR45+USR46` reduce to exact native masks, while bit 63, out-of-range user IDs, duplicates, and descending raw-ID order reject. Remaining blockers are byte-level/non-UTF-8 tokenizer calibration, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, full `AV_CHANNEL_ORDER_AMBISONIC` order semantics, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` retype coverage now covers the bounded target-CUSTOM path plus target-NATIVE, target-AMBISONIC, target-UNSPEC, and CANONICAL result paths for the current modeled variants, including explicit lossy results for direct name-dropping or identity-dropping conversions when allowed. Remaining blockers are broader raw-channel/custom/ambisonic retyping beyond the current modeled subset, byte-level/non-UTF-8 tokenizer calibration, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` retype coverage now covers the bounded target-CUSTOM path plus target-NATIVE and target-UNSPEC result paths, including explicit lossy results for name-dropping native retypes and identity-dropping unspecified retypes when allowed. Remaining blockers are target-AMBISONIC lossy retype behavior, `CANONICAL` flag behavior, broader custom/ambisonic retyping beyond the current parser path, byte-level/non-UTF-8 tokenizer calibration, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` ambisonic retype coverage now covers the bounded lossless canonical path for nameless complete AMBI-prefix custom maps with strictly ordered native extras. Remaining blockers are byte-level/non-UTF-8 tokenizer calibration, broader lossy and unspecified/custom/native/ambisonic retyping, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` ambisonic lookup coverage now covers the bounded explicit `AV_CHANNEL_ORDER_AMBISONIC` order/native-extra-mask surface through `AmbisonicChannelLayout` and `ChannelLayoutSpec`, including AMBI-leading indexes, native-extra mask-bit ordering, canonical string lookup, and typed invalid lookup failures. Remaining blockers are byte-level/non-UTF-8 tokenizer calibration, broader custom/native/ambisonic/unspecified retyping, oracle-vector calibration, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` parser coverage now covers bounded `av_opt_get_key_value`/`av_get_token` channel-list behavior for Rust UTF-8 strings, including escaped separators, quoted tokens, custom names containing `@`, whitespace trimming, and trailing `+` separators. Byte-level/non-UTF-8 edge calibration, broader retyping, full ambisonic comparison/index-string semantics, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers.

- Latest `avutil-channel-layout` custom parser coverage now has first-class `ChannelLayoutSpec::Custom` representation for bounded `CH[@name]+...` parsed maps and no longer drops those parses to the side helper. It remains deliberately limited because the Rust parser still lacks full `av_opt_get_key_value` escaping/quoting parity, explicit `AV_CHANNEL_ORDER_AMBISONIC` parsing and storage, broad lossy/lossless retyping behavior, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution.

- Latest `avutil-channel-layout` custom parser coverage now has a bounded `CustomChannelLayout::parse_channel_list` helper for `CH[@name]+...` strings, including optional names and raw channel IDs. It remains deliberately limited because `ChannelLayoutSpec::parse` still cannot return `AV_CHANNEL_ORDER_CUSTOM`, the helper does not implement full `av_opt_get_key_value` escaping/quoting parity, and broad native/custom/ambisonic retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers.

- Latest `avutil-channel-layout` parser coverage now preserves arbitrary nonzero native masks from both numeric masks and nameless native channel lists, but remains deliberately limited to current native names/channel expressions, numeric native masks, and count suffixes. Full custom `@name` map syntax, implicit/broad ambisonic parsing, broader retyping, full `ffmpeg -layouts` inventory comparison, upstream FATE parity, and actual fuzz execution remain blockers.

- Latest `avutil-channel-layout` default-count coverage now follows the modeled source-order `channel_layout_map` entries, including counts 10, 12, 14, 16, and 24, and `ChannelLayoutSpec` now preserves unmodeled positive counts as `AV_CHANNEL_ORDER_UNSPEC`-style count-only layouts. Remaining blockers are full string parsing/retyping for unspecified layouts, broad native/custom/ambisonic retyping, implicit `AV_CHANNEL_ORDER_AMBISONIC`, oracle inventory comparison, upstream FATE parity, and actual fuzz execution.

- On this Windows environment, the generated `avformat` test executable under `target-codex` was blocked by Application Control. The same focused test and FATE-runner mapping passed through Cargo's default `target` directory, so use the default target directory for avformat execution if this policy recurs.

- Latest `avutil-channel-layout` lookup coverage is limited to current native/custom surfaces and deliberately omits implicit `AV_CHANNEL_ORDER_AMBISONIC` index layout behavior and unspecified-order lookup because the Rust model does not yet expose those layout orders as first-class variants.

- Latest `avutil-channel-layout` subset-mask coverage is limited to current native/custom surfaces and deliberately omits implicit `AV_CHANNEL_ORDER_AMBISONIC` mask storage because the Rust model does not yet expose that layout order as a first-class `ChannelLayout` variant.

- Latest `avutil-channel-layout` comparison coverage now includes native, custom, and count-only unspecified layout specs, but it still deliberately omits implicit `AV_CHANNEL_ORDER_AMBISONIC` mask comparison and broad retyping.

- Latest `avutil-channel-layout` coverage adds bounded custom-map ambisonic order detection/description for complete standard-order `AMBI0..AMBI<N>` prefixes with optional trailing channels, but it still has no implicit `AV_CHANNEL_ORDER_AMBISONIC` layout model, no full retyping/comparison semantics, no full parser grammar, no pinned `ffmpeg -layouts` oracle inventory comparison, no upstream FATE parity, and no actual cargo-fuzz execution.

- `avutil-channel-layout` now covers local FFmpeg-shaped channel names and native mask helpers for the current subset, including `FLC`, `FRC`, `BC`, `TFL`, `TFC`, `TFR`, `TBL`, `TBC`, `TBR`, `TC`, `DL`, `DR`, `WL`, `WR`, `SDL`, `SDR`, `LFE2`, `TSL`, `TSR`, `BFC`, `BFL`, `BFR`, `SSL`, `SSR`, `TTL`, `TTR`, `BIL`, and `BIR`, plus named `mono`, `stereo`, `2.1`, `3.0`, `3.0(back)`, `4.0`, `quad`, `quad(side)`, `3.1`, `5.0`, `5.0(side)`, `4.1`, `5.1`, `5.1(side)`, `6.0`, `6.0(front)`, `hexagonal`, `6.1`, `6.1(back)`, `6.1(front)`, `7.0`, `7.0(front)`, `7.1`, `7.1(wide)`, `7.1(wide-side)`, `5.1.2`, `5.1.2(back)`, `octagonal`, `cube`, `5.1.4`, `7.1.2`, `7.1.4`, `7.2.3`, `9.1.4`, `9.1.6`, `hexadecagonal`, `binaural`, `downmix`, and `22.2` layouts. `ChannelId` now covers raw native, none, unused, unknown, ambisonic ACN, and user channel IDs. `ChannelCustom` and `CustomChannelLayout` now cover a bounded custom-map subset with default unknown entries, invalid-entry rejection, custom names, duplicate raw IDs, first-match lookup, custom-order descriptions, and lossless native canonicalization for nameless strictly ordered custom maps that reduce to a modeled native layout. It models FFmpeg default-layout count selection for currently covered first-map entries for counts 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, and 24, but still has no `AV_CHANNEL_ORDER_UNSPEC` fallback representation for unmodeled counts, no full pinned `ffmpeg -layouts` inventory comparison, no full `av_channel_layout_from_string()` grammar, no broad native/custom/ambisonic retyping, no `AV_CHANNEL_ORDER_AMBISONIC` layout semantics, no layout comparison parity, no upstream FATE parity, and no actual cargo-fuzz execution.

- `avutil-sample-format` now covers local FFmpeg-shaped sample format names, `av_get_sample_fmt_string`-style table row formatting, planar/packed metadata, basic payload sizing, sample-buffer layout math, fill-array plane layout/splitting, owned contiguous allocation, silence-fill byte/range behavior, and copy byte-range behavior, but it still has no pinned `ffmpeg -sample_fmts`/libavutil differential vector harness, no upstream FATE media parity, no actual cargo-fuzz execution, and no broader sample-data conversion routine parity.

- `avutil-bitwriter` now covers local checked truncation, clear/reset, aligned-byte appends, signed/unsigned bit writes, and Exp-Golomb writes, but it still has no pinned PutBitContext differential vector harness, no upstream FATE media parity, and no actual cargo-fuzz execution.

- `avutil-timebase` now covers source-checked rescale, compare, `av_rescale_delta`, and bounded signed `av_add_stable` behavior locally, but it still has no pinned FFmpeg differential vector harness, no upstream FATE media parity, no actual cargo-fuzz execution, and no calibration for exact out-of-range C behavior.

- `avutil-rational` now covers source-checked `av_q2intfloat` and `av_gcd_q`-style behavior locally, but it still has no pinned FFmpeg differential vector harness. Actual cargo-fuzz execution is still unavailable, and upstream FATE has no media-backed rational coverage mapping in this workspace.

- `avutil-rational` now covers source-checked `av_cmp_q`, `av_nearer_q`, and `av_find_nearest_q_idx`-style behavior locally, but has no pinned FFmpeg differential vector harness yet. Actual cargo-fuzz execution is still unavailable, and upstream FATE has no media-backed rational coverage mapping in this workspace.

- `avutil-error` now covers the pinned FFmpeg-defined `AVERROR_LIST` string table, but platform `AVERROR(errno)` descriptions are still unresolved because they depend on platform errno values and C library `strerror_r` behavior. No pinned oracle binary is available for differential checks, and actual cargo-fuzz execution is still unavailable.

- Rawvideo oracle differentials are now wired through `tests/differential/mappings.txt` with `env:FFMPEG_ORACLE={oracle_ffmpeg}`, but cannot run to parity in this workspace because the pinned FFmpeg 8.1.1 oracle binary is missing. The mapping prerequisite check intentionally fails without `--oracle-ffmpeg <path>`, and the ignored test harness fails before comparison until `FFMPEG_ORACLE` or `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)` is available.

- Current MSB-aligned planar YUV444/GBR validation has no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through `target-codex`; main and fuzz-package clippy passed; main and fuzz-package check passed; changed-path and single-component FATE dry-runs passed. The prior direct repeated-`--component` FATE dry-run limitation is resolved in the current runner. The remaining blockers are pinned oracle differentials, upstream FATE media parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed 32-bit integer RGB/RGBA validation has no remaining code/test assertion failures. Focused avutil, avformat, avcodec FATE, and fftools target-cache tests passed; format check, workspace clippy, fuzz-package check/clippy, changed-path FATE dry-run, and directly affected local FATE component mappings for avutil-frame, avcodec-rawvideo, and avformat rawvideo also passed. Fresh target-dir avcodec unit-test launches and the broad default-target fftools FATE mapping were blocked before execution by Windows Application Control. Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed floating RGBA validation has no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through `target-codex`; format check, workspace clippy, fuzz-package check/clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` also passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed floating RGB validation has no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through `target-codex`; format check, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` also passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current planar floating GBR validation has no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through `target-codex`; format check, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` also passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed UYYVYY411 validation has no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through accepted target caches; format check, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` also passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current Bayer CFA validation has no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through accepted target caches; format check, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` also passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, Bayer conversion/demosaic behavior, hardware formats, and actual fuzz execution.

- Current packed X/V YUV 4:4:4 validation has no remaining code/test assertion failures. Focused avutil, avcodec, avformat, and fftools tests passed through accepted target caches; format check, workspace clippy, fuzz-package clippy, changed-path FATE dry-run, and `git diff --check` also passed. Windows Application Control blocked some freshly built test executables in other target caches, including the broad workspace library sweep at unrelated `avdevice` and the actual changed-path FATE run at the first avutil mapping. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed floating gray+alpha YAF validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed 8-bit YUV/YUVA validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed X2RGB10/X2BGR10 validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed XYZ12 validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current packed AYUV64 validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current high-bit semi-planar P-family validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current semi-planar NV YUV 4:2:2/4:4:4 validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current high-bit-depth planar YUVA validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, and local changed-path FATE mappings passed through `target-codex`; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution. No `yuva420p12*` variants are tracked because pinned FFmpeg 8.1.1 does not define them.

- Current `yuva420p` / `yuva422p` / `yuva444p` planar YUVA validation has no remaining code/test assertion failures. Focused tests, fuzz build/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings, and `git diff --check` passed through `target-codex`. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, high-bit YUVA variants, hardware formats, and actual fuzz execution.

- Current `yuv440p10*` / `yuv440p12*` planar YUV 4:4:0 validation has no remaining code/test assertion failures. Focused tests, clippy, workspace library tests, direct local FATE mappings, and broad local `fate-runner run --changed` passed through accepted target directories. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.

- Current 14-bit and 16-bit planar YUV validation has no remaining code/test assertion failures. Broad local FATE `run --changed` execution hit Windows Application Control blocks on freshly built child test executables, but dry-run changed selection and the directly affected mapped component tests passed through accepted target directories. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. The remaining blockers are oracle differentials, upstream FATE media parity, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution.
- Current paletted `pal8` validation has no remaining code/test assertion failures. Fresh target-dir test executables were blocked by Windows Application Control, but the same focused avutil, avcodec, avformat, fftools, and local FATE-runner component coverage passed through accepted target directories. Full `AVFrame.data[1]` palette side-plane allocation, decoder palette context propagation, palette side-data behavior, oracle differentials, upstream FATE media coverage, full `AVPixFmtDescriptor` parity, and actual fuzz execution remain incomplete.
- Current deprecated full-range planar YUVJ validation has no remaining code/test assertion failures. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning. No pinned FFmpeg oracle binary exists in the workspace for differential vectors, upstream FATE media parity is still absent, and actual fuzz execution is still blocked by the missing cargo-fuzz subcommand.
- Current 1bpp monochrome bitstream validation has no remaining code/test assertion failures after the MSRV-compatible row-size helper fix. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning.
- Current packed 4bpp RGB bitstream validation has no remaining code/test assertion failures and did not hit a fresh Windows Application Control block. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning.
- Current byte-packed low-bit-depth RGB validation has no remaining code/test assertion failures. Default-target focused avutil and avcodec test launches were blocked by Windows Application Control, but the same tests passed through `target-codex`; subsequent focused tests, fuzz build checks, clippy gates, local FATE mappings, and local `run --changed` passed. Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning.
- Current semi-planar YUV 4:2:0 validation has no remaining code/test assertion failures. Two alternate target-dir attempts for `cargo test -p avutil pixel` were blocked before execution by Windows Application Control, but the same focused test passed through the default target cache; the rest of the focused tests, fuzz build checks, clippy gates, local FATE mappings, and local `run --changed` passed. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning.
- Current packed YUV 4:2:2 validation has no remaining code/test assertion failures and did not hit a fresh Windows Application Control block. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning.
- Current packed 16-bit RGB/BGR validation has no remaining code/test assertion failures and did not hit a fresh Windows Application Control block. `git diff --check` reported CRLF warnings only; other commands in this slice reported only the usual `could not canonicalize path C:\Users\trevo` cargo warning.
- Current high-bit-depth grayscale validation has no remaining code/test assertion failures and did not hit a fresh Windows Application Control block. `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning.
- Current planar GBRA validation has no remaining code/test assertion failures and did not hit a fresh Windows Application Control block. `rg` is unavailable in this PowerShell session, so descriptor/code searches used `Select-String`; `git diff --check` reported CRLF warnings only; Cargo commands reported only the usual `could not canonicalize path C:\Users\trevo` warning.
- Current `gbrp16le`/`gbrp16be` validation has no remaining code/test assertion failures and did not hit a fresh Windows Application Control block. `git diff --check` reported CRLF warnings only; other commands in this slice reported only the usual `could not canonicalize path C:\Users\trevo` cargo warning.
- Current `gbrp14le`/`gbrp14be` validation has no remaining code/test assertion failures. Default-target avutil focused tests and default-target `run --changed` were blocked by Windows Application Control, but the same tests passed through `target-codex`; the target-codex fftools focused test was blocked, but the same test passed through the default target cache. `git diff --check` reported CRLF warnings only.
- Current `gbrp12le`/`gbrp12be` validation has no remaining code/test failures. `git diff --check` reported CRLF warnings only.
- Current `gbrp10le`/`gbrp10be` validation hit Windows Application Control only for `$env:CARGO_TARGET_DIR='target-codex'; cargo test -p avutil pixel`; rerunning the same focused avutil test from the default Cargo target directory passed, and the focused gate plus local FATE mappings passed. Historical broad-test executable policy blocks remain relevant for future slices and are kept below.
- Previous `gbrp` validation hit Windows Application Control when trying to run one broad `run --changed` pass and a single-target full workspace test suite. The focused tests, affected fuzz build checks, workspace clippy, and local component FATE mappings passed through accepted target caches; the broad failures are policy blocks on generated executables, not Rust assertion failures.
- No pinned FFmpeg 8.1.1 oracle binary exists at `third_party/ffmpeg-oracle/build/bin/ffmpeg`, so oracle snapshots and differential tests have not been generated.
- Upstream FATE samples and media target mappings are not configured. `tests/fate/mappings.txt` currently contains local `fate-runner`, repo runtime guard, avutil unit-test, avcodec decoder unit-test, MOV demuxer, fftools version, fftools hide-banner, fftools option-parser, fftools CLI logging, fftools I/O-plan, shared ffmpeg unit-test, and shared ffprobe unit-test smoke mappings only. The runner has `mappings`, `--check-prereqs`, `--samples`, `--oracle-ffmpeg`, and `--dry-run` support for future mappings.
- The `cargo fuzz` subcommand is not installed in this environment. The fuzz package can be checked with `cargo check --manifest-path fuzz/Cargo.toml --bins` and `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings`, but actual fuzz execution requires installing cargo-fuzz.
- `./xtask quick` cannot be a file command while `xtask/` is a crate directory on this filesystem; use `cargo run -p xtask -- quick`.
- Windows Application Control intermittently blocks freshly built child executables and separate integration-test executables. During recent packet slices it blocked focused `avutil` and `fftools` unit-test executables in multiple target directories; `target-avutil-opaque-ref-test` and `target-avutil-timebase-test` have launched the same focused packet tests successfully, and the current packet side-data slices validate through `target-avutil-timebase-test`. During the dict iterator slice it blocked the freshly built `target-avutil-dict-iter-test` `fate-runner.exe`; rerunning the same local FATE mapping through the default `target` cache passed. The current ffprobe MOV command-path coverage is kept in the `fftools` unit-test binary instead of a process-spawn integration test.

## Summary Of Latest Commit Or Changes

Latest slice: strengthened `fate-runner` coverage for differential mappings. `tests/differential/` files now map to `fate-runner` in changed-path analysis, and new unit tests verify both that selection rule and that the live `tests/differential/mappings.txt` file parses against current ledger IDs, including rawvideo oracle rows and the channel-layout `ffmpeg -layouts` oracle row. Docs and the ledger record this as runner infrastructure coverage only. Validation passed with focused `fate-runner` tests, mapping listing, changed dry-run, differential mapping dry-run, `fate-runner` clippy, formatting, and diff checks; some freshly rebuilt default/target-codex executables were blocked by Windows Application Control and passed through fresh target directories.

Correction for latest slice after adding `fate-runner` regression coverage: full `run --changed` execution is blocked by Windows Application Control on rebuilt test executables in this environment, but the direct equivalent checks pass (`fate-runner` self-test through `target-codex`, default-target avutil channel-layout component mapping, and the ignored oracle-test compile). `run --changed --dry-run` also passes and selects the expected components.

Latest slice: added an ignored `ffmpeg -layouts` oracle harness for `avutil-channel-layout`. `crates/avutil/tests/channel_layout_oracle.rs` resolves `FFMPEG_ORACLE` or the standard `third_party/ffmpeg-oracle/build/bin/ffmpeg(.exe)` path, runs `-hide_banner -layouts`, parses the oracle's individual-channel and standard-layout tables, and compares them exactly against the current Rust `Channel::ALL` and `ChannelLayout::known_layouts()` inventories. The harness is wired into `tests/differential/mappings.txt`, documented in the differential/oracle docs, and `fate-runner` now maps changes to that integration test back to `avutil-channel-layout`. Validation passed with the ignored test compile, focused channel-layout tests, differential mapping listing, local and changed FATE-runner mappings, `fate-runner` tests, avutil/fate-runner clippy, format check, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because the pinned oracle binary is absent and the new ignored oracle test has not run.

Latest slice: tightened FFmpeg-facing channel-layout parsing for numeric/count/described-list whitespace. `ChannelLayoutSpec::parse` now passes raw input into the numeric-mask, count-suffix, described-list, and channel-list branches instead of trimming the whole expression first; `parse_numeric_channel_mask` skips only leading C numeric whitespace; and count parsing now uses a bounded `strtol`-shaped decimal prefix helper with optional leading `+`. Unit tests and `avutil_core_models` fixtures cover accepted leading-whitespace forms (` 0x3`, ` 2c`, `+2C`, ` +2 channels (FL+FR)`) and rejected trailing-junk forms (`0x3 `, `3 `, `2c `, `2C `, `2 channels `, `2 channels (FL+FR) `). Docs and the ledger record this as bounded source-checked parser parity only. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: tightened FFmpeg-facing channel/layout string parsing to source-checked case-sensitive matching. `ChannelId::from_ffmpeg_string` now uses exact native short names plus exact `UNK`/`UNSD`/uppercase `AMBI`/`USR`; `ChannelLayoutSpec::parse` accepts exact layout names through `ChannelLayout::from_name` and stops falling back to the ergonomic case-insensitive `ChannelLayout::parse` helper. Unit tests and `avutil_core_models` fixtures cover rejection of `fl+fr`, `unk+unk`, `ambi0`, `usr0`, `STEREO`, `stereo `, and `ambisonic 1+STEREO`; docs and the ledger record this as bounded parser parity only. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added explicit parser coverage for the FFmpeg `strtol`-shaped `ambisonic <order>[+extra]` branch. `ChannelLayoutSpec::parse` now accepts signed-zero `ambisonic -0`, signed-zero with extras `ambisonic -0+stereo`, and the no-conversion extra form `ambisonic +stereo` as zeroth-order ambisonics with the expected native extras, while negative nonzero orders still reject. The deterministic `avutil_core_models` fuzz fixture mirrors those cases, and docs plus the ledger record the behavior as bounded local parser coverage. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added explicit byte-entry channel-layout parsers. `CustomChannelLayout::parse_channel_list_bytes` and `ChannelLayoutSpec::parse_bytes` now accept valid UTF-8 byte strings through the existing bounded parser and reject non-UTF-8 inputs with typed `InvalidData` errors instead of lossy conversion; NUL-containing byte strings continue to return typed `InvalidArgument` via the existing parser. Unit tests and deterministic `avutil_core_models` fuzz fixtures cover valid byte/string equivalence, described-list bytes, raw-ID/ambisonic byte parses, non-UTF-8 channel IDs/custom names, and NUL byte rejection. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: aligned channel-layout custom native-mask and ambisonic-extra retyping with FFmpeg's source-checked `masked_description` raw-channel rule. `CustomChannelLayout::canonical_native_mask` now builds mask bits from raw `ChannelId` values `0..62`, so `USR45+USR46` becomes an exact `NativeMask` and complete AMBI-prefix layouts with `USR45` extras become explicit `AmbisonicChannelLayout` extra masks; `USR63`, out-of-range user IDs, duplicate raw IDs, and descending raw-ID order still reject. Unit tests and deterministic `avutil_core_models` fuzz fixtures cover direct native retype, lossy named raw-ID native retype, canonical parse reduction, ambisonic extra retype, and invalid raw-ID/order cases; docs and the ledger record this as local bounded coverage only. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added bounded target-AMBISONIC and CANONICAL retyping result helpers to `ChannelLayoutSpec`. `retype_to_ambisonic_order` converts custom maps with complete standard-order AMBI prefixes and strictly ordered native extras into explicit ambisonic specs, using the existing lossy result flag when names are dropped; `retype_to_canonical_order` mirrors the bounded source `canonical_order` branch by refusing to drop names and only reducing nameless current-model maps to unspecified, native, or ambisonic orders. Unit tests and deterministic `avutil_core_models` fuzz fixtures now cover lossless ambisonic retyping, lossy named custom-to-ambisonic retyping, invalid ambisonic target inputs, canonical no-op custom behavior for named/unreducible maps, and canonical reductions for unknown/native/ambisonic custom maps; docs and the ledger record this as local bounded coverage only. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added bounded lossy target-NATIVE and target-UNSPEC retyping result helpers to `ChannelLayoutSpec`. `retype_to_native_order` and `retype_to_unspecified_order` return `ChannelLayoutRetypeResult` so callers can distinguish lossless no-op/native/all-UNKNOWN conversions from allowed name-dropping or identity-dropping conversions, while `to_native_order_lossless` and `to_unspecified_order_lossless` remain strict wrappers. Unit tests and deterministic `avutil_core_models` fuzz fixtures now cover lossy result flags, name-dropping custom-to-native conversion, invalid duplicate/unknown native retypes, lossy native/ambisonic/custom-to-unspecified conversion, and lossless all-UNKNOWN-to-unspecified conversion; docs and the ledger record this as local bounded coverage only. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added `ChannelLayoutSpec::to_custom_layout` as a bounded source-shaped target-CUSTOM retype helper. Native, arbitrary native-mask, and explicit ambisonic layouts expand through `channel_from_index` into nameless `CustomChannelLayout` entries; custom maps clone without losing names; and count-only unspecified specs produce `UNK` custom maps. Unit tests and deterministic `avutil_core_models` fuzz fixtures now cover native, native-mask, high-bit native-mask, explicit ambisonic, custom, and unspecified conversions; docs and the ledger record this as local coverage only. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, and formatting.

Latest slice: added lossless custom-to-ambisonic retyping for the bounded channel-list parser surface. `CustomChannelLayout::canonical_ambisonic_layout` now rejects named maps, requires a complete `AMBI0..AMBI<N>` prefix, accepts only strictly ordered native trailing extras, and returns `AmbisonicChannelLayout` order/native-extra-mask values. `ChannelLayoutSpec::parse` now stores nameless `AMBI0+AMBI1+AMBI2+AMBI3` and `AMBI0+AMBI1+AMBI2+AMBI3+FL+FR` lists as explicit ambisonic specs while keeping named, unknown-extra, and out-of-order cases custom. Unit tests, deterministic `avutil_core_models` fuzz fixtures, docs, and ledger comments were updated; the ledger remains `implemented`, not `complete`, because byte-level parser parity, broader retyping, oracle vectors, upstream FATE, and actual fuzz execution remain pending. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added explicit channel lookup APIs for the current native-mask, explicit ambisonic, count-only unspecified, and aggregate `ChannelLayoutSpec` surfaces. `AmbisonicChannelLayout` now resolves raw/channel-string indexes with AMBI channels first and native extra-mask channels after them in mask-bit order; `ChannelLayoutSpec` delegates index and string lookup across every variant, and unspecified layouts fail lookups with typed invalid-argument errors. Unit tests and deterministic `avutil_core_models` fuzz fixtures were updated; the ledger remains `implemented`, not `complete`, because byte-level parser parity, broad retyping, oracle vectors, upstream FATE, and actual fuzz execution remain pending. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, and formatting.

Latest slice: replaced literal `+`/`@` splitting in the custom channel-list parser with a bounded FFmpeg-shaped tokenizer based on the pinned `av_opt_get_key_value` and `av_get_token` behavior. The parser now handles FFmpeg whitespace trimming, implicit channel tokens, quoted token segments, backslash-escaped separators, custom names containing additional unescaped `@`, and accepted trailing `+` separators, while `FL++FR`, `+FL`, `NONE`, unknown IDs, overlong names, and NULs remain typed invalid inputs. Unit tests, deterministic `avutil_core_models` fuzz fixtures, docs, and the ledger were updated; the ledger remains `implemented`, not `complete`, because byte-level/non-UTF-8 tokenizer calibration, broad retyping, full ambisonic semantics, oracle, upstream FATE, and actual fuzz parity remain pending. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added explicit `AmbisonicChannelLayout` storage and a `ChannelLayoutSpec::Ambisonic` variant. The parser now stores `ambisonic <order>`, `+stereo`, and native-mask extras such as `+0x5` as order/native-extra-mask specs, keeps named/custom extras such as `+FL@Left+FR@Right` as Custom-backed AMBI0..AMBI<N> maps, and rejects malformed suffixes, nested ambisonic extras, unspecified extras, and orders beyond the current AMBI0..AMBI1023 range. Unit tests, deterministic `avutil_core_models` fuzz fixtures, docs, and the ledger were updated; the ledger remains `implemented`, not `complete`, because full escaped custom-list parsing, broad retyping, full ambisonic semantics, oracle, upstream FATE, and actual fuzz parity remain pending. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added first-class `ChannelLayoutSpec::Custom` support and wired bounded custom channel-list parsing into `ChannelLayoutSpec::parse`. The parser now returns custom specs for named or otherwise non-canonical maps such as `FL@Left+FR@Right` and `FL+FL`, still reduces `FL+FR`/`FL+FC` to native or exact-mask specs where appropriate, and reduces `UNK+UNK` to count-only unspecified. `AudioFrame` and avformat audio metadata now clone/borrow channel specs instead of relying on `Copy`, and WAV helper calculations borrow audio stream parameters. Validation passed with focused avutil channel-layout tests, avformat audio tests, fuzz-package check/clippy, avutil/avformat clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full escaped custom-list parsing, explicit ambisonic-order specs, broad retyping, oracle, upstream FATE, and actual fuzz parity remain pending.

Latest slice: added bounded `CustomChannelLayout::parse_channel_list` support for FFmpeg's `parse_channel_list`-style `CH[@name]+...` token shape. The helper now produces custom-order maps for strings such as `FL@Left+FR@Right` and `UNK+UNSD+AMBI2@Height+USR2048@Vendor`, preserves optional custom names, keeps nameless native lists eligible for canonical native reduction, and rejects malformed tokens with typed invalid-argument errors. Unit tests, deterministic `avutil_core_models` fuzz fixtures, docs, and the ledger were updated; the ledger remains `implemented`, not `complete`, because the helper is not yet integrated into `ChannelLayoutSpec::parse` as a custom-order result and full escaping/quoting, retyping, oracle, upstream FATE, and actual fuzz parity remain pending. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only.

Latest slice: added source-checked nameless native channel-list parsing to `ChannelLayoutSpec::parse`. The parser now accepts native channel lists that do not map to a named layout and preserves them as exact `NativeChannelMaskLayout` values, so `FL+FR` still resolves to `stereo` while `FL`, `FL+FC`, and `2 channels (FL+FC)` keep their native masks with count validation. Unit tests and the `avutil_core_models` fuzz target fixtures cover the new native-list and described-list cases; docs and the ledger record this as implemented but not complete. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because custom `@name` parsing, implicit/broad ambisonic parsing and retyping, oracle comparison, upstream FATE, and actual fuzz execution remain absent.

Latest slice: added `NativeChannelMaskLayout` for arbitrary nonzero FFmpeg native-order channel masks and wired it into `ChannelLayoutSpec`. Modeled numeric masks still return named `Native` layouts, while unmodeled valid masks such as `0x5` and `0x8000000000000000` now preserve exact bitmasks, describe/index in mask-bit order, compute subsets, validate counts, and compare against native, mask, and custom layouts by FFmpeg-style channel IDs. Unit tests and the `avutil_core_models` fuzz target fixtures cover arbitrary low-bit/high-bit masks plus parser behavior; docs and the ledger now record this as implemented but not complete. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE dry-run and execution, formatting, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full custom-map syntax, implicit/broad ambisonic parsing and retyping, oracle comparison, upstream FATE, and actual fuzz execution remain absent.

Latest slice: added source-checked numeric-mask parsing to `ChannelLayoutSpec::parse` for the modeled native layout subset. The parser now follows FFmpeg's branch order after described channel lists and before count suffixes, accepting base-0 nonzero masks with no `-` only when `ChannelLayout::from_channel_mask` can resolve them to a modeled layout. Unit tests and the `avutil_core_models` fuzz target fixtures cover hex/decimal/octal/plus-prefixed stereo masks, a generated 5.1 mask, and invalid zero, negative, malformed, and valid-but-unmodeled masks. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE, formatting, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because arbitrary native masks, custom-map syntax, implicit/broad ambisonic parsing and retyping, oracle comparison, upstream FATE, and actual fuzz execution remain absent.

Latest slice: added source-checked `ChannelLayoutSpec::parse` count-suffix coverage. The parser now accepts current native layout names, current `FL+FR`-style native expressions, `Nc` when FFmpeg's default layout for that count is modeled as native, `NC`/`N channels` as count-only unspecified layouts, and `N channels (<native channel-list>)` with count validation. Unit tests and the `avutil_core_models` fuzz target fixtures cover valid `2c`, `10c`, `2C`, `9 channels`, and described-list forms plus invalid zero counts, `9c`, mismatches, unterminated lists, empty lists, trailing text, and NUL-containing strings. Validation passed with focused avutil channel-layout tests, fuzz-package check/clippy, avutil clippy, local component FATE, changed-path FATE, formatting, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because numeric masks, full custom-map parsing, implicit/broad ambisonic parsing and retyping, oracle comparison, upstream FATE, and actual fuzz execution remain absent.

Latest slice: added source-checked count-only unspecified channel-layout support. `UnspecifiedChannelLayout` and `ChannelLayoutSpec` now model FFmpeg's `AV_CHANNEL_ORDER_UNSPEC` fallback for positive unmodeled channel counts, including description, validation, comparison, subset, and index behavior. `AudioFrame` and avformat `AudioStreamParameters` store `ChannelLayoutSpec`, expose `channel_layout_spec()`, and preserve native-only legacy `channel_layout()` accessors. The fuzz fixture now checks unspecified default-count invariants, and local FATE mappings now cover avformat audio, WAV, and raw PCM unit filters when those components are selected. Validation passed with focused avutil channel-layout/audio-frame tests, avformat audio tests through the default target dir, fuzz-package check/clippy, avutil/avformat clippy, changed-path FATE dry-run and execution, FATE listing, formatting, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full parser/retyping/implicit-ambisonic/oracle/FATE/fuzz parity remains absent.

Previous slice: expanded `avutil-channel-layout` default-count derivation to use the modeled source-order `channel_layout_map` inventory. Source checking against pinned `libavutil/channel_layout.c` confirmed `av_channel_layout_default` returns the first map entry whose channel count matches, otherwise creates an unspecified-order layout. `ChannelLayout::default_for_count` returns the modeled FFmpeg native defaults for counts 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, and 24. Unit tests and `avutil_core_models` deterministic fixtures cover the expanded count table. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path FATE mappings, downstream `avformat` audio tests, format checks, FATE listing, and `git diff --check` with CRLF warnings only. The component remained `implemented`, not `complete`, because count-only fallback threading, full parsing/retyping/implicit-ambisonic/oracle/FATE/fuzz parity remained absent.

Latest slice: added source-checked current native/custom channel lookup helpers to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.c` confirmed native `channel_from_index` walks mask bits in source order, `index_from_channel` rejects absent and non-native IDs for native layouts, custom lookup returns first matching map entries, and `channel_from_string` is a string-to-index-to-channel wrapper. `ChannelLayout` now exposes `channel_from_index`, `index_from_channel`, `index_from_string`, and `channel_from_string`; `CustomChannelLayout` now exposes `channel_from_string`. Unit tests cover stereo lookup, invalid native IDs/strings, `22.2` mask-order indices, and custom string-to-channel lookup. `avutil_core_models` build-checks generated native source-order lookup and custom lookup fixtures. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path FATE mappings, downstream `avformat` audio tests, format checks, FATE listing, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full parsing/retyping/implicit-ambisonic/unspecified/oracle/FATE/fuzz parity remains absent.

Latest slice: added source-checked current-subset layout subset helpers to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.c` confirmed `av_channel_layout_subset` intersects native or ambisonic masks directly and, for custom maps, scans requested native mask bits and includes those whose raw channel IDs are present. `ChannelLayout::subset_mask` and `CustomChannelLayout::subset_native_mask` now cover the currently modeled native/custom surfaces without introducing implicit ambisonic layout order semantics. Unit tests cover native mask intersection, zero masks, named custom presence, duplicate native IDs collapsing to one bit, out-of-order custom maps, unknown-only custom maps, and exclusion of non-native custom IDs. `avutil_core_models` build-checks the same deterministic subset-mask fixtures. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path FATE mappings, downstream `avformat` audio tests, format checks, FATE listing, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full parsing/retyping/implicit-ambisonic/unspecified/oracle/FATE/fuzz parity remains absent.

Latest slice: added source-checked current-subset layout equivalence helpers to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.c` confirmed `av_channel_layout_compare`'s channel-count gate, name-insensitive channel-by-index custom comparison, and native mask comparison for same-order native layouts. `ChannelLayout::is_equivalent_to`, `ChannelLayout::is_equivalent_to_custom`, `CustomChannelLayout::is_equivalent_to_native`, and `CustomChannelLayout::is_equivalent_to_custom` now cover the currently modeled native/custom layout surfaces. Unit tests cover native mask equality and inequality, named custom stereo comparing equal to native/nameless custom stereo, out-of-order custom layouts comparing unequal, unknown custom maps comparing equal by channel ID while remaining unequal to native stereo, and named/nameless custom ambisonic IDs comparing equal. `avutil_core_models` now build-checks the same deterministic equivalence fixtures. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path FATE mappings, downstream `avformat` audio tests, format checks, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full parsing/retyping/implicit-ambisonic/unspecified/oracle/FATE/fuzz parity remains absent.

Latest slice: added source-checked custom-map ambisonic helpers to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.c` confirmed `av_channel_layout_ambisonic_order` and `try_describe_ambisonic`: custom maps must contain a complete standard-order ambisonic prefix, each ACN must match its index, ambisonic entries cannot appear after non-ambisonic entries, and valid maps describe as `ambisonic <order>` plus optional trailing channels. `CustomChannelLayout::ambisonic_order` now exposes the typed helper and `describe()` now reduces valid custom maps to strings such as `ambisonic 1+stereo` while invalid or incomplete maps fall back to explicit custom descriptions. Unit tests cover zeroth/first-order maps, named ambisonic-prefix behavior, native/named/custom extras, no-ambisonic maps, incomplete orders, ACN/index mismatch, and ambisonic-after-non-ambisonic rejection; `avutil_core_models` now build-checks deterministic valid and incomplete custom ambisonic fixtures. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path FATE mappings, downstream `avformat` audio tests, format checks, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full parsing/retyping/implicit-ambisonic/compare/oracle/FATE/fuzz parity remains absent.

Latest slice: added source-checked custom-to-native canonicalization helpers to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.c` confirmed `masked_description`'s native-channel and strict-order constraints, plus `canonical_order`'s custom-name guard. `CustomChannelLayout::canonical_native_mask` now reports a native mask only for canonical ordered native IDs, `canonical_native_layout` reduces nameless maps to the currently modeled native layouts, and `describe()` now returns native layout names such as `stereo` only for those lossless custom maps. Unit tests cover nameless stereo reduction, named custom maps staying custom, out-of-order/duplicate/non-native rejection, all-unknown maps staying custom, and existing custom lookup behavior; `avutil_core_models` now build-checks deterministic native-canonicalization fixtures. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path FATE mappings, downstream `avformat` audio tests, format checks, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full parsing/retyping/ambisonic/compare/oracle/FATE/fuzz parity remains absent.

Latest slice: added source-checked `ChannelCustom` and `CustomChannelLayout` helpers to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed the `AVChannelCustom` shape, custom-init `UNK` defaults, `NONE` rejection, first-match lookup, `CH@name`/`@name` custom-name lookup, and custom-order description shape. Unit tests now cover unknown-map initialization, bounded custom names, duplicate raw channel IDs, first-match lookup, named lookup, `UNSD`/`AMBI`/`USR` entries, custom descriptions, and invalid-entry rejection; `avutil_core_models` now build-checks deterministic custom-map fixtures. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path FATE mappings, downstream `avformat` audio tests, format checks, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full parsing/retyping/ambisonic/compare/oracle/FATE/fuzz parity remains absent.

Latest slice: added a source-checked `ChannelId` helper to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed the raw ID values and name/description formatting for `NONE`, `UNSD`, `UNK`, `AMBI0..AMBI1023`, and `USR<raw>` user IDs. Unit tests now cover raw ID mapping, canonical names, descriptions, native channel conversion, special IDs, ambisonic bounds, and noncanonical parse rejection; `avutil_core_models` now build-checks deterministic raw ID/name/description fixtures. Validation passed with focused channel-layout tests, fuzz-package build/clippy, avutil clippy, local FATE mapping, changed-path local mappings, downstream `avformat` audio tests, format checks, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full custom/native/ambisonic layout order semantics, oracle inventory parity, upstream FATE, and actual fuzz execution remain absent.

Latest slice: added source-checked standalone FFmpeg 8.1.1 native channel IDs `SDL`, `SDR`, `SSL`, `SSR`, `TTL`, and `TTR` to `avutil-channel-layout`. Source checking against pinned `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed the six enum entries, short names, descriptions, and exact native mask bits. Unit tests now cover channel names, case-insensitive lookup, masks, and continued rejection of unsupported standalone channel expressions; `avutil_core_models` now build-checks deterministic name/mask fixtures for those channels. Validation passed with focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local FATE mapping, changed-path local mappings, downstream `avformat` audio tests, format checks, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full oracle inventory parity, special channel ID semantics, custom/native/ambisonic orders, upstream FATE, and actual fuzz execution remain absent.

Latest slice: added source-checked `TC`, `BFC`, `BFL`, `BFR`, and `22.2` native-layout coverage to `avutil-channel-layout`. Source checking against pinned FFmpeg 8.1.1 `libavutil/channel_layout.h` and `libavutil/channel_layout.c` confirmed the four channel IDs, their native masks, short names, `AV_CH_LAYOUT_22POINT2`, `AV_CHANNEL_LAYOUT_22POINT2`, and the `channel_layout_map` name `22.2`. Unit tests now cover channel names/masks, layout order/count/channels, canonical string output, mask/channel-expression round trips, known-layout order, default lookup non-claiming for 24 channels, parser round trips, and unsupported expressions; `avutil_core_models` now build-checks the expanded layout generator. Validation passed with focused channel-layout tests, fuzz-package check/clippy, avutil clippy, local FATE mapping, changed-path local mappings, `avformat` audio tests, format checks, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`, because full oracle inventory parity, remaining FFmpeg default layout count behavior, custom/native/ambisonic orders, upstream FATE, and actual fuzz execution remain absent.

Latest slice: added FFmpeg-shaped sample-format table strings to `avutil-sample-format`. Source checking against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` confirmed the `av_get_sample_fmt_string` shape used here: negative sample format values print `name   depth`, and valid native rows use a left-aligned 6-column name, three spaces, a 2-column bit depth, and a trailing space. `SampleFormat::sample_fmt_string_header` and `SampleFormat::sample_fmt_string` expose that Rust-native table formatting for the current native sample-format inventory. Unit tests cover all current row strings, and `avutil_core_models` build-checks generated header/name-padding/depth/trailing-space invariants. The component remains `implemented`, not `complete`, because oracle differentials, upstream FATE parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest slice: added FFmpeg-shaped owned allocation helpers to `avutil-sample-format`. Source checking against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` confirmed the `av_samples_alloc` / `av_samples_alloc_array_and_samples` shape used here: compute the same buffer size, allocate one contiguous buffer, fill array pointers through the fill-array helper, and silence-fill only the originally requested sample count. `SampleAllocation` records the `SampleArrayLayout`, owns a writable `BufferRef`, reports requested versus effective sample counts, exposes immutable and mutable plane slices, and intentionally keeps Rust-visible padding/tail bytes zeroed rather than exposing FFmpeg's uninitialized allocation tail. Unit tests cover packed and planar allocation shape, `u8` silence initialization, deterministic auto-aligned tail bytes, mutable plane writes, invalid inputs, and `into_buffer`; `avutil_core_models` now build-checks generated allocation invariants. The component remains `implemented`, not `complete`, because oracle differentials, upstream FATE parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest slice: added FFmpeg-shaped fill-array layout helpers to `avutil-sample-format`. Source checking against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` confirmed the `av_samples_fill_arrays` shape used here: it calls the same buffer-size helper, reports a line size, assigns one packed data pointer or line-size-spaced planar channel pointers, and returns the total required buffer size. `SampleArrayLayout` records the computed buffer layout plus per-plane `SamplePlaneRange` offsets; `fill_arrays_layout` models the pointer layout without raw pointers; and `split_buffer`/`split_buffer_mut` expose safe immutable and mutable Rust slices from caller-owned contiguous buffers. Unit tests cover packed and planar layouts, `align=0` auto padding, extra trailing buffer tolerance, mutable split behavior, invalid sample/channel inputs, and short-buffer rejection; `avutil_core_models` now build-checks generated fill-array split invariants. The component remains `implemented`, not `complete`, because oracle differentials, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest slice: added FFmpeg-shaped sample copy helpers to `avutil-sample-format`. Source checking against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` confirmed the `av_samples_copy` shape used here: packed formats compute one interleaved byte span using all channels, planar formats compute one byte span per channel plane, source and destination sample offsets are scaled by that block alignment, and overlapping ranges use movement semantics rather than byte-for-byte unsafe aliasing assumptions. `SampleCopyRange` records destination byte offset, source byte offset, byte length, and plane count; `SampleFormat::copy_range` models the range math; `copy_samples` copies between bounded Rust plane slices; and `copy_samples_within` handles overlapping in-place moves. Unit tests cover packed/planar range math, packed and planar cross-buffer copy behavior, overlapping in-place copies, zero-length ranges, overflow rejection, wrong plane counts, short planes, and no-mutation invalid inputs; `avutil_core_models` now build-checks generated copy invariants. The component remains `implemented`, not `complete`, because oracle differentials, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest slice: added FFmpeg-shaped silence-fill helpers to `avutil-sample-format`. Source checking against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` confirmed the `av_samples_set_silence` shape used here: packed formats compute one interleaved byte span using all channels, planar formats compute one byte span per channel plane, `u8`/`u8p` fill with `0x80`, and every other current native sample format fills with `0x00`. `SampleSilenceRange` records byte offset, byte length, plane count, and fill byte; `SampleFormat::silence_range` models the range math; and `fill_silence` validates all plane counts and ranges before mutating. Unit tests cover silence byte values, packed/planar range math, packed and planar fill behavior, zero-length ranges, overflow rejection, and no-mutation invalid inputs; `avutil_core_models` now build-checks generated silence invariants. The component remains `implemented`, not `complete`, because oracle differentials, upstream FATE parity, `av_samples_alloc`-style owned allocation parity, broader sample-data conversion routines, and actual fuzz execution are still absent.

Latest slice: added FFmpeg-shaped sample-buffer layout helpers to `avutil-sample-format`. Source checking against pinned FFmpeg 8.1.1 `libavutil/samplefmt.c` confirmed the `av_samples_get_buffer_size` shape used here: `align=0` pads samples to 32 and uses byte alignment 1, packed line size includes channels, planar line size is per-channel, and total size is one line or one line per channel. `SampleBufferLayout` records line size, total buffer size, plane count, effective sample count, and effective alignment; `SampleFormat::buffer_layout` models packed versus planar line-size math with explicit alignment and FFmpeg's `align=0` 32-sample auto-padding behavior; and `aligned_plane_sizes` exposes the resulting per-plane storage sizes. Unit tests cover packed/planar layouts, explicit alignment, zero-alignment auto-padding, invalid inputs, and FFmpeg-int-range overflow rejection; `avutil_core_models` now build-checks generated buffer-layout invariants. The component remains `implemented`, not `complete`, because oracle differentials, upstream FATE parity, conversion/silence helper parity, and actual fuzz execution are still absent.

Latest slice: added `BitWriter::clear` and `BitWriter::truncate_bits` to support deterministic bitstream rollback. Truncation validates that the requested bit position is already written, preserves the original buffer and cursor on out-of-range requests, truncates byte storage to the retained bit count, masks unused bits in the final partial byte, and lets later writes resume at the truncated position. Unit tests cover partial-tail masking plus rewrite, out-of-range no-mutation errors, and clear/reset behavior; `avutil_bitreader` now build-checks generated truncation, clear, tail-padding, and no-mutation invariants. Pinned PutBitContext differential vectors, upstream FATE parity, and actual fuzz execution remain open.

Latest slice: extended `add_stable` to source-shaped bounded signed increments. The helper no longer rejects negative increments up front; it scales the increment time base through the existing rational reduction path, subtracts exact negative tick increments, keeps fractional negative increments unchanged through the source-shaped `m < d` branch, and returns typed errors for exact-result overflow. Unit tests cover exact negative subtraction, fractional negative no-op behavior, and exact overflow rejection; `avutil_core_models` now generates signed stable-add increments and checks them against an independent model. Pinned oracle differential vectors, upstream FATE parity, exact out-of-range C behavior, and actual fuzz execution remain open.

Latest slice: added `rescale_delta` to the shared `avutil-timebase` model. The helper validates positive input/frame-output time bases, rejects `AV_NOPTS_VALUE` input timestamps, requires nonnegative FFmpeg-int durations, initializes `last` on first call, follows the source-shaped zero-duration/simple-round path, preserves duration state when `last` is inside the accepted timestamp window, clips to that window when needed, falls back to input timestamp rescaling when state is outside the window, and preserves `last` on typed validation or overflow failures. Unit tests cover the first-call, zero-duration, stateful, clipping, fallback, and no-mutation error paths; `avutil_core_models` now build-checks the helper against an independent model. Pinned oracle differential vectors, upstream FATE parity, negative `av_add_stable` calibration, and actual fuzz execution remain open.

Latest slice: added `add_stable` to the shared `avutil-timebase` model. The helper validates positive timestamp and increment time bases, accepts nonnegative increments, scales increment time bases through the existing bounded rational reduction path, performs exact tick addition where possible, keeps sub-tick increments unchanged, and uses the source-shaped rescale path to avoid repeated fractional rounding drift while preserving existing phase. Unit tests cover exact increments, zero increments, 1/30-second repeated increments in a millisecond time base, phase preservation, sub-tick no-op behavior, and invalid inputs; `avutil_core_models` now build-checks the helper against an independent model. Pinned oracle differential vectors, upstream FATE parity, `av_rescale_delta`, negative-increment calibration, and actual fuzz execution remain open.

Latest slice: added `compare_ts` and `compare_mod` to the shared `avutil-timebase` model. `compare_ts` validates positive rational time bases and orders timestamps with exact cross-products; `compare_mod` validates power-of-two moduli and returns the centered modular difference matching FFmpeg's `av_compare_mod` shape. The slice was source-checked against pinned FFmpeg 8.1.1 `libavutil/mathematics.h`/`.c`, exports the helpers from `avutil`, adds focused timebase tests for cross-timebase ordering, invalid inputs, modular wraparound, and invalid moduli, and extends `avutil_core_models` with independent compare-model invariants. Pinned oracle differential vectors, upstream FATE parity, `av_rescale_delta`, and actual fuzz execution remain open.

Latest slice: added `Rational::to_int_float_bits` for FFmpeg 8.1.1 `av_q2intfloat`-style platform-independent IEEE-754 single-precision bit conversion. It preserves the pinned special values for `0/0`, zero numerator, and zero denominator, handles finite signed rationals and negative denominators through integer rescaling, and rejects raw `i32::MIN` negation cases with typed errors instead of modeling C signed-overflow behavior. Unit tests cover finite IEEE vectors, special values, and invalid raw inputs; `avutil_core_models` build-checks the generated invariant against Rust `f32` bits for small finite values plus the pinned special cases. Pinned oracle differential vectors, upstream FATE parity, and actual fuzz execution remain open.

Latest slice: added FFmpeg-shaped rational comparison and nearest-candidate helpers. `Rational::av_cmp` now mirrors `av_cmp_q` for finite values, raw positive/negative infinity sentinels, and `0/0` indeterminate results; `PartialOrd` routes through that comparison. `Rational::nearer_to` and `Rational::find_nearest_index` provide exact Rust-native equivalents for `av_nearer_q` and `av_find_nearest_q_idx` over explicit slices, with first-tie preservation, empty-list handling, and typed zero-denominator rejection. The slice was source-checked against pinned FFmpeg 8.1.1 `libavutil/rational.h`/`.c`, and unit plus `avutil_core_models` fuzz-harness build invariants cover the new behavior. Pinned oracle differential vectors, upstream FATE parity, additional helpers, and actual fuzz execution remain open.

Latest slice: added FFmpeg-shaped error-string helpers for the shared `avutil-error` model. `AvErrorCode::description`, `AvErrorCode::make_error_string`, `AvError::ffmpeg_description`, `AvError::ffmpeg_error_string`, and exported `av_error_description`/`av_strerror`/`av_make_error_string` now cover the pinned FFmpeg 8.1.1 `AVERROR_LIST` descriptions from `libavutil/error.c`, including BUG2 sharing BUG's description, HTTP/RTSP error descriptions, and generic unknown-code fallback strings. Source checking also captured that the combined input/output-changed raw code duplicates `INPUT_CHANGED` under FFmpeg's bitwise behavior, so Rust preserves the first-match `Input changed` result instead of inventing a distinct string. Unit tests and `avutil_core_models` build-check invariants cover the table; platform errno-string parity, oracle/FATE parity, and actual fuzz execution remain open.

Latest slice: updated `fate-runner` mapping parsing and execution so fields with `env:NAME=value` become child-process environment assignments, with the same placeholder resolution and prerequisite validation as command arguments. `tests/differential/mappings.txt` now maps the rawvideo oracle harness to `fftools-ffmpeg-rawvideo-file-output`, `avformat-rawvideo-demuxer`, and `avformat-rawvideo-muxer`, injecting `FFMPEG_ORACLE={oracle_ffmpeg}` from the validated `--oracle-ffmpeg` path. Unit coverage checks env parsing, invalid env rows, placeholder resolution, formatted diagnostics, and prerequisite failures. The docs and ledger record this as differential wiring only; rawvideo is still below `differential_pass` until a pinned FFmpeg 8.1.1 oracle binary runs the ignored tests successfully.

Latest slice: updated `fate-runner` explicit run parsing so repeated `--component <id>` flags select multiple components in a single invocation, with duplicate component IDs deduplicated before mapping execution. Parser coverage now checks multi-component parsing, duplicate removal, dry-run preservation, and unchanged `--changed`/`--component` ambiguity rejection. A real dry-run verified that `avformat-rawvideo-demuxer` and `avformat-rawvideo-muxer` mappings resolve together. The ledger and FATE/oracle/architecture docs record the new behavior. `fate-runner` remains `scaffolded`, not complete, because upstream sample-backed FATE execution is still absent.

Latest slice: added ignored rawvideo oracle integration tests in `crates/fftools/tests/rawvideo_oracle.rs`. The tests create deterministic raw `rgb24` and `gbrp10msble` inputs, run the Rust constrained rawvideo file-output path, run the pinned FFmpeg oracle with `-c:v copy -f rawvideo`, and compare output bytes plus Rust command accounting. `fate-runner` now maps this test path back to the rawvideo file-output, demuxer, and muxer ledger components, with unit coverage for the selection rule. `tests/differential/README.md`, the rawvideo CLI/demuxer/muxer and `fate-runner` ledger entries, and oracle/compatibility/architecture docs now describe the harness and the missing-oracle blocker. Default test compilation passes with the tests ignored; explicit ignored execution fails locally before parity comparison until a pinned oracle binary is configured.

Latest slice: added FFmpeg 8.1.1 MSB-aligned planar 10/12-bit YUV444 and GBR support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuv444p10msble`, `yuv444p10msbbe`, `yuv444p12msble`, `yuv444p12msbbe`, `gbrp10msble`, `gbrp10msbbe`, `gbrp12msble`, and `gbrp12msbbe` in the inventory with three full-resolution planes, YUV or RGB descriptor metadata, 10/12-bit components, 30/36 logical descriptor bpp, and two stored bytes per component sample with valid bits in the high bits. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `gbrp10msble` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, main and fuzz-package check/clippy, changed-path FATE dry-run, and directly affected single-component FATE dry-runs. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, conversion behavior, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed 32-bit integer RGB/RGBA support to the current shared pixel/rawvideo model. `PixelFormat` now reports `rgb96le`, `rgb96be`, `rgba128le`, and `rgba128be` in the inventory with RGB descriptor metadata, one packed integer payload plane, no chroma subsampling, 32-bit components, 96/128 logical descriptor bpp, twelve/sixteen stored bytes per pixel, and alpha metadata only for `rgba128*`. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `rgba128le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil, avformat, avcodec FATE, and fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, changed-path FATE dry-run, directly affected local FATE component mappings except the broad fftools default-target mapping blocked by Windows Application Control, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed floating RGBA support to the current shared pixel/rawvideo model. `PixelFormat` now reports `rgbaf16le`, `rgbaf16be`, `rgbaf32le`, and `rgbaf32be` in the inventory with RGB descriptor metadata, one packed payload plane, alpha and float metadata, no chroma subsampling, 16/32-bit components, 64/128 logical descriptor bpp, and eight/sixteen stored bytes per pixel. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `rgbaf32le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed floating RGB support to the current shared pixel/rawvideo model. `PixelFormat` now reports `rgbf16le`, `rgbf16be`, `rgbf32le`, and `rgbf32be` in the inventory with RGB descriptor metadata, one packed payload plane, float metadata, no alpha, no chroma subsampling, 16/32-bit components, 48/96 logical descriptor bpp, and six/twelve stored bytes per pixel. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `rgbf32le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 planar floating GBR support to the current shared pixel/rawvideo model. `PixelFormat` now reports `gbrpf16le`, `gbrpf16be`, `gbrpf32le`, and `gbrpf32be` in the inventory with RGB descriptor metadata, three full-resolution payload planes, float metadata, no alpha, no chroma subsampling, 16/32-bit components, 48/96 logical descriptor bpp, and two/four stored bytes per component sample. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `gbrpf32le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed UYYVYY411 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `uyyvyy411` in the inventory with YUV descriptor metadata, one payload plane, three exposed 8-bit components, no alpha, log2 chroma `(2,0)`, 12 logical descriptor bpp, no scalar packed bytes-per-pixel value, and width-divisible-by-4 sizing where each 6-byte group stores four pixels. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the format, including descriptor names, representative decode/demux/mux sizing, `uyyvyy411` CLI coverage, invalid width checks, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, changed-path FATE dry-run, directly affected local FATE component mappings, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 Bayer CFA support to the current shared pixel/rawvideo model. `PixelFormat` now reports the 8-bit and 16-bit Bayer BGGR/RGGB/GBRG/GRBG names in the inventory with one payload plane, RGB-class descriptor metadata, a Bayer helper flag, no alpha, no chroma subsampling, 8/16 logical descriptor bpp, and one/two stored bytes per pixel. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `bayer_bggr8` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, changed-path FATE dry-run, and directly affected local FATE component mappings. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, Bayer conversion/demosaic behavior, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed X/V YUV 4:4:4 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `xv30le`, `xv30be`, `xv36le`, `xv36be`, `xv48le`, `xv48be`, `v30xle`, and `v30xbe` in the inventory with YUV descriptor metadata, one payload plane, three exposed YUV components, no alpha, no chroma subsampling, 30/36/48 logical descriptor bpp, and four/six/eight stored bytes per pixel preserving the undefined X lane. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `xv30le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests through accepted target caches, fuzz package check/clippy, workspace format check, workspace clippy, changed-path FATE dry-run, and `git diff --check` with CRLF warnings only. Broad workspace library execution and actual changed-path FATE execution remain partly blocked by Windows Application Control on target-cache-specific test executables. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed floating gray+alpha support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yaf16le`, `yaf16be`, `yaf32le`, and `yaf32be` in the inventory with grayscale descriptor metadata, one payload plane, two YA components, alpha and float flags, no chroma subsampling, 32/64 logical descriptor bpp, and four/eight stored bytes per pixel. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including descriptor names, representative decode/demux/mux sizing, `yaf16le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed 8-bit YUV/YUVA support to the current shared pixel/rawvideo model. `PixelFormat` now reports `vuya`, `vuyx`, `ayuv`, `uyva`, and `vyu444` in the inventory with YUV descriptor metadata, one payload plane, log2 chroma `(0,0)`, and FFmpeg-shaped logical bpp versus storage sizing. `vuya`, `ayuv`, and `uyva` are four-component alpha formats stored in four bytes per pixel; `vuyx` preserves FFmpeg's three-component/no-alpha descriptor with four stored bytes per pixel; `vyu444` is a three-component three-byte-per-pixel path. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including all five descriptor names, representative decode/demux/mux sizing, `vuya` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed X2RGB10/X2BGR10 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `x2rgb10le`, `x2rgb10be`, `x2bgr10le`, and `x2bgr10be` in the inventory with RGB descriptor metadata, one payload plane, three 10-bit exposed color components, 30 logical descriptor bpp, no alpha, no chroma subsampling, and four stored bytes per pixel with the two-bit X lane preserved as raw padding. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including all four descriptor names, representative decode/demux/mux sizing, `x2rgb10le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed XYZ12 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `xyz12le` and `xyz12be` in the inventory with XYZ descriptor metadata, one payload plane, three 12-bit components, 36 logical descriptor bpp, no alpha, no chroma subsampling, and six stored bytes per pixel. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including both descriptor names, representative decode/demux/mux sizing, `xyz12le` CLI coverage, invalid payload checks, split-plane coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 packed AYUV64 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `ayuv64le` and `ayuv64be` in the inventory with YUV descriptor metadata, one payload plane, four 16-bit components, alpha metadata, 64 logical descriptor bpp, no chroma subsampling, and eight stored bytes per pixel. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including both descriptor names, representative decode/demux/mux sizing, `ayuv64le` CLI coverage, invalid payload checks, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 high-bit packed YUV 4:2:2 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `y210le`/`y210be`, `y212le`/`y212be`, and `y216le`/`y216be` in the inventory with YUV descriptor metadata, one payload plane, 10/12/16-bit component depths, 20/24/32 logical average descriptor bpp, four stored bytes per pixel, and even-width chroma validation. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise the family, including all six descriptor names, representative decode/demux/mux sizing, `y210le` CLI coverage, and fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 semi-planar NV YUV support to the current shared pixel/rawvideo model. `PixelFormat` now reports `nv16`, `nv20le`, `nv20be`, `nv24`, and `nv42` in the inventory with YUV descriptor metadata, two payload planes, 8-bit or 10-bit component depths, 4:2:2 or 4:4:4 chroma geometry, endian metadata for `nv20be`, and matching even-width validation for the 4:2:2 variants. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise representative `nv16`, `nv20le`/`nv20be`, `nv24`, and `nv42` paths. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 high-bit-depth planar YUVA support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuva420p9le`/`yuva420p9be`, `yuva422p9le`/`yuva422p9be`, `yuva444p9le`/`yuva444p9be`, `yuva420p10le`/`yuva420p10be`, `yuva422p10le`/`yuva422p10be`, `yuva444p10le`/`yuva444p10be`, `yuva422p12le`/`yuva422p12be`, `yuva444p12le`/`yuva444p12be`, `yuva420p16le`/`yuva420p16be`, `yuva422p16le`/`yuva422p16be`, and `yuva444p16le`/`yuva444p16be` in the inventory with YUV descriptor metadata, alpha flag, four payload planes, 9/10/12/16-bit component depths, two stored bytes per sample, and matching chroma validation. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and affected fuzz harnesses now exercise representative 9-bit, 10-bit, 12-bit, and 16-bit YUVA paths. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, and local changed-path FATE mappings through `target-codex`. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 `yuva420p`, `yuva422p`, and `yuva444p` support to the current shared pixel/rawvideo model. `PixelFormat` now reports the three names in the inventory with YUV descriptor metadata, four 8-bit components, 20/24/32 average descriptor bpp, four planar payload planes, alpha flag, no float flag, and the corresponding 4:2:0, 4:2:2, or 4:4:4 chroma validation with full-resolution alpha. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including decoder plane/line-size coverage, demux/mux frame-size and invalid-geometry coverage for all three names, `yuva420p` CLI coverage, and YUVA fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools checks, fuzz package check/clippy, workspace format check, workspace clippy, workspace library tests, local changed-path FATE mappings through `target-codex`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, high-bit YUVA variants, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 `yuv440p10le`, `yuv440p10be`, `yuv440p12le`, and `yuv440p12be` support to the current shared pixel/rawvideo model. `PixelFormat` now reports the four names in the inventory with YUV descriptor metadata, 10-bit or 12-bit component depth, 20 or 24 average descriptor bpp, three planar payload planes, no alpha or float flag, and 4:4:0 chroma validation requiring even height only. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including decoder plane/line-size coverage, demux/mux frame-size and invalid-geometry coverage for all four names, `yuv440p10le` CLI coverage, and high-bit planar YUV fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, broad local FATE changed-path mappings, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added FFmpeg 8.1.1 14-bit and 16-bit planar YUV support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuv420p14le`/`yuv420p14be`, `yuv422p14le`/`yuv422p14be`, `yuv444p14le`/`yuv444p14be`, `yuv420p16le`/`yuv420p16be`, `yuv422p16le`/`yuv422p16be`, and `yuv444p16le`/`yuv444p16be` in the inventory with YUV descriptor metadata, 14-bit or 16-bit component depth, 21/28/42 or 24/32/48 average descriptor bpp, three planar payload planes, no alpha or float flag, and the same 4:2:0/4:2:2/4:4:4 chroma validation as the matching 8-bit/9-bit/10-bit/12-bit families. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including decoder plane/line-size coverage, demux/mux frame-size and invalid-geometry coverage for all twelve names, `yuv420p14le` CLI coverage, and high-bit planar YUV fuzz-build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, fuzz package check/clippy, workspace format check, workspace clippy, directly affected local FATE component mappings, workspace library tests, and `git diff --check` with CRLF warnings only. Broad local FATE `run --changed` execution remains blocked by Windows Application Control on freshly built child test executables, but changed selection dry-run passed. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, pixel conversion, hardware formats, and actual fuzz execution are still absent.

Latest slice: added paletted `pal8` support to the current shared pixel/rawvideo model. `PixelFormat` now reports `pal8` in the inventory with RGB descriptor metadata, one modeled index component, one byte-per-pixel raw packet payload plane, alpha and paletted descriptor flags, no float flag, and exported `AVPALETTE_COUNT`/`AVPALETTE_SIZE` constants matching FFmpeg's 256-entry/1024-byte palette definition. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format, including a decoder fixture, rawvideo demux/mux sizing, `pal8` CLI coverage, and palette-constant descriptor invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace format check, local FATE-runner component and changed mappings, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only; several fresh test executable launches were blocked by Windows Application Control and passed through accepted target directories. The affected components remain `implemented`, not `complete`, because full frame palette side-plane/context propagation, oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, and actual fuzz execution are still absent.

Latest slice: added deprecated full-range planar YUVJ support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuvj420p`, `yuvj422p`, `yuvj411p`, `yuvj440p`, and `yuvj444p` in the inventory with YUV descriptor metadata, three 8-bit components, three planar payload planes, no alpha flag, no float flag, and the same chroma geometry validation as the matching non-`j` YUV formats. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including all five decoder fixtures, rawvideo demux/mux sizing, `yuvj420p` CLI coverage, and fuzz build invariants. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace format check, workspace clippy, workspace library tests, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because color-range metadata/conversion, oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` parity, and actual fuzz execution are still absent.

Latest slice: added 1bpp monochrome bitstream support to the current shared pixel/rawvideo model. `PixelFormat` now reports `monow` and `monob` in the inventory with grayscale descriptor metadata, one modeled component, one row-padded payload plane, one descriptor bit per pixel, no alpha flag, no float flag, no chroma subsampling, and no whole-byte packed-bytes-per-pixel value because eight pixels share each byte. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including decoder coverage for both polarities, `monow` CLI coverage, and rawvideo mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` bitstream/component layout parity, and actual fuzz execution are still absent.

Latest slice: added packed 4bpp RGB bitstream support to the current shared pixel/rawvideo model. `PixelFormat` now reports `rgb4` and `bgr4` in the inventory with RGB descriptor metadata, three modeled components, one row-padded payload plane, four descriptor bits per pixel, no alpha flag, no float flag, no chroma subsampling, and no whole-byte packed-bytes-per-pixel value because two pixels share each byte. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including decoder coverage, `rgb4` CLI coverage, and rawvideo mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, full `AVPixFmtDescriptor` bitstream/component layout parity, and actual fuzz execution are still absent.

Latest slice: added byte-packed low-bit-depth RGB support to the current shared pixel/rawvideo model. `PixelFormat` now reports `rgb8`, `bgr8`, `rgb4_byte`, and `bgr4_byte` in the inventory with RGB descriptor metadata, three modeled components, one packed payload plane, one byte per pixel, no alpha flag, no float flag, and no chroma subsampling. `rgb8`/`bgr8` report scalar max component depth 3 and `rgb4_byte`/`bgr4_byte` report scalar max component depth 2 until full per-component descriptor parity exists. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including representative decoder coverage, `rgb8` CLI coverage, and rawvideo mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace clippy, local FATE-runner component mappings, and local `run --changed`. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added semi-planar YUV 4:2:0 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `nv12` and `nv21` in the inventory with YUV descriptor metadata, three modeled 8-bit components, two payload planes, log2 chroma `(1,1)`, no alpha flag, no float flag, one full luma plane, one interleaved chroma plane, and even-width/even-height validation. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise both names, including decoder coverage, `nv12` CLI coverage, and rawvideo mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace format check, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed YUV 4:2:2 support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuyv422`, `uyvy422`, and `yvyu422` in the inventory with YUV descriptor metadata, three modeled 8-bit components, one packed payload plane, two bytes per pixel, log2 chroma `(1,0)`, no alpha flag, no float flag, and even-width validation. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including representative decoder coverage, `yuyv422` CLI coverage, and rawvideo mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace format check, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed 16-bit RGB/BGR support to the current shared pixel/rawvideo model. `PixelFormat` now reports `rgb565be`, `rgb565le`, `rgb555be`, `rgb555le`, `bgr565be`, `bgr565le`, `bgr555be`, `bgr555le`, `rgb444le`, `rgb444be`, `bgr444le`, and `bgr444be` in the inventory with RGB descriptor metadata, three modeled components, one packed payload plane, two bytes per pixel, no alpha flag, no float flag, and no chroma subsampling. `rgb565*`/`bgr565*` report max component depth 6 in the current scalar descriptor model while full per-component descriptor parity remains pending. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including new representative decoder coverage, `rgb565le` CLI coverage, and rawvideo mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed high-bit-depth grayscale support to the current shared pixel/rawvideo model. `PixelFormat` now reports `gray9le`, `gray9be`, `gray10le`, `gray10be`, `gray12le`, `gray12be`, `gray14le`, and `gray14be` in the inventory, accepts `y9le`/`y9be`, `y10le`/`y10be`, `y12le`/`y12be`, and `y14le`/`y14be` aliases, and exposes grayscale descriptor metadata with one modeled component, 9/10/12/14 bits per component and pixel, one packed payload plane, no alpha flag, no float flag, no chroma subsampling, and two stored bytes per sample. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including new `gray10le` decoder coverage, `y10le` CLI alias coverage, and gray9/gray10/gray12/gray14 mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar GBRA support to the current shared pixel/rawvideo model. `PixelFormat` now reports `gbrap`, `gbrap10le`, `gbrap10be`, `gbrap12le`, `gbrap12be`, `gbrap14le`, `gbrap14be`, `gbrap16le`, `gbrap16be`, `gbrap32le`, `gbrap32be`, `gbrapf16le`, `gbrapf16be`, `gbrapf32le`, and `gbrapf32be` with RGB descriptor metadata, four modeled components, alpha metadata, no chroma subsampling, float metadata on the `f16`/`f32` variants, and four full-resolution payload planes. The raw storage model uses one byte per sample for `gbrap`, two bytes per sample for 10/12/14/16-bit integer and f16 variants, and four bytes per sample for 32-bit integer and f32 variants, treating endian as pixel-format naming only until conversion exists. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the family, including new `gbrap`/`gbrapf32le` decoder coverage, `gbrapf32le` CLI coverage, and one-/two-/four-byte mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, workspace clippy, local FATE-runner component mappings, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 16-bit `gbrp16le` and `gbrp16be` support to the current shared pixel/rawvideo model. `PixelFormat` now reports both names in the inventory with RGB descriptor metadata, three modeled 16-bit components, 48 descriptor bits per pixel, three full-resolution GBR payload planes, no packed byte stride, no alpha flag, no float flag, and no chroma subsampling. The raw storage model uses two bytes per sample in each plane and treats endian as pixel-format naming only until conversion exists. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including new `gbrp16le` decoder/CLI coverage and `gbrp16be` mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 14-bit `gbrp14le` and `gbrp14be` support to the current shared pixel/rawvideo model. `PixelFormat` now reports both names in the inventory with RGB descriptor metadata, three modeled 14-bit components, 42 descriptor bits per pixel, three full-resolution GBR payload planes, no packed byte stride, no alpha flag, no float flag, and no chroma subsampling. The raw storage model uses two bytes per sample in each plane and treats endian as pixel-format naming only until conversion exists. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including new `gbrp14le` decoder/CLI coverage and `gbrp14be` mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 12-bit `gbrp12le` and `gbrp12be` support to the current shared pixel/rawvideo model. `PixelFormat` now reports both names in the inventory with RGB descriptor metadata, three modeled 12-bit components, 36 descriptor bits per pixel, three full-resolution GBR payload planes, no packed byte stride, no alpha flag, no float flag, and no chroma subsampling. The raw storage model uses two bytes per sample in each plane and treats endian as pixel-format naming only until conversion exists. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including new `gbrp12le` decoder/CLI coverage and `gbrp12be` mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 10-bit `gbrp10le` and `gbrp10be` support to the current shared pixel/rawvideo model. `PixelFormat` now reports both names in the inventory with RGB descriptor metadata, three modeled 10-bit components, 30 descriptor bits per pixel, three full-resolution GBR payload planes, no packed byte stride, no alpha flag, no float flag, and no chroma subsampling. The raw storage model uses two bytes per sample in each plane and treats endian as pixel-format naming only until conversion exists. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including new `gbrp10le` decoder/CLI coverage and `gbrp10be` mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 9-bit `gbrp9le` and `gbrp9be` support to the current shared pixel/rawvideo model. `PixelFormat` now reports both names in the inventory with RGB descriptor metadata, three modeled 9-bit components, 27 descriptor bits per pixel, three full-resolution GBR payload planes, no packed byte stride, no alpha flag, no float flag, and no chroma subsampling. The raw storage model uses two bytes per sample in each plane and treats endian as pixel-format naming only until conversion exists. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including new `gbrp9le` decoder/CLI coverage and `gbrp9be` mux sizing coverage. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, local `run --changed`, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 8-bit `gbrp` support to the current shared pixel/rawvideo model. `PixelFormat` now reports `gbrp` in the inventory with RGB descriptor metadata, three modeled 8-bit components, 24 bits per pixel, three full-resolution payload planes, no packed byte stride, no alpha flag, no float flag, and no chroma subsampling. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format, including a new `gbrp` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, affected crate clippy, local FATE-runner component mappings, workspace format check, workspace clippy, and `git diff --check` with CRLF warnings only. Broad `run --changed` and full workspace test execution remain blocked by Windows Application Control in specific target caches, while the corresponding focused component gates passed. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed floating grayscale formats `grayf16le`, `grayf16be`, `grayf32le`, and `grayf32be` to the current shared pixel/rawvideo model, including `yf32le` and `yf32be` aliases for the 32-bit pair. `PixelFormat` now reports these names in the inventory with grayscale descriptor metadata, one modeled component, 16 or 32 bits per component and pixel, one packed payload plane, no alpha flag, a float flag, and 2-byte or 4-byte frame sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including a new `grayf32le` CLI test. Validation passed with pinned-source descriptor verification, focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed 32-bit grayscale formats `gray32le` and `gray32be` to the current shared pixel/rawvideo model, including `y32le` and `y32be` aliases. `PixelFormat` now reports these names in the inventory with grayscale descriptor metadata, one modeled component, 32 bits per component and pixel, one packed payload plane, no alpha flag, and 4-byte-per-pixel sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including a new `gray32le` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed 16-bit gray+alpha formats `ya16le` and `ya16be` to the current shared pixel/rawvideo model. `PixelFormat` now reports these names in the inventory with grayscale descriptor metadata, two modeled components, 16 bits per component, 32 bits per pixel, one packed payload plane, alpha flag, and 4-byte-per-pixel sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including a new `ya16le` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed 64-bit RGBA/BGRA formats `rgba64le`, `rgba64be`, `bgra64le`, and `bgra64be` to the current shared pixel/rawvideo model. `PixelFormat` now reports these names in the inventory with RGB descriptor metadata, four modeled color/alpha components, 16 bits per component, 64 bits per pixel, one packed payload plane, alpha flag, and 8-byte-per-pixel sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including a new `rgba64le` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and `git diff --check` with CRLF warnings only. A single `cargo test --workspace --all-features --lib` invocation remains blocked by Windows Application Control in whichever target directory is used, but the changed crate tests plus blocked zero-test crate binaries passed individually through accepted target caches. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed gray+alpha `ya8` support to the current shared pixel/rawvideo model, including `gray8a` and `y400a` aliases. `PixelFormat` now reports `ya8` in the inventory with grayscale descriptor metadata, two modeled 8-bit components, 16 bits per pixel, one packed payload plane, alpha flag, and 2-byte-per-pixel sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format, including a new `ya8` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed 48-bit RGB/BGR formats `rgb48le`, `rgb48be`, `bgr48le`, and `bgr48be` to the current shared pixel/rawvideo model. `PixelFormat` now reports these names in the inventory with RGB descriptor metadata, three modeled color components, 16 bits per component, 48 bits per pixel, one packed payload plane, no alpha flag, and 6-byte-per-pixel sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including a new `rgb48le` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed 16-bit grayscale formats `gray16le` and `gray16be` to the current shared pixel/rawvideo model. `PixelFormat` now reports these names in the inventory with grayscale descriptor metadata, one modeled component, 16 bits per component and pixel, one packed payload plane, no alpha flag, and 2-byte-per-pixel sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including a new `gray16le` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added packed 32-bit no-alpha RGB padding formats `0rgb`, `rgb0`, `0bgr`, and `bgr0` to the current shared pixel/rawvideo model. `PixelFormat` now reports these names in the inventory with RGB descriptor metadata, three modeled color components, 32 bits per pixel, one packed payload plane, no alpha flag, and 4-byte-per-pixel sizing. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the formats, including a new `rgb0` CLI test. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 8-bit `yuv440p` support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuv440p` in the inventory with YUV descriptor metadata, validates height divisible by 2 while permitting odd width, computes 4:4:0 plane sizes, and splits frame payloads by plane. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 8-bit `yuv410p` support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuv410p` in the inventory with YUV descriptor metadata, validates width and height divisible by 4, computes 4:1:0 plane sizes, and splits frame payloads by plane. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 8-bit `yuv411p` support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuv411p` in the inventory with YUV descriptor metadata, validates width divisible by 4 while permitting odd height, computes 4:1:1 plane sizes, and splits frame payloads by plane. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 8-bit `yuv444p` support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuv444p` in the inventory with YUV descriptor metadata, computes full-resolution three-plane 4:4:4 sizes without chroma parity constraints, and splits frame payloads by plane. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and workspace library tests. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added planar 8-bit `yuv422p` support to the current shared pixel/rawvideo model. `PixelFormat` now reports `yuv422p` in the inventory with YUV descriptor metadata, validates even width while permitting odd height, computes 4:2:2 plane sizes, and splits frame payloads by plane. `VideoFrame`, `RawVideoDecoder`, `RawVideoDemuxer`, `RawVideoMuxer`, constrained `ffmpeg-rs` rawvideo-to-null execution, and the affected fuzz harnesses now exercise the format. Validation passed with focused avutil/avcodec/avformat/fftools tests, touched fuzz-target check/clippy, touched crate clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`, because oracle differentials, upstream FATE media coverage, full pixel inventory coverage, and actual fuzz execution are still absent.

Latest slice: added descriptor-style pixel format metadata for the current `avutil::PixelFormat` subset. `PixelFormatDescriptor` and `PixelFormatClass` are exported from `avutil`; `PixelFormat` now reports class, component count, bits per component, average bits per pixel, packed bytes per pixel, alpha status, plane count, and log2 chroma subsampling. Unit coverage now includes `pixel_formats_report_descriptor_metadata`, and `avutil_core_models` build-checks descriptor/name/class/component/bit-depth/chroma invariants against frame-size and plane-splitting behavior. Validation passed with focused pixel tests, fuzz target check/clippy, `avutil` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added packed RGB-family pixel formats to the current shared model. `PixelFormat` now includes `bgr24`, `bgra`, `argb`, and `abgr` alongside `gray`/`gray8`, `rgb24`, `rgba`, and `yuv420p`, exposes `ALL`, packed/planar metadata, alpha metadata, and packed bytes-per-pixel metadata, and feeds updated frame line-size and frame-size math through rawvideo decode/demux/mux paths. `ffmpeg-rs` rawvideo parsing now uses the shared lookup, so uppercase `BGRA` rawvideo input is accepted by the constrained `-f null -` path covered by a new unit test. The touched avutil, avcodec, avformat, fftools, and fuzz harness coverage passed; local FATE mappings were added for rawvideo demuxer/muxer plus WAV, raw PCM, and yuv4mpegpipe muxer smoke coverage selected by the shared basic-muxer fuzz target. The affected components remain `implemented`, not `complete`.

Latest slice: added narrow parser and mask support for `avutil-channel-layout`. `Channel` now exposes the modeled channel inventory, case-insensitive short-name lookup, and native mask bits; `ChannelLayout` now exposes known-layout enumeration, mask lookup, `from_channels`, canonical `FL+FR`-style serialization, and `parse` for current named layouts or channel expressions that canonicalize to one of the six modeled layouts. Invalid empty, NUL-containing, unknown, duplicate, and unsupported custom expressions are rejected with typed invalid-argument errors. Unit coverage now includes mask/string canonicalization, named/expression parsing, and invalid-expression rejection; `avutil_core_models` build-checks name, mask, and expression round trips plus duplicate/unsupported rejection. Validation passed with focused channel-layout tests, fuzz target check/clippy, avutil clippy, local FATE-runner avutil-channel-layout and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added escaped pair serialization/parsing for `avutil-dict`. `Dictionary::to_pairs_string` emits insertion-ordered key/value pairs with backslash escaping for the configured separators and literal backslashes; `Dictionary::parse_pairs` and `parse_pairs_into` parse escaped strings with configurable separator sets, use the caller-selected match/set modes, validate separators before mutation, reject malformed tokens, and preserve successfully parsed entries on later parse failure. Unit coverage now includes duplicate-key round trips, escaped separator round trips, append-mode parsing, partial-success failures, invalid separator rejection, dangling escapes, and empty-key rejection; `avutil_metadata_options` build-checks parser/serializer round trips and malformed-input paths. Validation passed with focused dict tests, fuzz target check/clippy, avutil clippy, local FATE-runner avutil-dict and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added parent-mediated child option mutation for `avutil-options`. `OptionSet` now exposes `child_mut`, `get_child_option`, `child_range`, `set_child`, and `set_child_from_str`, while `OptionChild` exposes `options_mut`; child setters preserve existing values on missing-child, missing-option, read-only, type, and range failures. Unit coverage now includes root/child namespace independence, child unit-constant parsing, child range lookup, and error preservation; `avutil_metadata_options` build-checks generated child option mutations and failed-mutation invariants. Validation passed with focused option tests, fuzz target check/clippy, avutil clippy, local FATE-runner avutil-options and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added bit I/O cursor seeking and aligned-byte writer support. `BitReader` now has `peek_bit`, `set_bit_position`, `seek_bits`, and `rewind` with invalid-seek cursor preservation; `BitWriter::write_aligned_bytes` appends raw bytes only when byte-aligned and validates bit-count overflow before mutation. Unit coverage now includes single-bit peeks, checked bit seeks, aligned-byte append round trips, and unaligned no-mutation behavior; `avutil_bitreader` build-checks generated seek, rewind, peek, aligned-byte write, cursor, and buffer-mutation invariants. Validation passed with focused bitreader/bitwriter tests, fuzz-target build/clippy, avutil clippy, local FATE-runner bitreader/bitwriter and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The components remain `implemented`, not `complete`.

Latest slice: added checked byte I/O random-access and patching support. `ByteReader` now has `set_position`, `seek_relative`, and `rewind` with invalid-seek cursor preservation; `ByteWriter` now has append-position, clear, checked truncate, raw `patch_all`, and endian-aware patch helpers for signed/unsigned 8/16/24/32/48/64-bit values with constrained-width validation before mutation. Unit coverage now includes checked reader seeking, writer patch/truncate round trips, and no-mutation error cases; `avutil_byteio` build-checks generated seek, patch, truncate, cursor, and buffer-mutation invariants. Validation passed with focused byteio tests, fuzz-target build/clippy, avutil clippy, local FATE-runner avutil-byteio and changed mappings, workspace format check, workspace clippy, workspace library tests, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added SHA-1/SHA-160 hash support. `avutil::Sha1` and `avutil::sha1` implement SHA-1 with standard vectors and streaming-boundary tests; `avformat::HashAlgorithm::Sha160` renders `SHA160` and routes through the shared hash, framehash, and streamhash muxer state; `ffmpeg-rs` accepts `-hash sha1`, `-hash sha160`, and normalized `-hash sha-160` for constrained hash output; and the avutil/avformat fuzz harnesses now build-check SHA-1/SHA-160 invariants. Local avformat packet-muxer FATE mappings were added so `run --changed` can exercise null/hash/framecrc/framehash/streamhash unit filters. Validation passed with focused hash/muxer/ffmpeg tests, affected-crate clippy, fuzz package check/clippy, local FATE-runner avutil-hash and changed mappings, workspace format check, workspace clippy, workspace library compile check, full workspace library tests, and `git diff --check` with CRLF warnings only. The affected components remain `implemented`, not `complete`.

Latest slice: routed process-level CLI diagnostics through the shared logger. `fftools::cli_logging` now formats diagnostics with `avutil::Logger`, preserving the existing one-error runtime path while enabling repeat-summary compression for consecutive identical diagnostics when `-loglevel repeat+...` sets `LogFlags::SKIP_REPEATED`. New tests cover compression under `repeat+level+error` and preservation of repeated lines without `repeat`; the ledger and docs now record this as local coverage, not upstream parity. Validation passed with focused `cli_logging` tests, full `fftools` library tests, `fftools` clippy, local FATE-runner option-parser and changed mappings, workspace format check, workspace clippy, workspace library compile check, full workspace library tests, and `git diff --check` with CRLF warnings only. One fresh target-dir test launch was blocked by Windows Application Control and passed through the already allowed target dir. The component remains `implemented`, not `complete`.

Latest slice: added deterministic terminal-color coverage for process-level CLI error formatting. `fftools::cli_logging` now has a test-only injection path that feeds env presence and terminal state into the same shared `LogFormatOptions` color resolver used by runtime entrypoint errors, and unit tests cover terminal stderr coloring, non-terminal plain output, and `AV_LOG_FORCE_NOCOLOR` precedence over terminal detection. The `fftools-option-parser` ledger now lists the new tests and documents the deterministic coverage; compatibility/oracle docs mention the terminal decision coverage and local CLI logging FATE mapping. Validation passed with focused `cli_logging` tests, full `fftools` library tests, `fftools` clippy, local FATE-runner option-parser and changed mappings, workspace format check, workspace clippy, workspace library compile check, full workspace library tests, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added terminal-aware log color resolution. `LogColorMode::from_ffmpeg_env` now checks `stderr().is_terminal()` after applying `AV_LOG_FORCE_NOCOLOR`/`AV_LOG_FORCE_COLOR` precedence, and `LogFormatOptions` exposes a deterministic helper that injects terminal state for tests and fuzz harnesses. Unit coverage now checks terminal stderr color enablement and no-color precedence when stderr is terminal; `avutil_core_models` build-checks matching invariants. Focused logging tests, fuzz-target build/clippy, `avutil` clippy, local FATE-runner logging and changed mappings, workspace format check, workspace clippy, workspace lib `--no-run` compile check, and `git diff --check` pass. Runtime workspace lib execution is blocked by Windows Application Control on a rebuilt `avcodec` test executable. The component remains `implemented`, not `complete`.

Latest slice: wired process-level CLI error stderr to the shared forced-color resolver. `fftools::cli_logging::tool_error_stderr` now builds `LogFormatOptions` from the parsed log flags and applies `AV_LOG_FORCE_COLOR`/`AV_LOG_FORCE_NOCOLOR` environment resolution before formatting entrypoint errors. Deterministic tests cover forced-color output and no-color precedence without changing process environment. Validation passed with focused `cli_logging` tests, full `fftools` library tests, `fftools` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and workspace library tests. The component remains `implemented`, not `complete`.

Latest slice: added deterministic forced-color environment-variable resolution to `avutil` logging. `LogColorMode::from_ffmpeg_env_vars`/`from_ffmpeg_env` map FFmpeg's `AV_LOG_FORCE_NOCOLOR` and `AV_LOG_FORCE_COLOR` presence to explicit color modes with no-color precedence, and `LogFormatOptions` exposes matching helpers for callers that need env-derived formatting. Unit coverage now checks forced-color, default no-color, no-color precedence, and formatted output; `avutil_core_models` build-checks the same resolver invariants. Focused logging tests, fuzz-target build/clippy, `avutil` clippy, local FATE-runner component/changed mappings, formatting checks, and `git diff --check` passed; the first sandboxed focused test binary launch and one target-specific `fate-runner` launch were blocked by Windows Application Control, then passed through fallback execution paths. The component remains `implemented`, not `complete`.

Latest slice: added `time`/`datetime` handling to the shared process-level CLI error formatter. `fftools::cli_logging` now attaches a current `LogTimestamp` before formatting `ffmpeg-rs`/`ffprobe-rs` entrypoint errors when timestamp flags are active; deterministic tests inject a fixed timestamp for `time`, `datetime`, datetime precedence with `level`, and clock-unavailable fallback behavior. Validation passed with focused helper tests, `fftools` library tests, `fftools` clippy, local FATE-runner component and changed mappings, workspace format check, workspace clippy, and workspace library tests. The component remains `implemented`, not `complete`.

Latest slice: added shared CLI error logging for process-level `ffmpeg-rs` and `ffprobe-rs` errors. New `fftools::cli_logging` derives loglevel flags from argv with the existing parser model, formats errors through `avutil::LogRecord`, preserves default `ffmpeg: error`/`ffprobe: error` text, suppresses output for quiet, and emits severity prefixes when `level` is active. `fate-runner` now maps the new helper path to `fftools-option-parser`, and `tests/fate/mappings.txt` runs a local CLI logging unit filter for that component. Validation passed with focused helper tests, `fftools` library tests, `fate-runner` tests, targeted and workspace clippy, component and changed FATE mappings, workspace lib tests, format check, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added explicit logging color formatting. `LogFormatOptions` now carries formatting flags plus a `LogColorMode`, `LogRecord::format_line_with_options` applies opt-in ANSI yellow/red severity coloring, `Logger::formatted_records_with_options` and `global_formatted_log_records_with_options` expose the same formatting path, and default formatted output remains uncolored. Unit coverage checks record-level colors, uncolored info/repeat summaries, logger repeat summaries under color mode, and global formatted-record color output; `avutil_core_models` now build-checks the deterministic color invariants. Validation passed with formatting, focused logging tests, fuzz-target check/clippy, avutil clippy, workspace clippy, local FATE-runner logging/changed mappings, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added `LogTimestamp::from_system_time`, `LogTimestamp::now_utc`, and `LogRecord::with_current_timestamp`. Unit coverage now checks exact post-epoch conversion, pre-epoch conversion, positive truncation below one microsecond, internal negative floor behavior below one microsecond, current timestamp bounds, and timestamp attachment; `avutil_core_models` mirrors deterministic system-time conversion and current timestamp attachment. Validation passed with workspace format check, focused logging tests, fuzz-target check/clippy, avutil clippy, workspace clippy, local FATE-runner logging and changed mappings, and `git diff --check` with CRLF warnings only. The separate fuzz-package rustfmt check still overflows rustfmt's stack, so the touched fuzz target was validated through build and clippy instead. The component remains `implemented`, not `complete`.

Latest slice: added a process-global logging primitive on top of the in-memory `Logger`. The new global helpers configure shared level and flags, install or clear a global callback, submit records, flush repeated summaries, inspect formatted records, clear state, and drain records. Unit coverage serializes global-state tests and checks filtering, shared flag updates, repeated-summary flushing, callback delivery, callback clearing, and record draining; `avutil_core_models` now build-checks global logger invariants alongside repeat, callback, and timestamp logging invariants. Validation passed with focused logging tests, fuzz-target check/clippy, avutil clippy, format check, workspace clippy, local FATE-runner logging and changed avutil mappings, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added deterministic repeated-message compression to the in-memory logging primitive. `Logger` now suppresses consecutive identical accepted records under `LogFlags::SKIP_REPEATED`, exposes pending repeat summaries through `formatted_records()`, materializes summaries through `flush_repeated()` and `take_records()`, flushes pending summaries when repeat suppression is disabled, drops pending state on `clear()`, and only invokes per-call callbacks for emitted records. Unit coverage now exercises suppression, summary ordering, take/clear/flag-change behavior, non-skip retention, callback behavior, and summary formatting; `avutil_core_models` now build-checks repeat compression invariants. Validation passed with focused logging tests, fuzz-target check/clippy, avutil clippy, format check, workspace clippy, local FATE-runner logging and changed avutil mappings, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added rational `OptionValue`/`OptionKind` support to the AVOption-like model. Rational options parse `num/den` and integer strings, reject malformed fractions and non-positive denominators, validate typed/default/range values, and expose rational `OptionRange` bounds. Unit coverage now exercises valid rational parse paths, integer-form parsing, invalid definitions/defaults/ranges, typed mismatches, out-of-range values, and malformed string inputs; `avutil_metadata_options` now build-checks generated rational definitions, typed values, parsed strings, range invariants, and failed-mutation preservation. Validation passed with focused option tests, fuzz-target check/clippy, avutil clippy, workspace clippy, local FATE-runner options/changed mappings, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added `Dictionary::matching_entries` and `Dictionary::prefixed_entries` for ordered exact and prefix scans across duplicate metadata keys. Unit coverage now checks duplicate exact-match iteration, case-sensitive filtering, prefix ordering, and empty-prefix all-entry scans; `avutil_metadata_options` now build-checks iterator first-match consistency plus generated exact/prefix match predicates. Focused dict tests, fuzz-target build/clippy checks, workspace clippy, and local FATE-runner dict/changed mappings pass; the first target-specific FATE-runner launch was blocked by Windows Application Control and passed when rerun through the default target cache. The component remains `implemented`, not `complete`.

Latest slice: added `ByteReader::peek_exact` plus endian-aware unsigned and signed 8/16/24/32/48/64-bit peek helpers. The peek paths share checked EOF/overflow validation with reads and never advance the cursor on success or failure. Unit coverage now checks unsigned peeks, signed sign extension, read-after-peek behavior, EOF preservation, and overflow errors; `avutil_byteio` now build-checks peek operations and cursor preservation in its generated operation loop. Focused `avutil` byteio tests and fuzz-target build checks pass. The component remains `implemented`, not `complete`.

Latest slice: added `AV_TIME_BASE`, `AV_TIME_BASE_Q`, `rescale`, `rescale_rnd`, and `rescale_rnd_pass_minmax`, and routed `rescale_q`/`rescale_q_rnd`/`rescale_q_rnd_pass_minmax` through the same checked multiplier/divisor core. Unit and fuzz-harness coverage now checks direct integer-term rounding, direct pass-min/max sentinel preservation, invalid term rejection, overflow rejection, constants, and rational rescale behavior against an independent i128 model. Validation passed with focused timebase tests, fuzz target check/clippy, avutil clippy, format check, workspace clippy, and FATE-runner timebase plus changed avutil mappings. The component remains `implemented`, not `complete`.

Latest slice: added `Rational::reduce_i64`, `Rational::from_f64_limited`, and `Rational::to_f64`. The reduction path reports whether the bounded result is exact, preserves raw zero/infinity sentinel shapes where FFmpeg's rational API permits them, approximates oversized finite fractions within caller limits, and rejects invalid max bounds. The f64 conversion path preserves finite bounded conversion plus NaN and infinity/overlarge sentinels. Unit and fuzz-harness coverage now checks exact reductions, approximation under tight limits, invalid limits, finite f64 conversions, bounded approximations, sentinel handling, reduction bounds, and exactness cross-products. Validation passed with focused rational tests, fuzz target check/clippy, avutil clippy, format check, workspace clippy, and FATE-runner rational plus changed avutil mappings. The component remains `implemented`, not `complete`.

Latest slice: added `AvErrorCode`, `AV_ERROR_MAX_STRING_SIZE`, documented tag-based FFmpeg `AVERROR_*` constants, `AvError::with_code`, and `AvError::code`. Constructors now attach unambiguous FFmpeg code metadata for invalid data, EOF, external, and bug errors; IO-derived UnexpectedEof and InvalidData errors preserve the corresponding FFmpeg code while platform errno-shaped cases remain code-less. Unit and fuzz-harness coverage now checks FFERRTAG values, raw-code round trips, custom-code preservation, constructor code metadata, IO-code mapping, and EOF predicates. Validation passed with focused error tests, fuzz target check/clippy, avutil clippy, format check, workspace clippy, and FATE-runner avutil-error plus changed avutil mappings. The component remains `implemented`, not `complete`.

Latest slice: added `PacketOpaque` and `Packet::opaque`/`opaque_address`/set/take/clear helpers for raw `AVPacket.opaque` parity without unsafe dereference. `copy_props_from` now copies raw opaque metadata, `ref_from` clones it, `move_ref_from` transfers it, and `unref` resets it. Unit and fuzz-harness coverage now checks nullable zero handling, nonzero address preservation, copy-props payload preservation, ref/move/unref lifecycle behavior, and independence from `opaque_ref`. Validation passed with focused packet tests, fuzz target check/clippy, avutil clippy, format check, workspace clippy, FATE-runner packet and changed avutil mappings, and `git diff --check` with CRLF warnings only. The component remains `implemented`, not `complete`.

Latest slice: added packet `AV_PKT_DATA_IAMF_MIX_GAIN_PARAM`, `AV_PKT_DATA_IAMF_DEMIXING_INFO_PARAM`, and `AV_PKT_DATA_IAMF_RECON_GAIN_INFO_PARAM` parsing. `PacketIamfParamDefinition` models FFmpeg's native `AVIAMFParamDefinition` side-data envelope with subblock offsets, subblock sizes, subblock counts, definition type, parameter ID/rate/duration fields, parsed length, and trailing-byte preservation; typed subblock views expose mix-gain animation and rationals, demixing `dmixp_mode`, and recon-gain 6x12 byte tables. Focused packet unit tests cover valid constructors/deferred accessors for all three IAMF side-data kinds, raw byte preservation, field access, trailing-byte preservation, non-matching kind behavior, wrong definition type, zero parameter rate, invalid offsets/sizes, truncation, zero subblock durations, invalid mix-gain animation values, and deferred accessor errors; `avutil_core_models` now build-checks the same invariants. Validation passed with focused IAMF and packet tests, fuzz target check/clippy, avutil clippy, format check, workspace clippy, FATE-runner packet and changed mappings, `fftools` library fallback tests, bin compile checks, and `git diff --check`. `xtask quick` passed its runtime guard plus avutil and fftools library tests, then failed only because Windows Application Control blocked the rebuilt `ffprobe-rs` bin-test executable. The component remains `implemented`, not `complete`.
