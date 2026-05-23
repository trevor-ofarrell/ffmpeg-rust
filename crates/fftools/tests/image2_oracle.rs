use fftools::ffmpeg_output;
use std::{
    env, fs,
    path::PathBuf,
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
fn image2_ppm_numbered_sequence_file_output_matches_ffmpeg_oracle() {
    compare_image2_sequence_file_output("ppm", "1", &[PPM_1X1_RED, PPM_1X1_GREEN]);
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

fn oracle_ffmpeg() -> PathBuf {
    if let Some(path) = env::var_os("FFMPEG_ORACLE").map(PathBuf::from) {
        return path;
    }

    for candidate in [
        PathBuf::from("third_party/ffmpeg-oracle/build/bin/ffmpeg.exe"),
        PathBuf::from("third_party/ffmpeg-oracle/build/bin/ffmpeg.cmd"),
        PathBuf::from("third_party/ffmpeg-oracle/build/bin/ffmpeg"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!("missing pinned FFmpeg oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg");
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

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
