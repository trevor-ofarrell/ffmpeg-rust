use std::process::Command;

#[test]
fn ffmpeg_rs_prints_version_banner() {
    let output = Command::new(env!("CARGO_BIN_EXE_ffmpeg-rs"))
        .arg("-version")
        .output()
        .expect("ffmpeg-rs should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.starts_with("ffmpeg version 8.1.1-rust target FFmpeg 8.1.1"));
    assert!(stdout.contains("libavcodec"));
}

#[test]
fn ffprobe_rs_prints_version_banner() {
    let output = Command::new(env!("CARGO_BIN_EXE_ffprobe-rs"))
        .arg("-version")
        .output()
        .expect("ffprobe-rs should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.starts_with("ffprobe version 8.1.1-rust target FFmpeg 8.1.1"));
    assert!(stdout.contains("libavformat"));
}
