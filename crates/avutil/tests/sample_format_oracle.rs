use avutil::SampleFormat;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleFormatRow {
    name: String,
    depth: usize,
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_sample_format_inventory_matches_current_sample_format_model() {
    let oracle = oracle_ffmpeg();
    let output = Command::new(&oracle)
        .args(["-hide_banner", "-sample_fmts"])
        .output()
        .unwrap_or_else(|err| panic!("failed to run oracle `{}`: {err}", oracle.display()));

    assert!(
        output.status.success(),
        "oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    assert_eq!(
        parse_sample_format_inventory(&text),
        expected_sample_formats(),
        "ffmpeg -sample_fmts inventory diverged"
    );
}

fn expected_sample_formats() -> Vec<SampleFormatRow> {
    SampleFormat::ALL
        .iter()
        .map(|format| SampleFormatRow {
            name: format.name().to_string(),
            depth: format.sample_bits(),
        })
        .collect()
}

fn parse_sample_format_inventory(text: &str) -> Vec<SampleFormatRow> {
    let mut rows = Vec::new();
    let mut found_header = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == SampleFormat::sample_fmt_string_header() {
            found_header = true;
            continue;
        }
        if !found_header {
            continue;
        }

        let columns = trimmed.split_whitespace().collect::<Vec<_>>();
        if columns.len() != 2 {
            continue;
        }
        let depth = columns[1].parse().unwrap_or_else(|err| {
            panic!("invalid ffmpeg -sample_fmts depth in `{trimmed}`: {err}")
        });
        rows.push(SampleFormatRow {
            name: columns[0].to_string(),
            depth,
        });
    }

    assert!(found_header, "missing ffmpeg -sample_fmts header");
    assert!(!rows.is_empty(), "missing ffmpeg -sample_fmts rows");
    rows
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

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("avutil crate should be under crates/");

    for candidate in default_ffmpeg_candidates(root) {
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
