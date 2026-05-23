use fftools::ffmpeg_output;
use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const PPM_1X1_RED: &[u8] = b"P6\n1 1\n255\n\xff\0\0";

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn image2_ppm_single_file_output_matches_ffmpeg_oracle() {
    compare_image2_file_output("ppm", "1", PPM_1X1_RED);
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

fn remove_temp_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
