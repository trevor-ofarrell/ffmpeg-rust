use avutil::PixelFormat;
use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PixelFormatRow {
    flags: String,
    name: String,
    component_count: usize,
    bits_per_pixel: usize,
    bit_depths: Vec<u8>,
}

impl PixelFormatRow {
    fn is_hardware(&self) -> bool {
        matches!(self.flags.as_bytes().get(2), Some(b'H'))
    }

    fn is_bitstream(&self) -> bool {
        matches!(self.flags.as_bytes().get(4), Some(b'B'))
    }

    fn is_paletted(&self) -> bool {
        matches!(self.flags.as_bytes().get(3), Some(b'P'))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedPixelFormatRow {
    name: String,
    component_count: usize,
    bits_per_pixel: Option<usize>,
    bit_depths: Vec<u8>,
    is_hardware: bool,
    is_bitstream: bool,
    is_paletted: bool,
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_pixel_format_inventory_contains_current_pixel_format_subset() {
    let oracle = oracle_ffmpeg();
    let output = Command::new(&oracle)
        .args(["-hide_banner", "-pix_fmts"])
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

    let oracle_rows = parse_pixel_format_inventory(&text);
    for expected in expected_pixel_format_subset() {
        let actual = oracle_rows
            .get(expected.name.as_str())
            .unwrap_or_else(|| panic!("missing ffmpeg -pix_fmts row for `{}`", expected.name));

        assert_eq!(
            actual.component_count, expected.component_count,
            "ffmpeg -pix_fmts component count diverged for `{}`",
            expected.name
        );
        if let Some(expected_bits_per_pixel) = expected.bits_per_pixel {
            assert_eq!(
                actual.bits_per_pixel, expected_bits_per_pixel,
                "ffmpeg -pix_fmts integer bits-per-pixel diverged for `{}`",
                expected.name
            );
        }
        assert_eq!(
            actual.bit_depths, expected.bit_depths,
            "ffmpeg -pix_fmts component bit depths diverged for `{}`",
            expected.name
        );
        assert_eq!(
            actual.is_hardware(),
            expected.is_hardware,
            "ffmpeg -pix_fmts hardware flag diverged for `{}`",
            expected.name
        );
        assert_eq!(
            actual.is_bitstream(),
            expected.is_bitstream,
            "ffmpeg -pix_fmts bitstream flag diverged for `{}`",
            expected.name
        );
        assert_eq!(
            actual.is_paletted(),
            expected.is_paletted,
            "ffmpeg -pix_fmts paletted flag diverged for `{}`",
            expected.name
        );
    }
}

#[test]
fn parses_ffmpeg_pixel_format_inventory_table() {
    let rows = parse_pixel_format_inventory(
        r#"
Pixel formats:
I.... = Supported Input  format for conversion
.O... = Supported Output format for conversion
..H.. = Hardware accelerated format
...P. = Paletted format
....B = Bitstream format
FLAGS NAME            NB_COMPONENTS BITS_PER_PIXEL BIT_DEPTHS
-----
IO... yuv420p                3            12      8-8-8
IO..B monow                  1             1      1
IO.P. pal8                   1             8      8
....B xv30be                  3             30      10-10-10
..H.. vaapi                  0             0      0
"#,
    );

    assert_eq!(
        rows.get("yuv420p"),
        Some(&PixelFormatRow {
            flags: "IO...".to_string(),
            name: "yuv420p".to_string(),
            component_count: 3,
            bits_per_pixel: 12,
            bit_depths: vec![8, 8, 8],
        })
    );
    assert!(rows["vaapi"].is_hardware());
    assert!(!rows["monow"].is_paletted());
    assert!(rows["pal8"].is_paletted());
    assert!(rows["monow"].is_bitstream());
    assert!(rows["xv30be"].is_bitstream());
    assert!(!rows["monow"].is_hardware());
    assert!(!rows["xv30be"].is_hardware());
    assert_eq!(rows["monow"].bit_depths, vec![1]);
    assert_eq!(rows["pal8"].bit_depths, vec![8]);
}

fn expected_pixel_format_subset() -> Vec<ExpectedPixelFormatRow> {
    PixelFormat::ALL
        .iter()
        .chain(PixelFormat::HARDWARE.iter())
        .map(|format| {
            let descriptor = format.descriptor();
            ExpectedPixelFormatRow {
                name: descriptor.name.to_string(),
                component_count: descriptor.component_count,
                bits_per_pixel: descriptor.bits_per_pixel_integer().map(usize::from),
                bit_depths: format.component_bit_depths(),
                is_hardware: format.is_hardware(),
                is_bitstream: format.is_bitstream(),
                is_paletted: descriptor.is_paletted,
            }
        })
        .collect()
}

fn parse_pixel_format_inventory(text: &str) -> BTreeMap<String, PixelFormatRow> {
    let mut rows = BTreeMap::new();
    let mut found_header = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("FLAGS NAME") {
            found_header = true;
            continue;
        }
        if !found_header || trimmed.starts_with('-') {
            continue;
        }

        let columns = trimmed.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 5 || columns[0].len() != 5 {
            continue;
        }

        let component_count = columns[2].parse().unwrap_or_else(|err| {
            panic!("invalid ffmpeg -pix_fmts component count in `{trimmed}`: {err}")
        });
        let bits_per_pixel = columns[3].parse().unwrap_or_else(|err| {
            panic!("invalid ffmpeg -pix_fmts bits-per-pixel in `{trimmed}`: {err}")
        });
        let bit_depths = columns[4]
            .split('-')
            .map(|depth| {
                depth.parse::<u8>().unwrap_or_else(|err| {
                    panic!("invalid ffmpeg -pix_fmts bit-depth entry in `{trimmed}`: {err}")
                })
            })
            .collect::<Vec<_>>();

        let row = PixelFormatRow {
            flags: columns[0].to_string(),
            name: columns[1].to_string(),
            component_count,
            bits_per_pixel,
            bit_depths,
        };
        let previous = rows.insert(row.name.clone(), row);
        assert!(previous.is_none(), "duplicate ffmpeg -pix_fmts row");
    }

    assert!(found_header, "missing ffmpeg -pix_fmts table header");
    assert!(!rows.is_empty(), "missing ffmpeg -pix_fmts rows");
    rows
}

fn oracle_ffmpeg() -> PathBuf {
    let root = repo_root();

    if let Ok(path) = env::var("FFMPEG_ORACLE") {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        assert!(
            path.is_file(),
            "FFMPEG_ORACLE must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
            path.display()
        );
        return path;
    }

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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("avutil crate should be under crates/")
        .to_path_buf()
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
