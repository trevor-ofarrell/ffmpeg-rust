use fftools::ffmpeg_output;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const DEFAULT_FATE_WAV_SAMPLE: &str = "audio-reference/luckynight_2ch_44kHz_s16.wav";

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle plus FATE_WAV_SAMPLE or FATE_SAMPLES"]
fn wav_pcm_s16le_md5_matches_ffmpeg_oracle_sample() {
    let sample = fate_wav_sample();
    compare_wav_md5(&sample);
}

fn compare_wav_md5(sample_path: &Path) {
    assert!(
        sample_path.is_file(),
        "FATE WAV sample must point to an existing PCM s16le WAV file, got `{}`",
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
        let path = PathBuf::from(path);
        assert!(
            path.is_file(),
            "FFMPEG_ORACLE must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
            path.display()
        );
        return path;
    }

    let root = repo_root();
    let unix_path = root.join("third_party/ffmpeg-oracle/build/bin/ffmpeg");
    if unix_path.is_file() {
        return unix_path;
    }

    let windows_path = root.join("third_party/ffmpeg-oracle/build/bin/ffmpeg.exe");
    if windows_path.is_file() {
        return windows_path;
    }

    panic!(
        "missing pinned FFmpeg oracle; set FFMPEG_ORACLE or install `{}`",
        unix_path.display()
    );
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
