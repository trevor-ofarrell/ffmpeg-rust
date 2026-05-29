use avutil::{colors_table_string, parse_color, NamedColor, RgbaColor};
use std::{
    collections::BTreeMap,
    env, fs,
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
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_parse_color_vectors_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/parseutils.h").is_file(),
        "missing pinned FFmpeg libavutil headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-color");
    fs::create_dir_all(&work_dir).expect("create avutil-color oracle work dir");
    let source = work_dir.join("color_oracle.c");
    let executable = work_dir.join("color_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-color oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let rows = parse_oracle_output(&stdout);

    for (label, input) in [
        ("named-red", "red"),
        ("named-case", "LIGHTGOLDENRODYELLOW"),
        ("hex-hash", "#112233"),
        ("hex-0x-alpha", "0x11223344"),
        ("hex-bare", "112233"),
        ("hex-bare-alpha", "AABBCCDD"),
        ("alpha-hex", "red@0x80"),
        ("alpha-float-half", "Blue@0.5"),
        ("alpha-one", "white@1"),
        ("alpha-trunc", "red@0.999"),
        ("alpha-overrides-embedded", "#01020304@0x05"),
        ("invalid-empty", ""),
        ("invalid-short-hex", "#12345"),
        ("invalid-hex-byte", "#11223z"),
        ("invalid-uppercase-prefix", "0X112233"),
        ("invalid-unknown-name", "transparent"),
        ("invalid-empty-alpha", "white@"),
        ("invalid-hex-alpha-overflow", "red@0x100"),
        ("invalid-empty-hex-alpha", "red@0x"),
        ("invalid-double-alpha", "red@@0.5"),
    ] {
        let fields = row_fields(&rows, &format!("parse:{label}"));
        match parse_color(input) {
            Ok(color) => {
                assert_eq!(
                    fields,
                    &[String::from("0"), rgba_hex(color)],
                    "av_parse_color success row diverged for `{input}`"
                );
            }
            Err(err) => {
                assert_eq!(
                    fields.first(),
                    err.code().map(|code| code.raw().to_string()).as_ref(),
                    "av_parse_color error code diverged for `{input}`"
                );
            }
        }
    }
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

fn rgba_hex(color: RgbaColor) -> String {
    let [r, g, b, a] = color.rgba();
    format!("{r:02x}{g:02x}{b:02x}{a:02x}")
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
            .expect("run WSL libavutil color oracle")
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
            .expect("run libavutil color oracle")
    };

    assert!(
        output.status.success(),
        "libavutil color oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r##"#include <stdint.h>
#include <stdio.h>
#include <libavutil/parseutils.h>

static void print_parse(const char *label, const char *input) {
    uint8_t rgba[4] = { 0xaa, 0xbb, 0xcc, 0xdd };
    int ret = av_parse_color(rgba, input, -1, NULL);
    printf("parse:%s|%d|%02x%02x%02x%02x\n",
           label, ret, rgba[0], rgba[1], rgba[2], rgba[3]);
}

int main(void) {
    print_parse("named-red", "red");
    print_parse("named-case", "LIGHTGOLDENRODYELLOW");
    print_parse("hex-hash", "#112233");
    print_parse("hex-0x-alpha", "0x11223344");
    print_parse("hex-bare", "112233");
    print_parse("hex-bare-alpha", "AABBCCDD");
    print_parse("alpha-hex", "red@0x80");
    print_parse("alpha-float-half", "Blue@0.5");
    print_parse("alpha-one", "white@1");
    print_parse("alpha-trunc", "red@0.999");
    print_parse("alpha-overrides-embedded", "#01020304@0x05");
    print_parse("invalid-empty", "");
    print_parse("invalid-short-hex", "#12345");
    print_parse("invalid-hex-byte", "#11223z");
    print_parse("invalid-uppercase-prefix", "0X112233");
    print_parse("invalid-unknown-name", "transparent");
    print_parse("invalid-empty-alpha", "white@");
    print_parse("invalid-hex-alpha-overflow", "red@0x100");
    print_parse("invalid-empty-hex-alpha", "red@0x");
    print_parse("invalid-double-alpha", "red@@0.5");
    return 0;
}
"##
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
