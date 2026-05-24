use avformat::WavMuxer;
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
    assert_eq!(rust.packet_count(), 1);
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
