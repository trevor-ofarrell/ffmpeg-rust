# Compatibility

## Target

- FFmpeg version: 8.1.1 "Hoare"
- Release branch: `release/8.1`
- Initial profile: `ffmpeg-8.1.1-default-native`
- Oracle configure baseline: `./configure --disable-gpl --disable-nonfree --disable-doc`

## Compatible Today

- `ffmpeg-rs -version` prints a version banner naming the pinned target and ABI versions.
- `ffprobe-rs -version` prints a version banner naming the pinned target and ABI versions.
- `avutil` has initial typed errors, rational normalization/comparison, timestamp rescaling, bounded byte I/O helpers, MSB-first bit reader/writer helpers, packet timestamp/flag/side-data skeletons, frame shape validation, and an in-memory logging abstraction.
- `oracle inventory` can execute a pinned FFmpeg binary and capture the required inventory command outputs.

## Incomplete

All media parsing, decoding, encoding, muxing, demuxing, filtering, playback, probing, stream mapping, option ordering, metadata semantics, hardware acceleration, devices, FATE execution, fuzzing, and differential media parity remain incomplete unless a later ledger entry proves otherwise.

## Known Behavior Deltas

- CLI support is limited to `-version`; most FFmpeg/ffprobe options return unsupported-command errors.
- The version banner is compatibility-oriented but not byte-identical to upstream FFmpeg.
- No inventory snapshot has been generated because no pinned FFmpeg oracle binary exists in this workspace yet.
- FATE components are listed from the ledger, but runnable FATE mappings have not been implemented.
