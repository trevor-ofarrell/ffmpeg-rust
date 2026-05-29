use avutil::PixelFormat;
use std::{
    collections::BTreeMap,
    env, fs,
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
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_pixel_format_name_lookup_matches_bounded_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/pixdesc.h").is_file(),
        "missing pinned FFmpeg libavutil pixel format headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-pixel-format");
    fs::create_dir_all(&work_dir).expect("create avutil-pixel-format oracle work dir");
    let source = work_dir.join("pixel_format_oracle.c");
    let executable = work_dir.join("pixel_format_oracle");
    fs::write(&source, oracle_c_source()).expect("write pixel format oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let rows = parse_oracle_output(&stdout);

    for input in [
        "gray",
        "gray8",
        "gray8a",
        "y400a",
        "x2rgb10",
        "x2bgr10",
        "rgb24",
        "RGB24",
        "y32le",
        "yf32le",
        "vaapi",
        "not_a_pix_fmt",
    ] {
        let expected = PixelFormat::from_av_get_pix_fmt_name(input);
        assert_eq!(
            row_fields(&rows, &format!("lookup:{input}")),
            &[
                u8::from(expected.is_some()).to_string(),
                expected.map(PixelFormat::name).unwrap_or("").to_string(),
            ],
            "av_get_pix_fmt lookup diverged for `{input}`"
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

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let mut parts = line.split('|');
        let name = parts.next().expect("row name").to_string();
        let fields = parts.map(str::to_string).collect::<Vec<_>>();
        assert!(!fields.is_empty(), "oracle row `{line}` has no fields");
        assert!(
            rows.insert(name, fields).is_none(),
            "duplicate oracle row `{line}`"
        );
    }
    rows
}

fn row_fields<'a>(rows: &'a BTreeMap<String, Vec<String>>, name: &str) -> &'a [String] {
    rows.get(name)
        .unwrap_or_else(|| panic!("missing oracle row `{name}`"))
}

fn compile_and_run_oracle(
    include_dir: &Path,
    libavutil: &Path,
    source: &Path,
    executable: &Path,
) -> String {
    let output = if cfg!(windows) {
        let script = format!(
            "gcc -I {} {} {} -lm -pthread -ldl -o {} && {}",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(libavutil)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable))
        );
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run WSL libavutil pixel format oracle")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gcc -I {} {} {} -lm -pthread -ldl -o {} && {}",
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&libavutil.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string())
            ))
            .output()
            .expect("run libavutil pixel format oracle")
    };

    assert!(
        output.status.success(),
        "libavutil pixel format oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <stdio.h>
#include <libavutil/pixdesc.h>
#include <libavutil/pixfmt.h>

static void print_lookup(const char *input) {
    enum AVPixelFormat fmt = av_get_pix_fmt(input);
    const char *name = av_get_pix_fmt_name(fmt);
    printf("lookup:%s|%d|%s\n", input, fmt != AV_PIX_FMT_NONE, name ? name : "");
}

int main(void) {
    print_lookup("gray");
    print_lookup("gray8");
    print_lookup("gray8a");
    print_lookup("y400a");
    print_lookup("x2rgb10");
    print_lookup("x2bgr10");
    print_lookup("rgb24");
    print_lookup("RGB24");
    print_lookup("y32le");
    print_lookup("yf32le");
    print_lookup("vaapi");
    print_lookup("not_a_pix_fmt");
    return 0;
}
"#
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

fn oracle_root(repo_root: &Path) -> PathBuf {
    let default_root = repo_root.join("third_party/ffmpeg-oracle");
    if let Ok(ffmpeg) = env::var("FFMPEG_ORACLE") {
        let ffmpeg = PathBuf::from(ffmpeg);
        let ffmpeg = if ffmpeg.is_absolute() {
            ffmpeg
        } else {
            repo_root.join(ffmpeg)
        };
        if let Some(root) = ffmpeg.ancestors().find(|ancestor| {
            ancestor
                .file_name()
                .is_some_and(|name| name == "ffmpeg-oracle")
        }) {
            return root.to_path_buf();
        }
    }
    default_root
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(windows)]
fn to_wsl_path(path: &Path) -> String {
    let absolute = absolute_path(path);
    let mut text = absolute.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = text.strip_prefix("//?/") {
        text = stripped.to_string();
    }
    let bytes = text.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        text.replace_range(0..3, &format!("/mnt/{drive}/"));
    }
    text
}

#[cfg(windows)]
fn absolute_path(path: &Path) -> PathBuf {
    if path.exists() {
        return path.canonicalize().unwrap_or_else(|err| {
            panic!(
                "failed to canonicalize existing path `{}`: {err}",
                path.display()
            )
        });
    }
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("path `{}` has no parent", path.display()))
        .canonicalize()
        .unwrap_or_else(|err| {
            panic!(
                "failed to canonicalize parent of `{}`: {err}",
                path.display()
            )
        });
    parent.join(
        path.file_name()
            .unwrap_or_else(|| panic!("path `{}` has no file name", path.display())),
    )
}

#[cfg(not(windows))]
fn to_wsl_path(path: &Path) -> String {
    path.display().to_string()
}
