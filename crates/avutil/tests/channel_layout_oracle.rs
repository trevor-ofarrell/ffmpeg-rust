use avutil::{Channel, ChannelLayout};
use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Default)]
struct LayoutInventory {
    channels: BTreeMap<String, String>,
    layouts: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutSection {
    None,
    Channels,
    Layouts,
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_layout_inventory_matches_current_channel_layout_model() {
    let oracle = oracle_ffmpeg();
    let output = Command::new(&oracle)
        .args(["-hide_banner", "-layouts"])
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

    let inventory = parse_layout_inventory(&text);

    assert_eq!(
        inventory.channels,
        expected_channels(),
        "ffmpeg -layouts individual channel inventory diverged"
    );
    assert_eq!(
        inventory.layouts,
        expected_layouts(),
        "ffmpeg -layouts standard layout inventory diverged"
    );
}

fn expected_channels() -> BTreeMap<String, String> {
    Channel::ALL
        .iter()
        .map(|channel| {
            (
                channel.name().to_string(),
                channel.description().to_string(),
            )
        })
        .collect()
}

fn expected_layouts() -> BTreeMap<String, String> {
    ChannelLayout::known_layouts()
        .into_iter()
        .map(|layout| (layout.name().to_string(), layout.channel_string()))
        .collect()
}

fn parse_layout_inventory(text: &str) -> LayoutInventory {
    let mut inventory = LayoutInventory::default();
    let mut section = LayoutSection::None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("Individual channels:") {
            section = LayoutSection::Channels;
            continue;
        }
        if trimmed.starts_with("Standard channel layouts:") {
            section = LayoutSection::Layouts;
            continue;
        }
        if section == LayoutSection::None || trimmed.starts_with("NAME") {
            continue;
        }

        let Some((name, value)) = parse_inventory_entry(trimmed) else {
            continue;
        };

        let previous = match section {
            LayoutSection::Channels => inventory.channels.insert(name, value),
            LayoutSection::Layouts => inventory.layouts.insert(name, value),
            LayoutSection::None => unreachable!(),
        };
        assert!(previous.is_none(), "duplicate ffmpeg -layouts entry");
    }

    inventory
}

fn parse_inventory_entry(line: &str) -> Option<(String, String)> {
    let split_at = line.find(char::is_whitespace)?;
    let name = line[..split_at].to_string();
    let value = line[split_at..].trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some((name, value))
    }
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
