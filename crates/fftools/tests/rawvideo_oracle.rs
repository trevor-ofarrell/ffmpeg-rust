use avformat::{AviDemuxer, AviMediaType, AviMuxer};
use avutil::{Packet, Rational};
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
fn rawvideo_bgr24_avi_file_output_matches_ffmpeg_oracle() {
    compare_rawvideo_avi_file_output("bgr24", 2, 1, "25", &(0_u8..12).collect::<Vec<_>>());
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn avi_rgb24_framecrc_records_match_ffmpeg_oracle() {
    let first = [0_u8, 1, 2, 3, 4, 5];
    let second = [6_u8, 7, 8, 9, 10, 11];
    let avi = avi_file_bytes(
        2,
        1,
        Rational::new(25, 1).unwrap(),
        &[first.as_slice(), second.as_slice()],
    );
    compare_avi_framecrc_records(&avi, 2, 16);
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

fn compare_rawvideo_avi_file_output(
    pixel_format: &str,
    width: u32,
    height: u32,
    rate: &str,
    payload: &[u8],
) {
    let oracle = oracle_ffmpeg();
    let size = format!("{width}x{height}");
    let input_path = write_temp_bytes(&format!("{pixel_format}-avi-input"), "raw", payload);
    let rust_output_path = unique_temp_path(&format!("{pixel_format}-rust-output"), "avi");
    let oracle_output_path = unique_temp_path(&format!("{pixel_format}-oracle-output"), "avi");

    let input_arg = input_path.to_string_lossy().into_owned();
    let rust_output_arg = rust_output_path.to_string_lossy().into_owned();
    let oracle_output_arg = oracle_output_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-f",
        "rawvideo",
        "-pix_fmt",
        pixel_format,
        "-s",
        size.as_str(),
        "-r",
        rate,
        "-i",
        input_arg.as_str(),
        "-f",
        "avi",
        rust_output_arg.as_str(),
    ]))
    .expect("Rust rawvideo to AVI file-output path should execute");

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
            size.as_str(),
            "-r",
            rate,
            "-i",
            input_arg.as_str(),
            "-c:v",
            "rawvideo",
            "-f",
            "avi",
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

    let rust_bytes = fs::read(&rust_output_path).expect("Rust AVI output should be readable");
    let oracle_bytes = fs::read(&oracle_output_path).expect("oracle AVI output should be readable");

    remove_temp_files(&[input_path, rust_output_path, oracle_output_path]);

    assert_eq!(rust.output_format(), Some("avi"));
    assert_eq!(rust.packet_count(), 2);
    assert_eq!(rust.byte_count(), u64::try_from(rust_bytes.len()).unwrap());
    assert!(rust.stdout().is_empty());
    assert!(rust.stderr().is_empty());

    let mut rust_demuxer = AviDemuxer::open(&rust_bytes).expect("Rust AVI output should demux");
    let mut oracle_demuxer =
        AviDemuxer::open(&oracle_bytes).expect("oracle AVI output should demux");
    compare_avi_demuxed_semantics(
        &mut rust_demuxer,
        &mut oracle_demuxer,
        width,
        height,
        Rational::new(25, 1).unwrap(),
        payload,
    );
}

fn compare_avi_demuxed_semantics(
    rust: &mut AviDemuxer,
    oracle: &mut AviDemuxer,
    expected_width: u32,
    expected_height: u32,
    expected_frame_rate: Rational,
    expected_payload: &[u8],
) {
    assert_eq!(rust.info().width(), expected_width);
    assert_eq!(oracle.info().width(), expected_width);
    assert_eq!(rust.info().height(), expected_height);
    assert_eq!(oracle.info().height(), expected_height);
    assert_eq!(rust.info().total_frames(), oracle.info().total_frames());
    assert_eq!(rust.info().packet_count(), oracle.info().packet_count());
    assert_eq!(rust.info().streams().len(), 1);
    assert_eq!(oracle.info().streams().len(), 1);

    let rust_stream = &rust.info().streams()[0];
    let oracle_stream = &oracle.info().streams()[0];
    assert_eq!(rust_stream.index(), oracle_stream.index());
    assert_eq!(rust_stream.media_type(), AviMediaType::Video);
    assert_eq!(oracle_stream.media_type(), AviMediaType::Video);
    assert_eq!(rust_stream.handler(), oracle_stream.handler());
    assert_eq!(rust_stream.time_base(), oracle_stream.time_base());
    assert_eq!(rust_stream.frame_rate(), expected_frame_rate);
    assert_eq!(oracle_stream.frame_rate(), expected_frame_rate);
    assert_eq!(rust_stream.length(), oracle_stream.length());
    assert_eq!(rust_stream.sample_size(), oracle_stream.sample_size());
    assert_eq!(rust_stream.width(), expected_width);
    assert_eq!(oracle_stream.width(), expected_width);
    assert_eq!(rust_stream.height(), expected_height);
    assert_eq!(oracle_stream.height(), expected_height);
    assert_eq!(rust_stream.bit_count(), 24);
    assert_eq!(oracle_stream.bit_count(), 24);
    assert_eq!(rust_stream.compression(), oracle_stream.compression());

    let mut demuxed = Vec::new();
    let mut packet_index = 0_i64;
    loop {
        let rust_packet = rust
            .read_packet()
            .expect("Rust AVI packet read should succeed");
        let oracle_packet = oracle
            .read_packet()
            .expect("oracle AVI packet read should succeed");
        match (rust_packet, oracle_packet) {
            (Some(rust_packet), Some(oracle_packet)) => {
                assert_eq!(rust_packet.stream_index(), oracle_packet.stream_index());
                assert_eq!(rust_packet.pts(), Some(packet_index));
                assert_eq!(oracle_packet.pts(), Some(packet_index));
                assert_eq!(rust_packet.dts(), Some(packet_index));
                assert_eq!(oracle_packet.dts(), Some(packet_index));
                assert_eq!(rust_packet.duration(), oracle_packet.duration());
                assert_eq!(rust_packet.side_data(), oracle_packet.side_data());
                assert_eq!(rust_packet.data(), oracle_packet.data());
                demuxed.extend_from_slice(rust_packet.data());
                packet_index += 1;
            }
            (None, None) => break,
            other => panic!("packet count mismatch between Rust and oracle AVI outputs: {other:?}"),
        }
    }

    assert_eq!(
        demuxed,
        dib_padded_rgb24_payload(expected_payload, expected_width)
    );
}

fn avi_file_bytes(width: u32, height: u32, frame_rate: Rational, frames: &[&[u8]]) -> Vec<u8> {
    let mut muxer =
        AviMuxer::new_rgb24(width, height, frame_rate).expect("AVI muxer should be constructible");
    for frame in frames {
        muxer
            .write_packet(&Packet::new((*frame).to_vec(), 0))
            .expect("test AVI packet should be valid");
    }
    muxer.finish().expect("test AVI should finish")
}

fn dib_padded_rgb24_payload(payload: &[u8], width: u32) -> Vec<u8> {
    let row_bytes = usize::try_from(width).unwrap() * 3;
    let stride = (row_bytes + 3) & !3;
    let mut padded = Vec::new();
    for row in payload.chunks_exact(row_bytes) {
        padded.extend_from_slice(row);
        padded.resize(padded.len() + stride - row_bytes, 0);
    }
    padded
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

fn compare_avi_framecrc_records(avi: &[u8], packet_count: u64, expected_byte_count: usize) {
    let oracle = oracle_ffmpeg();
    let input_path = write_temp_bytes("avi-framecrc-input", "avi", avi);
    let input_arg = input_path.to_string_lossy().into_owned();

    let rust = ffmpeg_output(&strings(&[
        "-hide_banner",
        "-i",
        input_arg.as_str(),
        "-f",
        "framecrc",
        "-",
    ]))
    .unwrap_or_else(|err| panic!("Rust AVI framecrc path should execute: {err}"));

    let oracle_output = Command::new(&oracle)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
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
        "oracle AVI framecrc failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        oracle_output.status.code(),
        String::from_utf8_lossy(&oracle_output.stdout),
        String::from_utf8_lossy(&oracle_output.stderr)
    );

    let oracle_stdout =
        String::from_utf8(oracle_output.stdout).expect("oracle checksum output should be UTF-8");

    assert_eq!(rust.output_format(), Some("framecrc"));
    assert_eq!(rust.packet_count(), packet_count);
    assert_eq!(
        rust.byte_count(),
        u64::try_from(expected_byte_count).unwrap()
    );
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
