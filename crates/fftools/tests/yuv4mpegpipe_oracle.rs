use avformat::Yuv4MpegDemuxer;
use avutil::Rational;
use fftools::ffmpeg_output;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn rawvideo_yuv420p_yuv4mpegpipe_file_output_matches_ffmpeg_oracle() {
    let first = [0, 1, 2, 3, 4, 5];
    let second = [6, 7, 8, 9, 10, 11];
    let payload = [first.as_slice(), second.as_slice()].concat();
    compare_rawvideo_yuv4mpegpipe_file_output("2x2", "25", &payload);
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn yuv4mpegpipe_yuv420p_framecrc_records_match_ffmpeg_oracle() {
    let first = [0, 1, 2, 3, 4, 5];
    let second = [6, 7, 8, 9, 10, 11];
    let payload = [first.as_slice(), second.as_slice()].concat();
    let y4m = y4m_file_bytes(2, 2, "25:1", &[first.as_slice(), second.as_slice()]);
    compare_yuv4mpegpipe_framecrc_records(&y4m, 2, payload.len());
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn yuv4mpegpipe_truncated_frame_eof_records_match_ffmpeg_oracle() {
    let first = [0, 1, 2, 3, 4, 5];

    let mut truncated_first = y4m_file_bytes(2, 2, "25:1", &[]);
    truncated_first.extend_from_slice(b"FRAME\nabc");
    compare_yuv4mpegpipe_framecrc_records(&truncated_first, 0, 0);

    let mut truncated_tail = y4m_file_bytes(2, 2, "25:1", &[first.as_slice()]);
    truncated_tail.extend_from_slice(b"FRAME\nabc");
    compare_yuv4mpegpipe_framecrc_records(&truncated_tail, 1, first.len());
}

fn compare_rawvideo_yuv4mpegpipe_file_output(size: &str, rate: &str, payload: &[u8]) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("yuv420p-y4m-input", "raw", payload);
    let rust_output_path = unique_temp_path("yuv420p-rust-output", "y4m");
    let oracle_output_path = unique_temp_path("yuv420p-oracle-output", "y4m");

    let input_arg = input_path.to_string_lossy().into_owned();
    let rust_output_arg = rust_output_path.to_string_lossy().into_owned();
    let oracle_output_arg = oracle_output_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "rawvideo",
        "-pix_fmt",
        "yuv420p",
        "-s",
        size,
        "-r",
        rate,
        "-i",
        input_arg.as_str(),
        "-f",
        "yuv4mpegpipe",
        rust_output_arg.as_str(),
    ]))
    .expect("Rust yuv4mpegpipe file-output path should execute");

    let oracle_status = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "-s",
            size,
            "-r",
            rate,
            "-i",
            input_arg.as_str(),
            "-c:v",
            "copy",
            "-f",
            "yuv4mpegpipe",
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

    assert_eq!(rust.output_format(), Some("yuv4mpegpipe"));
    assert_eq!(rust.packet_count(), 2);
    assert_eq!(rust.byte_count(), u64::try_from(rust_bytes.len()).unwrap());
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());
    assert_eq!(rust_bytes, oracle_bytes);

    let mut demuxer =
        Yuv4MpegDemuxer::open(&oracle_bytes).expect("oracle YUV4MPEG2 output should parse");
    assert_eq!(demuxer.info().width(), 2);
    assert_eq!(demuxer.info().height(), 2);
    assert_eq!(demuxer.info().frame_rate(), Rational::new(25, 1).unwrap());
    assert_eq!(demuxer.info().sample_aspect_ratio(), None);
    let first = demuxer.read_packet().unwrap().unwrap();
    let second = demuxer.read_packet().unwrap().unwrap();
    assert_eq!(first.data(), &payload[..6]);
    assert_eq!(second.data(), &payload[6..]);
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn compare_yuv4mpegpipe_framecrc_records(y4m: &[u8], packet_count: u64, payload_len: usize) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("yuv420p-y4m-framecrc-input", "y4m", y4m);
    let input_arg = input_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "yuv4mpegpipe",
        "-i",
        input_arg.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .expect("Rust yuv4mpegpipe framecrc path should execute");

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "yuv4mpegpipe",
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
        "oracle yuv4mpegpipe framecrc failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle framecrc output should be UTF-8");

    assert_eq!(rust.output_format(), Some("framecrc"));
    assert_eq!(rust.packet_count(), packet_count);
    assert_eq!(rust.byte_count(), u64::try_from(payload_len).unwrap());
    assert!(rust.stderr().is_empty());
    assert_eq!(
        normalize_framecrc_records(rust.stdout()),
        normalize_framecrc_records(&oracle_stdout)
    );
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

fn y4m_file_bytes(width: u32, height: u32, frame_rate: &str, frames: &[&[u8]]) -> Vec<u8> {
    let mut bytes =
        format!("YUV4MPEG2 W{width} H{height} F{frame_rate} Ip C420jpeg\n").into_bytes();
    for frame in frames {
        bytes.extend_from_slice(b"FRAME\n");
        bytes.extend_from_slice(frame);
    }
    bytes
}

fn normalize_framecrc_records(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
        .map(|line| line.split(',').map(str::trim).collect::<Vec<_>>().join("|"))
        .collect()
}
