use avformat::WavDemuxer;
use fftools::ffmpeg_output;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_framecrc_records_match_ffmpeg_oracle() {
    let payload = (0_u8..16).collect::<Vec<_>>();
    compare_pcm_s16le_framecrc_records("48000", "2", &payload);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_empty_framecrc_records_match_ffmpeg_oracle() {
    compare_pcm_s16le_framecrc_records("48000", "2", &[]);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_partial_framecrc_records_match_ffmpeg_oracle() {
    let payload = (0_u8..6).collect::<Vec<_>>();
    compare_pcm_s16le_framecrc_records("48000", "2", &payload);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_odd_packet_framecrc_records_match_ffmpeg_oracle() {
    let payload = vec![0_u8, 1, 2, 3, 4];
    compare_pcm_s16le_framecrc_records("48000", "2", &payload);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_three_channel_partial_framecrc_records_match_ffmpeg_oracle() {
    let payload = (0_u8..14).collect::<Vec<_>>();
    compare_pcm_s16le_framecrc_records("48000", "3", &payload);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_file_output_matches_ffmpeg_oracle() {
    let payload = (0_u8..16).collect::<Vec<_>>();
    compare_pcm_s16le_file_output("48000", "2", &payload);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_odd_packet_file_output_matches_ffmpeg_oracle() {
    let payload = vec![0_u8, 1, 2, 3, 4];
    compare_pcm_s16le_file_output("48000", "2", &payload);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn pcm_s16le_wav_file_output_matches_ffmpeg_oracle() {
    let payload = (0_u8..16).collect::<Vec<_>>();
    compare_pcm_s16le_wav_file_output("48000", "2", &payload);
}

fn compare_pcm_s16le_framecrc_records(sample_rate: &str, channels: &str, payload: &[u8]) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("pcm-s16le-framecrc-input", "raw", payload);
    let input_arg = input_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "s16le",
        "-ar",
        sample_rate,
        "-ac",
        channels,
        "-i",
        input_arg.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .expect("Rust raw PCM framecrc path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "s16le",
            "-ar",
            sample_rate,
            "-ac",
            channels,
            "-i",
            input_arg.as_str(),
            "-c:a",
            "copy",
            "-f",
            "framecrc",
            "-",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    remove_temp_files(&[input_path]);

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
    assert_eq!(rust.packet_count(), u64::from(!payload.is_empty()));
    assert_eq!(rust.byte_count(), u64::try_from(payload.len()).unwrap());
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_framecrc_records(rust.stdout()),
        normalize_framecrc_records(&oracle_stdout)
    );
}

fn compare_pcm_s16le_file_output(sample_rate: &str, channels: &str, payload: &[u8]) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("pcm-s16le-file-input", "raw", payload);
    let rust_output_path = unique_temp_path("pcm-s16le-rust-output", "raw");
    let oracle_output_path = unique_temp_path("pcm-s16le-oracle-output", "raw");

    let input_arg = input_path.to_string_lossy().into_owned();
    let rust_output_arg = rust_output_path.to_string_lossy().into_owned();
    let oracle_output_arg = oracle_output_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "s16le",
        "-ar",
        sample_rate,
        "-ac",
        channels,
        "-i",
        input_arg.as_str(),
        "-f",
        "s16le",
        rust_output_arg.as_str(),
    ]))
    .expect("Rust raw PCM file-output path should execute");

    let oracle_status = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "s16le",
            "-ar",
            sample_rate,
            "-ac",
            channels,
            "-i",
            input_arg.as_str(),
            "-c:a",
            "copy",
            "-f",
            "s16le",
            oracle_output_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        oracle_status.status.success(),
        "oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_status.status.code(),
        String::from_utf8_lossy(&oracle_status.stdout),
        String::from_utf8_lossy(&oracle_status.stderr)
    );

    let rust_bytes = fs::read(&rust_output_path).expect("Rust output should be readable");
    let oracle_bytes = fs::read(&oracle_output_path).expect("oracle output should be readable");

    remove_temp_files(&[input_path, rust_output_path, oracle_output_path]);

    assert_eq!(rust.output_format(), Some("s16le"));
    assert_eq!(rust.packet_count(), 1);
    assert_eq!(rust.byte_count(), u64::try_from(payload.len()).unwrap());
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());
    assert_eq!(rust_bytes, oracle_bytes);
}

fn compare_pcm_s16le_wav_file_output(sample_rate: &str, channels: &str, payload: &[u8]) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("pcm-s16le-wav-input", "raw", payload);
    let rust_output_path = unique_temp_path("pcm-s16le-rust-output", "wav");
    let oracle_output_path = unique_temp_path("pcm-s16le-oracle-output", "wav");

    let input_arg = input_path.to_string_lossy().into_owned();
    let rust_output_arg = rust_output_path.to_string_lossy().into_owned();
    let oracle_output_arg = oracle_output_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "s16le",
        "-ar",
        sample_rate,
        "-ac",
        channels,
        "-i",
        input_arg.as_str(),
        "-f",
        "wav",
        rust_output_arg.as_str(),
    ]))
    .expect("Rust raw PCM WAV file-output path should execute");

    let oracle_status = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "s16le",
            "-ar",
            sample_rate,
            "-ac",
            channels,
            "-i",
            input_arg.as_str(),
            "-c:a",
            "copy",
            "-f",
            "wav",
            oracle_output_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        oracle_status.status.success(),
        "oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_status.status.code(),
        String::from_utf8_lossy(&oracle_status.stdout),
        String::from_utf8_lossy(&oracle_status.stderr)
    );

    let rust_bytes = fs::read(&rust_output_path).expect("Rust WAV output should be readable");
    let oracle_bytes = fs::read(&oracle_output_path).expect("oracle WAV output should be readable");

    remove_temp_files(&[input_path, rust_output_path, oracle_output_path]);

    assert_eq!(rust.output_format(), Some("wav"));
    assert_eq!(rust.packet_count(), 1);
    assert_eq!(rust.byte_count(), u64::try_from(rust_bytes.len()).unwrap());
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());
    assert_eq!(rust_bytes, oracle_bytes);

    let mut demuxer = WavDemuxer::open(&oracle_bytes).expect("oracle WAV output should parse");
    assert_eq!(
        demuxer.info().sample_rate().to_string(),
        sample_rate,
        "oracle WAV output should preserve sample rate"
    );
    assert_eq!(
        demuxer.info().channels().to_string(),
        channels,
        "oracle WAV output should preserve channel count"
    );
    let packet = demuxer.read_packet().unwrap().unwrap();
    assert_eq!(packet.data(), payload);
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn normalize_framecrc_records(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .map(|line| line.split(',').map(str::trim).collect::<Vec<_>>().join("|"))
        .collect()
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

fn write_temp_bytes(label: &str, extension: &str, bytes: &[u8]) -> PathBuf {
    let path = unique_temp_path(label, extension);
    fs::write(&path, bytes).expect("temp media file should be writable");
    path
}

fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "ffmpegrust-oracle-{}-{label}-{unique}.{extension}",
        std::process::id()
    ))
}

fn remove_temp_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}
