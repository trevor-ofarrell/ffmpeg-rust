use avutil::{colors_table_string, NamedColor};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColorRow {
    name: String,
    rgb: String,
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_colors_inventory_matches_named_color_model() {
    let oracle = oracle_ffmpeg();
    let output = Command::new(&oracle)
        .args(["-hide_banner", "-colors"])
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
        parse_colors_inventory(&text),
        expected_colors(),
        "ffmpeg -colors inventory diverged"
    );
}

#[test]
fn parses_ffmpeg_colors_inventory_table() {
    let rows = parse_colors_inventory(
        r#"
name                             #RRGGBB
AliceBlue                        #f0f8ff
Darkorange                       #ff8c00
YellowGreen                      #9acd32
"#,
    );

    assert_eq!(
        rows,
        vec![
            ColorRow {
                name: "AliceBlue".to_string(),
                rgb: "#f0f8ff".to_string(),
            },
            ColorRow {
                name: "Darkorange".to_string(),
                rgb: "#ff8c00".to_string(),
            },
            ColorRow {
                name: "YellowGreen".to_string(),
                rgb: "#9acd32".to_string(),
            },
        ]
    );

    assert_eq!(
        parse_colors_inventory(&colors_table_string()),
        expected_colors()
    );
}

fn expected_colors() -> Vec<ColorRow> {
    NamedColor::ALL
        .iter()
        .map(|color| ColorRow {
            name: color.name().to_string(),
            rgb: color.rgb_hex_lower(),
        })
        .collect()
}

fn parse_colors_inventory(text: &str) -> Vec<ColorRow> {
    let mut rows = Vec::new();
    let mut found_header = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let columns = trimmed.split_whitespace().collect::<Vec<_>>();
        if columns == ["name", "#RRGGBB"] {
            found_header = true;
            continue;
        }
        if !found_header {
            continue;
        }
        if columns.len() != 2 || !is_lower_rgb_hex(columns[1]) {
            continue;
        }

        rows.push(ColorRow {
            name: columns[0].to_string(),
            rgb: columns[1].to_string(),
        });
    }

    assert!(found_header, "missing ffmpeg -colors table header");
    assert!(!rows.is_empty(), "missing ffmpeg -colors rows");
    rows
}

fn is_lower_rgb_hex(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value
            .as_bytes()
            .iter()
            .skip(1)
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
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
