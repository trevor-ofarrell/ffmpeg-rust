use fftools::ffmpeg_output;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const PPM_1X1_RED: &[u8] = b"P6\n1 1\n255\n\xff\0\0";
const PPM_1X1_GREEN: &[u8] = b"P6\n1 1\n255\n\0\xff\0";

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_single_file_output_matches_ffmpeg_oracle() {
    compare_image2_file_output("ppm", "1", PPM_1X1_RED);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_single_framecrc_records_match_ffmpeg_oracle() {
    compare_image2_single_framecrc_records("ppm", "1", PPM_1X1_RED);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_numbered_sequence_file_output_matches_ffmpeg_oracle() {
    compare_image2_sequence_file_output("ppm", "1", &[PPM_1X1_RED, PPM_1X1_GREEN]);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_numbered_sequence_framecrc_records_match_ffmpeg_oracle() {
    compare_image2_sequence_framecrc_records("ppm", "1", &[PPM_1X1_RED, PPM_1X1_GREEN]);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_nonzero_width_pattern_framecrc_records_match_ffmpeg_oracle() {
    compare_image2_sequence_framecrc_records_with_pattern(
        "ppm",
        "1",
        0,
        &[PPM_1X1_RED, PPM_1X1_GREEN],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_padded_width_growth_framecrc_records_match_ffmpeg_oracle() {
    compare_image2_sequence_framecrc_records_from_start(
        "ppm",
        "25",
        999,
        &[PPM_1X1_RED, PPM_1X1_GREEN],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_wide_zero_padded_sequence_framecrc_records_match_ffmpeg_oracle() {
    compare_image2_sequence_framecrc_records_from_start_with_width(
        "ppm",
        "1",
        1,
        20,
        &[PPM_1X1_RED, PPM_1X1_GREEN],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_sequence_with_gap_stops_at_first_missing_frame() {
    compare_image2_sequence_framecrc_records_from_sparse_indices(
        "ppm",
        "25",
        0,
        3,
        &[(0u64, PPM_1X1_RED), (2u64, PPM_1X1_GREEN)],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_sequence_with_start_number_skips_until_first_available() {
    compare_image2_sequence_framecrc_records_from_sparse_indices(
        "ppm",
        "25",
        1,
        3,
        &[(0u64, PPM_1X1_RED), (2u64, PPM_1X1_GREEN)],
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_sequence_with_start_number_accepts_probe_window_upper_boundary() {
    compare_image2_sequence_framecrc_records_from_sparse_indices(
        "ppm",
        "25",
        1,
        3,
        &[(5u64, PPM_1X1_RED)],
    );
}

fn compare_image2_file_output(extension: &str, frame_rate: &str, payload: &[u8]) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("image2-single-input", extension, payload);
    let rust_output_path = unique_temp_path("image2-rust-output", extension);
    let oracle_output_path = unique_temp_path("image2-oracle-output", extension);

    let input_arg = input_path.to_string_lossy().into_owned();
    let rust_output_arg = rust_output_path.to_string_lossy().into_owned();
    let oracle_output_arg = oracle_output_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "image2",
        "-framerate",
        frame_rate,
        "-i",
        input_arg.as_str(),
        "-f",
        "image2",
        rust_output_arg.as_str(),
    ]))
    .expect("Rust image2 file-output path should execute");

    let oracle_status = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "image2",
            "-framerate",
            frame_rate,
            "-i",
            input_arg.as_str(),
            "-c:v",
            "copy",
            "-f",
            "image2",
            oracle_output_arg.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        oracle_status.status.success(),
        "oracle image2 file output failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_status.status.code(),
        String::from_utf8_lossy(&oracle_status.stdout),
        String::from_utf8_lossy(&oracle_status.stderr)
    );

    let rust_bytes = fs::read(&rust_output_path).expect("Rust image2 output should be readable");
    let oracle_bytes =
        fs::read(&oracle_output_path).expect("oracle image2 output should be readable");

    remove_temp_files(&[input_path, rust_output_path, oracle_output_path]);

    assert_eq!(rust.output_format(), Some("image2"));
    assert_eq!(rust.packet_count(), 1);
    assert_eq!(rust.byte_count(), u64::try_from(payload.len()).unwrap());
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());
    assert_eq!(rust_bytes, payload);
    assert_eq!(rust_bytes, oracle_bytes);
}

fn compare_image2_single_framecrc_records(extension: &str, frame_rate: &str, payload: &[u8]) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("image2-single-framecrc-input", extension, payload);
    let input_arg = input_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "image2",
        "-framerate",
        frame_rate,
        "-i",
        input_arg.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .expect("Rust image2 single framecrc path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "image2",
            "-framerate",
            frame_rate,
            "-i",
            input_arg.as_str(),
            "-c:v",
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
        "oracle image2 single framecrc failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle framecrc output should be UTF-8");

    assert_eq!(rust.output_format(), Some("framecrc"));
    assert_eq!(rust.packet_count(), 1);
    assert_eq!(rust.byte_count(), u64::try_from(payload.len()).unwrap());
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_framecrc_records(rust.stdout()),
        normalize_framecrc_records(&oracle_stdout)
    );
}

fn compare_image2_sequence_file_output(extension: &str, frame_rate: &str, payloads: &[&[u8]]) {
    let oracle = oracle_ffmpeg();
    let input_dir = unique_temp_dir("image2-sequence-input");
    let rust_output_dir = unique_temp_dir("image2-sequence-rust-output");
    let oracle_output_dir = unique_temp_dir("image2-sequence-oracle-output");

    fs::create_dir(&input_dir).expect("temp image2 input dir should be creatable");
    fs::create_dir(&rust_output_dir).expect("temp Rust image2 output dir should be creatable");
    fs::create_dir(&oracle_output_dir).expect("temp oracle image2 output dir should be creatable");

    for (index, payload) in payloads.iter().enumerate() {
        let frame_number = u64::try_from(index).expect("test index should fit u64");
        fs::write(
            input_dir.join(sequence_file_name("in", frame_number, extension)),
            payload,
        )
        .expect("temp image2 sequence input should be writable");
    }

    let input_pattern = input_dir
        .join(format!("in-%03d.{extension}"))
        .to_string_lossy()
        .into_owned();
    let rust_output_pattern = rust_output_dir
        .join(format!("out-%03d.{extension}"))
        .to_string_lossy()
        .into_owned();
    let oracle_output_pattern = oracle_output_dir
        .join(format!("out-%03d.{extension}"))
        .to_string_lossy()
        .into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "image2",
        "-framerate",
        frame_rate,
        "-start_number",
        "0",
        "-i",
        input_pattern.as_str(),
        "-f",
        "image2",
        "-start_number",
        "0",
        rust_output_pattern.as_str(),
    ]))
    .expect("Rust image2 sequence file-output path should execute");

    let oracle_status = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "image2",
            "-framerate",
            frame_rate,
            "-start_number",
            "0",
            "-i",
            input_pattern.as_str(),
            "-c:v",
            "copy",
            "-f",
            "image2",
            "-start_number",
            "0",
            oracle_output_pattern.as_str(),
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        oracle_status.status.success(),
        "oracle image2 sequence file output failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_status.status.code(),
        String::from_utf8_lossy(&oracle_status.stdout),
        String::from_utf8_lossy(&oracle_status.stderr)
    );

    let rust_outputs = read_sequence_outputs(&rust_output_dir, extension, payloads.len());
    let oracle_outputs = read_sequence_outputs(&oracle_output_dir, extension, payloads.len());

    remove_temp_dirs(&[input_dir, rust_output_dir, oracle_output_dir]);

    let expected_byte_count: usize = payloads.iter().map(|payload| payload.len()).sum();
    assert_eq!(rust.output_format(), Some("image2"));
    assert_eq!(rust.packet_count(), u64::try_from(payloads.len()).unwrap());
    assert_eq!(
        rust.byte_count(),
        u64::try_from(expected_byte_count).unwrap()
    );
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());
    let expected_outputs: Vec<Vec<u8>> = payloads.iter().map(|payload| payload.to_vec()).collect();
    assert_eq!(rust_outputs, expected_outputs);
    assert_eq!(rust_outputs, oracle_outputs);
}

fn compare_image2_sequence_framecrc_records(extension: &str, frame_rate: &str, payloads: &[&[u8]]) {
    compare_image2_sequence_framecrc_records_from_start(extension, frame_rate, 0, payloads);
}

fn compare_image2_sequence_framecrc_records_from_start(
    extension: &str,
    frame_rate: &str,
    start_number: u64,
    payloads: &[&[u8]],
) {
    compare_image2_sequence_framecrc_records_from_start_with_width(
        extension,
        frame_rate,
        start_number,
        3,
        payloads,
    );
}

fn compare_image2_sequence_framecrc_records_from_start_with_width(
    extension: &str,
    frame_rate: &str,
    start_number: u64,
    frame_width: usize,
    payloads: &[&[u8]],
) {
    let oracle = oracle_ffmpeg();
    let input_dir = unique_temp_dir("image2-sequence-framecrc-input");

    fs::create_dir(&input_dir).expect("temp image2 framecrc input dir should be creatable");

    for (index, payload) in payloads.iter().enumerate() {
        let frame_number = start_number + u64::try_from(index).expect("test index should fit u64");
        fs::write(
            input_dir.join(sequence_file_name_with_width(
                "in",
                frame_number,
                frame_width,
                extension,
            )),
            payload,
        )
        .expect("temp image2 sequence framecrc input should be writable");
    }

    let input_pattern = input_dir
        .join(format!("in-%0{frame_width}d.{extension}"))
        .to_string_lossy()
        .into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "image2",
        "-framerate",
        frame_rate,
        "-start_number",
        &start_number.to_string(),
        "-i",
        input_pattern.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .expect("Rust image2 sequence framecrc path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "image2",
            "-framerate",
            frame_rate,
            "-start_number",
            &start_number.to_string(),
            "-i",
            input_pattern.as_str(),
            "-c:v",
            "copy",
            "-f",
            "framecrc",
            "-",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    remove_temp_dirs(&[input_dir]);

    assert!(
        oracle_output.status.success(),
        "oracle image2 sequence framecrc failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle framecrc output should be UTF-8");
    let expected_byte_count: usize = payloads.iter().map(|payload| payload.len()).sum();

    assert_eq!(rust.output_format(), Some("framecrc"));
    assert_eq!(rust.packet_count(), u64::try_from(payloads.len()).unwrap());
    assert_eq!(
        rust.byte_count(),
        u64::try_from(expected_byte_count).unwrap()
    );
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_framecrc_records(rust.stdout()),
        normalize_framecrc_records(&oracle_stdout)
    );
}

fn compare_image2_sequence_framecrc_records_with_pattern(
    extension: &str,
    frame_rate: &str,
    start_number: u64,
    payloads: &[&[u8]],
) {
    let oracle = oracle_ffmpeg();
    let input_dir = unique_temp_dir("image2-sequence-framecrc-input");

    fs::create_dir(&input_dir).expect("temp image2 framecrc input should be creatable");

    for (index, payload) in payloads.iter().enumerate() {
        let frame_number = start_number + u64::try_from(index).expect("test index should fit u64");
        fs::write(
            input_dir.join(format!("in-{frame_number:03}.{extension}")),
            payload,
        )
        .expect("temp image2 framecrc input should be writable");
    }

    let input_pattern = input_dir
        .join(format!("in-%3d.{extension}"))
        .to_string_lossy()
        .into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "image2",
        "-framerate",
        frame_rate,
        "-start_number",
        &start_number.to_string(),
        "-i",
        input_pattern.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .expect("Rust image2 sequence framecrc path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "image2",
            "-framerate",
            frame_rate,
            "-start_number",
            &start_number.to_string(),
            "-i",
            input_pattern.as_str(),
            "-c:v",
            "copy",
            "-f",
            "framecrc",
            "-",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    remove_temp_dirs(&[input_dir]);

    assert!(
        oracle_output.status.success(),
        "oracle image2 framecrc with nonzero width pattern failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle framecrc output should be UTF-8");
    let expected_byte_count: usize = payloads.iter().map(|payload| payload.len()).sum();

    assert_eq!(rust.output_format(), Some("framecrc"));
    assert_eq!(rust.packet_count(), u64::try_from(payloads.len()).unwrap());
    assert_eq!(
        rust.byte_count(),
        u64::try_from(expected_byte_count).unwrap()
    );
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_framecrc_records(rust.stdout()),
        normalize_framecrc_records(&oracle_stdout)
    );
}

fn compare_image2_sequence_framecrc_records_from_sparse_indices(
    extension: &str,
    frame_rate: &str,
    start_number: u64,
    frame_width: usize,
    payloads: &[(u64, &[u8])],
) {
    let oracle = oracle_ffmpeg();
    let input_dir = unique_temp_dir("image2-sequence-framecrc-input-sparse");

    fs::create_dir(&input_dir).expect("temp image2 framecrc sparse input dir should be creatable");

    for (frame_number, payload) in payloads.iter().copied() {
        fs::write(
            input_dir.join(sequence_file_name_with_width(
                "in",
                frame_number,
                frame_width,
                extension,
            )),
            payload,
        )
        .expect("temp image2 sparse framecrc input should be writable");
    }

    let input_pattern = input_dir
        .join(format!("in-%0{frame_width}d.{extension}"))
        .to_string_lossy()
        .into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "image2",
        "-framerate",
        frame_rate,
        "-start_number",
        &start_number.to_string(),
        "-i",
        input_pattern.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .expect("Rust image2 sparse framecrc path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "image2",
            "-framerate",
            frame_rate,
            "-start_number",
            &start_number.to_string(),
            "-i",
            input_pattern.as_str(),
            "-c:v",
            "copy",
            "-f",
            "framecrc",
            "-",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    remove_temp_dirs(&[input_dir]);

    assert!(
        oracle_output.status.success(),
        "oracle image2 sparse framecrc failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    assert_eq!(rust.output_format(), Some("framecrc"));
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_framecrc_records(rust.stdout()),
        normalize_framecrc_records(
            &String::from_utf8(oracle_output.stdout)
                .expect("oracle sparse framecrc output should be UTF-8")
        )
    );
}

fn oracle_ffmpeg() -> PathBuf {
    if let Some(path) = env::var_os("FFMPEG_ORACLE")
        .map(PathBuf::from)
        .map(resolve_repo_relative_path)
    {
        assert!(
            path.is_file(),
            "FFMPEG_ORACLE must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
            path.display()
        );
        return path;
    }

    let root = repo_root();
    for candidate in [
        root.join("third_party/ffmpeg-oracle/build/bin/ffmpeg.exe"),
        root.join("third_party/ffmpeg-oracle/build/bin/ffmpeg.cmd"),
        root.join("third_party/ffmpeg-oracle/build/bin/ffmpeg"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!("missing pinned FFmpeg oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg");
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

fn write_temp_bytes(prefix: &str, extension: &str, bytes: &[u8]) -> PathBuf {
    let path = unique_temp_path(prefix, extension);
    fs::write(&path, bytes).expect("temp input should be writable");
    path
}

fn unique_temp_path(prefix: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "{prefix}-{}-{nanos}.{extension}",
        std::process::id()
    ))
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn sequence_file_name(prefix: &str, frame_number: u64, extension: &str) -> String {
    format!("{prefix}-{frame_number:03}.{extension}")
}

fn sequence_file_name_with_width(
    prefix: &str,
    frame_number: u64,
    width: usize,
    extension: &str,
) -> String {
    format!("{prefix}-{frame_number:0width$}.{extension}")
}

fn read_sequence_outputs(dir: &std::path::Path, extension: &str, count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|index| {
            let frame_number = u64::try_from(index).expect("test index should fit u64");
            fs::read(dir.join(sequence_file_name("out", frame_number, extension)))
                .expect("image2 output sequence file should be readable")
        })
        .collect()
}

fn remove_temp_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn remove_temp_dirs(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_dir_all(path);
    }
}

fn normalize_framecrc_records(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .map(|line| line.split(',').map(str::trim).collect::<Vec<_>>().join("|"))
        .collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
