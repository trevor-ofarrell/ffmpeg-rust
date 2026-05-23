use fftools::ffmpeg_output;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_rgb24_file_output_matches_ffmpeg_oracle() {
    compare_rawvideo_file_output("rgb24", "2x1", "25", &(0_u8..12).collect::<Vec<_>>());
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_gbrp10msble_file_output_matches_ffmpeg_oracle() {
    compare_rawvideo_file_output("gbrp10msble", "2x1", "25", &(0_u8..24).collect::<Vec<_>>());
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_rgb24_framecrc_records_match_ffmpeg_oracle() {
    compare_rawvideo_framecrc_records("rgb24", "2x1", "25", &(0_u8..12).collect::<Vec<_>>());
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_rgb24_framehash_records_match_ffmpeg_oracle() {
    compare_rawvideo_frame_checksum_records(
        "rgb24",
        "2x1",
        "25",
        &(0_u8..12).collect::<Vec<_>>(),
        "framehash",
        &["-c:v", "copy"],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_rgb24_framemd5_records_match_ffmpeg_oracle() {
    compare_rawvideo_frame_checksum_records(
        "rgb24",
        "2x1",
        "25",
        &(0_u8..12).collect::<Vec<_>>(),
        "framemd5",
        &["-c:v", "copy"],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_rgb24_streamhash_records_match_ffmpeg_oracle() {
    compare_rawvideo_frame_checksum_records(
        "rgb24",
        "2x1",
        "25",
        &(0_u8..12).collect::<Vec<_>>(),
        "streamhash",
        &["-c:v", "copy"],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_rgb24_hash_records_match_ffmpeg_oracle() {
    compare_rawvideo_frame_checksum_records(
        "rgb24",
        "2x1",
        "25",
        &(0_u8..12).collect::<Vec<_>>(),
        "hash",
        &["-c:v", "copy"],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_rgb24_md5_records_match_ffmpeg_oracle() {
    compare_rawvideo_frame_checksum_records(
        "rgb24",
        "2x1",
        "25",
        &(0_u8..12).collect::<Vec<_>>(),
        "md5",
        &["-c:v", "copy"],
    );
}

fn compare_rawvideo_file_output(pixel_format: &str, size: &str, rate: &str, payload: &[u8]) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes(&format!("{pixel_format}-input"), "raw", payload);
    let rust_output_path = unique_temp_path(&format!("{pixel_format}-rust-output"), "raw");
    let oracle_output_path = unique_temp_path(&format!("{pixel_format}-oracle-output"), "raw");

    let input_arg = input_path.to_string_lossy().into_owned();
    let rust_output_arg = rust_output_path.to_string_lossy().into_owned();
    let oracle_output_arg = oracle_output_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "rawvideo",
        "-pix_fmt",
        pixel_format,
        "-s",
        size,
        "-r",
        rate,
        "-i",
        input_arg.as_str(),
        "-f",
        "rawvideo",
        rust_output_arg.as_str(),
    ]))
    .expect("Rust rawvideo file-output path should execute");

    let oracle_status = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            pixel_format,
            "-s",
            size,
            "-r",
            rate,
            "-i",
            input_arg.as_str(),
            "-c:v",
            "copy",
            "-f",
            "rawvideo",
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

    assert_eq!(rust.output_format(), Some("rawvideo"));
    assert_eq!(rust.byte_count(), u64::try_from(payload.len()).unwrap());
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());
    assert_eq!(rust_bytes, oracle_bytes);
}

fn compare_rawvideo_framecrc_records(pixel_format: &str, size: &str, rate: &str, payload: &[u8]) {
    compare_rawvideo_frame_checksum_records(
        pixel_format,
        size,
        rate,
        payload,
        "framecrc",
        &["-c:v", "copy"],
    );
}

fn compare_rawvideo_frame_checksum_records(
    pixel_format: &str,
    size: &str,
    rate: &str,
    payload: &[u8],
    output_format: &str,
    oracle_extra_args: &[&str],
) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes(
        &format!("{pixel_format}-{output_format}-input"),
        "raw",
        payload,
    );
    let input_arg = input_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "rawvideo",
        "-pix_fmt",
        pixel_format,
        "-s",
        size,
        "-r",
        rate,
        "-i",
        input_arg.as_str(),
        "-f",
        output_format,
        "-",
    ]))
    .unwrap_or_else(|err| panic!("Rust rawvideo {output_format} path should execute: {err}"));

    let mut oracle_args = strings(&[
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "rawvideo",
        "-pix_fmt",
        pixel_format,
        "-s",
        size,
        "-r",
        rate,
        "-i",
        input_arg.as_str(),
    ]);
    oracle_args.extend(oracle_extra_args.iter().map(|arg| (*arg).to_string()));
    oracle_args.extend(strings(&["-f", output_format, "-"]));

    let oracle_output = Command::new(&oracle)
        .args(&oracle_args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    remove_temp_files(&[input_path]);

    assert!(
        oracle_output.status.success(),
        "oracle {output_format} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle checksum output should be UTF-8");

    assert_eq!(rust.output_format(), Some(output_format));
    assert_eq!(rust.packet_count(), 2);
    assert_eq!(rust.byte_count(), u64::try_from(payload.len()).unwrap());
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_frame_checksum_records(rust.stdout()),
        normalize_frame_checksum_records(&oracle_stdout)
    );
}

fn normalize_frame_checksum_records(output: &str) -> Vec<String> {
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
