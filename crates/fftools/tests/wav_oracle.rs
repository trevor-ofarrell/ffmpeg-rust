use avformat::{WavDemuxer, WavMuxer};
use avutil::Packet;
use fftools::ffmpeg_output;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_FATE_WAV_SAMPLE: &str = "audio-reference/luckynight_2ch_44kHz_s16.wav";

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_generated_md5_matches_ffmpeg_oracle() {
    let payload = [
        0x00, 0x00, 0x01, 0x00, 0xfe, 0xff, 0x7f, 0x00, 0x80, 0xff, 0x34, 0x12,
    ];
    let path = write_generated_wav("generated-pcm-s16le", 2, 44_100, &payload);

    compare_wav_md5(&path);
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_generated_framecrc_matches_ffmpeg_oracle() {
    let payload = [
        0x00, 0x00, 0x01, 0x00, 0xfe, 0xff, 0x7f, 0x00, 0x80, 0xff, 0x34, 0x12,
    ];
    let path = write_generated_wav("generated-pcm-s16le-framecrc", 2, 44_100, &payload);

    compare_wav_framecrc(&path, payload.len());
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_extensible_generated_framecrc_matches_ffmpeg_oracle() {
    let payload = [
        0x00, 0x00, 0x01, 0x00, 0xfe, 0xff, 0x7f, 0x00, 0x80, 0xff, 0x34, 0x12,
    ];
    let path = write_generated_extensible_wav(
        "generated-extensible-pcm-s16le-framecrc",
        2,
        44_100,
        &payload,
    );

    compare_wav_framecrc(&path, payload.len());
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_empty_data_framecrc_matches_ffmpeg_oracle() {
    let path = write_generated_wav("generated-empty-pcm-s16le-framecrc", 1, 44_100, &[]);

    compare_wav_framecrc(&path, 0);
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_duplicate_data_chunk_last_data_used_for_framecrc() {
    let first_payload = [0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00];
    let second_payload = [0xAA, 0x00, 0xBB, 0x00];
    let path = write_generated_wav_with_duplicate_data_chunks(
        "generated-pcm-s16le-duplicate-data-last-framecrc",
        2,
        44_100,
        &first_payload,
        &second_payload,
    );

    compare_wav_framecrc(&path, second_payload.len());
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_duplicate_data_chunk_last_data_empty() {
    let first_payload = [0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00];
    let path = write_generated_wav_with_duplicate_data_chunks(
        "generated-pcm-s16le-duplicate-data-last-empty-framecrc",
        2,
        44_100,
        &first_payload,
        &[],
    );

    compare_wav_framecrc(&path, 0);
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_duplicate_fmt_chunk_first_fmt_used_for_framecrc() {
    let payload = [0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00];
    let path = write_generated_wav_with_duplicate_fmt_chunks(
        "generated-pcm-s16le-duplicate-fmt-first-framecrc",
        1,
        44_100,
        2,
        48_000,
        &payload,
    );

    compare_wav_framecrc(&path, payload.len());
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_duplicate_fmt_short_second_is_ignored_for_framecrc() {
    let payload = [0x00, 0x00, 0x01, 0x00];
    let path = write_generated_wav_with_short_second_duplicate_fmt_chunk(
        "generated-pcm-s16le-duplicate-fmt-short-second-framecrc",
        1,
        44_100,
        &payload,
    );

    compare_wav_framecrc(&path, payload.len());
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_short_pcm_fmt_chunk_is_rejected_by_ffmpeg() {
    let path = write_generated_short_pcm_fmt_chunk_wav("generated-short-pcm-fmt-chunk");

    assert_oracle_rejects_wav_for_null_output(&path);
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn wav_pcm_s16le_empty_file_output_matches_ffmpeg_oracle() {
    let path = unique_temp_path("empty-pcm-s16le-file-output", "raw");
    fs::write(&path, []).expect("empty raw PCM fixture should be writable");

    compare_wav_file_output(&path);
    remove_temp_file(&path);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle plus FATE_WAV_SAMPLE or FATE_SAMPLES"]
fn wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample() {
    let sample = fate_wav_sample();
    compare_wav_md5(&sample);
}

fn compare_wav_framecrc(sample_path: &Path, payload_len: usize) {
    assert!(
        sample_path.is_file(),
        "WAV oracle input must point to an existing PCM s16le WAV file, got `{}`",
        sample_path.display()
    );

    let oracle = oracle_ffmpeg();
    let sample_arg = sample_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-hide_banner",
        "-i",
        sample_arg.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .expect("Rust WAV framecrc command path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            sample_arg.as_str(),
            "-c:a",
            "copy",
            "-f",
            "framecrc",
            "-",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        oracle_output.status.success(),
        "oracle framecrc failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle framecrc output should be UTF-8");

    assert_eq!(rust.output_format(), Some("framecrc"));
    assert_eq!(rust.packet_count(), u64::from(payload_len > 0));
    assert_eq!(rust.byte_count(), u64::try_from(payload_len).unwrap());
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_framecrc_records(rust.stdout()),
        normalize_framecrc_records(&oracle_stdout)
    );
}

fn compare_wav_md5(sample_path: &Path) {
    assert!(
        sample_path.is_file(),
        "WAV oracle input must point to an existing PCM s16le WAV file, got `{}`",
        sample_path.display()
    );

    let oracle = oracle_ffmpeg();
    let sample_arg = sample_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-hide_banner",
        "-i",
        sample_arg.as_str(),
        "-f",
        "md5",
        "-",
    ]))
    .expect("Rust WAV md5 command path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            sample_arg.as_str(),
            "-c:a",
            "copy",
            "-f",
            "md5",
            "-",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        oracle_output.status.success(),
        "oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle md5 output should be UTF-8");

    assert_eq!(rust.output_format(), Some("md5"));
    assert_eq!(rust.stdout().trim_end(), oracle_stdout.trim_end());
    assert!(rust.stderr().is_empty());
}

fn compare_wav_file_output(sample_path: &Path) {
    assert!(
        sample_path.is_file(),
        "WAV oracle input must point to an existing raw PCM s16le file, got `{}`",
        sample_path.display()
    );

    let oracle = oracle_ffmpeg();
    let sample_arg = sample_path.to_string_lossy().into_owned();
    let rust_output_path = unique_temp_path("generated-empty-pcm-s16le-rust-output", "wav");
    let oracle_output_path = unique_temp_path("generated-empty-pcm-s16le-oracle-output", "wav");
    let rust_output_arg = rust_output_path.to_string_lossy().into_owned();
    let oracle_output_arg = oracle_output_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "s16le",
        "-ar",
        "44100",
        "-ac",
        "1",
        "-i",
        sample_arg.as_str(),
        "-f",
        "wav",
        rust_output_arg.as_str(),
    ]))
    .expect("Rust WAV file-output path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "s16le",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-i",
            sample_arg.as_str(),
            "-c:a",
            "copy",
            "-f",
            "wav",
            oracle_output_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        oracle_output.status.success(),
        "oracle file-output failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let rust_bytes = fs::read(&rust_output_path).expect("Rust WAV output should be readable");
    let oracle_bytes = fs::read(&oracle_output_path).expect("oracle WAV output should be readable");

    remove_temp_file(&rust_output_path);
    remove_temp_file(&oracle_output_path);

    assert_eq!(rust.output_format(), Some("wav"));
    assert_eq!(rust.packet_count(), 0);
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());
    assert_eq!(rust.byte_count(), u64::try_from(rust_bytes.len()).unwrap());
    assert_eq!(rust_bytes, oracle_bytes);

    let mut demuxer = WavDemuxer::open(&rust_bytes).expect("Rust WAV output should parse");
    assert_eq!(demuxer.info().channels(), 1);
    assert_eq!(
        demuxer.info().channel_layout(),
        Some(avutil::ChannelLayout::mono())
    );
    assert_eq!(demuxer.info().sample_rate(), 44_100);
    assert_eq!(demuxer.info().data_size(), 0);
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn assert_oracle_rejects_wav_for_null_output(sample_path: &Path) {
    let oracle = oracle_ffmpeg();
    let sample_arg = sample_path.to_string_lossy().into_owned();

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            sample_arg.as_str(),
            "-f",
            "null",
            "-",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        !oracle_output.status.success(),
        "oracle unexpectedly succeeded for invalid WAV input `{}`\nstdout:\n{}\nstderr:\n{}",
        sample_path.display(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );
}

fn normalize_framecrc_records(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .map(|line| line.split(',').map(str::trim).collect::<Vec<_>>().join("|"))
        .collect()
}

fn fate_wav_sample() -> PathBuf {
    if let Ok(path) = env::var("FATE_WAV_SAMPLE") {
        let path = PathBuf::from(path);
        assert!(
            path.is_file(),
            "FATE_WAV_SAMPLE must point to a PCM s16le WAV FATE sample, got `{}`",
            path.display()
        );
        return path;
    }

    for env_var in ["FATE_SAMPLES", "SAMPLES"] {
        if let Ok(root) = env::var(env_var) {
            let path = default_fate_wav_sample(Path::new(&root));
            assert!(
                path.is_file(),
                "{env_var} must contain `{DEFAULT_FATE_WAV_SAMPLE}`, got missing `{}`",
                path.display()
            );
            return path;
        }
    }

    let repo_root = repo_root();
    for candidate in [
        repo_root.join("third_party/fate-samples"),
        repo_root.join("third_party/fate-suite"),
        repo_root.join("fate-suite"),
    ] {
        let path = default_fate_wav_sample(&candidate);
        if path.is_file() {
            return path;
        }
    }

    panic!(
        "missing FATE WAV sample; set FATE_WAV_SAMPLE or install `{DEFAULT_FATE_WAV_SAMPLE}` under FATE_SAMPLES"
    );
}

fn default_fate_wav_sample(root: &Path) -> PathBuf {
    root.join("audio-reference")
        .join("luckynight_2ch_44kHz_s16.wav")
}

fn oracle_ffmpeg() -> PathBuf {
    if let Ok(path) = env::var("FFMPEG_ORACLE") {
        let path = resolve_repo_relative_path(PathBuf::from(path));
        assert!(
            path.is_file(),
            "FFMPEG_ORACLE must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
            path.display()
        );
        return path;
    }

    let root = repo_root();
    for candidate in default_ffmpeg_candidates(&root) {
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "missing pinned FFmpeg oracle; set FFMPEG_ORACLE or install `{}`",
        root.join("third_party/ffmpeg-oracle/build/bin/ffmpeg")
            .display()
    );
}

fn default_ffmpeg_candidates(root: &Path) -> Vec<PathBuf> {
    let bin = root.join("third_party/ffmpeg-oracle/build/bin");
    if cfg!(windows) {
        vec![
            bin.join("ffmpeg.exe"),
            bin.join("ffmpeg.cmd"),
            bin.join("ffmpeg"),
        ]
    } else {
        vec![
            bin.join("ffmpeg"),
            bin.join("ffmpeg.exe"),
            bin.join("ffmpeg.cmd"),
        ]
    }
}

fn resolve_repo_relative_path(path: PathBuf) -> PathBuf {
    if path.is_file() || path.is_absolute() {
        return path;
    }
    let candidate = repo_root().join(&path);
    if candidate.is_file() {
        return candidate;
    }
    path
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("fftools crate should be under crates/")
        .to_path_buf()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn repo_relative_oracle_paths_resolve_from_workspace_root() {
    let resolved = resolve_repo_relative_path(PathBuf::from("crates/fftools/Cargo.toml"));
    assert!(
        resolved.is_file(),
        "repo-relative path should resolve to an existing file, got `{}`",
        resolved.display()
    );
}

fn write_generated_wav(label: &str, channels: u16, sample_rate: u32, payload: &[u8]) -> PathBuf {
    let path = unique_temp_path(label, "wav");
    let mut muxer =
        WavMuxer::new_pcm_s16le(channels, sample_rate).expect("generated WAV parameters are valid");
    muxer
        .write_packet(&Packet::new(payload.to_vec(), 0))
        .expect("generated WAV packet should be valid");
    let bytes = muxer.finish().expect("generated WAV should finish");
    fs::write(&path, bytes).expect("generated WAV fixture should be writable");
    path
}

fn write_generated_wav_with_duplicate_data_chunks(
    label: &str,
    channels: u16,
    sample_rate: u32,
    first_payload: &[u8],
    second_payload: &[u8],
) -> PathBuf {
    let path = unique_temp_path(label, "wav");
    let bytes =
        wav_multiple_data_chunks_bytes(channels, sample_rate, first_payload, second_payload);
    fs::write(&path, bytes).expect("generated duplicated data WAV fixture should be writable");
    path
}

fn write_generated_wav_with_duplicate_fmt_chunks(
    label: &str,
    first_channels: u16,
    first_sample_rate: u32,
    second_channels: u16,
    second_sample_rate: u32,
    payload: &[u8],
) -> PathBuf {
    let path = unique_temp_path(label, "wav");
    let bytes = wav_multiple_fmt_chunks_bytes(
        first_channels,
        first_sample_rate,
        second_channels,
        second_sample_rate,
        payload,
    );
    fs::write(&path, bytes).expect("generated duplicate fmt WAV fixture should be writable");
    path
}

fn write_generated_wav_with_short_second_duplicate_fmt_chunk(
    label: &str,
    first_channels: u16,
    first_sample_rate: u32,
    payload: &[u8],
) -> PathBuf {
    let path = unique_temp_path(label, "wav");
    let bytes = wav_multiple_fmt_chunks_with_short_second_payload_bytes(
        first_channels,
        first_sample_rate,
        payload,
    );
    fs::write(&path, bytes)
        .expect("generated truncated duplicate fmt WAV fixture should be writable");
    path
}

fn write_generated_short_pcm_fmt_chunk_wav(label: &str) -> PathBuf {
    let path = unique_temp_path(label, "wav");
    fs::write(&path, wav_short_pcm_fmt_chunk_bytes())
        .expect("generated short PCM fmt WAV fixture should be writable");
    path
}

fn write_generated_extensible_wav(
    label: &str,
    channels: u16,
    sample_rate: u32,
    payload: &[u8],
) -> PathBuf {
    let path = unique_temp_path(label, "wav");
    let bytes = wav_extensible_bytes(channels, sample_rate, payload);
    fs::write(&path, bytes).expect("generated extensible WAV fixture should be writable");
    path
}

fn wav_extensible_bytes(channels: u16, sample_rate: u32, payload: &[u8]) -> Vec<u8> {
    let block_align = channels * 2;
    let byte_rate = sample_rate * u32::from(block_align);

    let mut body = Vec::new();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&40_u32.to_le_bytes());
    body.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&sample_rate.to_le_bytes());
    body.extend_from_slice(&byte_rate.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());
    body.extend_from_slice(&22_u16.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());
    body.extend_from_slice(&3_u32.to_le_bytes());
    body.extend_from_slice(&[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B,
        0x71,
    ]);
    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(payload.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        body.push(0);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

fn wav_short_pcm_fmt_chunk_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&20_u32.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&8_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out
}

fn wav_multiple_data_chunks_bytes(
    channels: u16,
    sample_rate: u32,
    first_payload: &[u8],
    second_payload: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    let block_align = channels * 2;
    let byte_rate = sample_rate * u32::from(block_align);
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&sample_rate.to_le_bytes());
    body.extend_from_slice(&byte_rate.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());
    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(first_payload.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(first_payload);
    if first_payload.len() % 2 == 1 {
        body.push(0);
    }

    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(second_payload.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(second_payload);
    if second_payload.len() % 2 == 1 {
        body.push(0);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

fn wav_multiple_fmt_chunks_bytes(
    first_channels: u16,
    first_sample_rate: u32,
    second_channels: u16,
    second_sample_rate: u32,
    payload: &[u8],
) -> Vec<u8> {
    let first_block_align = first_channels * 2;
    let first_byte_rate = first_sample_rate * u32::from(first_block_align);
    let second_block_align = second_channels * 2;
    let second_byte_rate = second_sample_rate * u32::from(second_block_align);

    let mut body = Vec::new();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&first_channels.to_le_bytes());
    body.extend_from_slice(&first_sample_rate.to_le_bytes());
    body.extend_from_slice(&first_byte_rate.to_le_bytes());
    body.extend_from_slice(&first_block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());

    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&second_channels.to_le_bytes());
    body.extend_from_slice(&second_sample_rate.to_le_bytes());
    body.extend_from_slice(&second_byte_rate.to_le_bytes());
    body.extend_from_slice(&second_block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());

    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(payload.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        body.push(0);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

fn wav_multiple_fmt_chunks_with_short_second_payload_bytes(
    first_channels: u16,
    first_sample_rate: u32,
    payload: &[u8],
) -> Vec<u8> {
    let first_block_align = first_channels * 2;
    let first_byte_rate = first_sample_rate * u32::from(first_block_align);

    let mut body = Vec::new();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&first_channels.to_le_bytes());
    body.extend_from_slice(&first_sample_rate.to_le_bytes());
    body.extend_from_slice(&first_byte_rate.to_le_bytes());
    body.extend_from_slice(&first_block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());

    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&8_u32.to_le_bytes());
    body.extend_from_slice(&[1, 0, 1, 0, 0, 0, 0, 0]);

    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(payload.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        body.push(0);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "ffmpegrust-wav-oracle-{}-{label}-{unique}.{extension}",
        std::process::id()
    ))
}

fn remove_temp_file(path: &Path) {
    let _ = fs::remove_file(path);
}
