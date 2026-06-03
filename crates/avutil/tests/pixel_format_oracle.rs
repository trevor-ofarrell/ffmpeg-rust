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
        "x2rgb10be",
        "x2bgr10be",
        "x2rgb10le",
        "x2bgr10le",
        "rgb24",
        "RGB24",
        "X2RGB10BE",
        "rgb24 ",
        " rgb24",
        " ",
        "",
        "\t",
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

    assert_libavutil_best_rows_match_bounded_model(&rows);
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

fn assert_libavutil_best_rows_match_bounded_model(rows: &BTreeMap<String, Vec<String>>) {
    let gray10 = native(PixelFormat::Gray10Le, PixelFormat::Gray10Be);
    let gray16 = native(PixelFormat::Gray16Le, PixelFormat::Gray16Be);
    let yuv420p10 = native(PixelFormat::Yuv420p10Le, PixelFormat::Yuv420p10Be);
    let yuv420p16 = native(PixelFormat::Yuv420p16Le, PixelFormat::Yuv420p16Be);
    let yuv422p10 = native(PixelFormat::Yuv422p10Le, PixelFormat::Yuv422p10Be);
    let yuv422p16 = native(PixelFormat::Yuv422p16Le, PixelFormat::Yuv422p16Be);
    let yuv444p10 = native(PixelFormat::Yuv444p10Le, PixelFormat::Yuv444p10Be);
    let yuv444p16 = native(PixelFormat::Yuv444p16Le, PixelFormat::Yuv444p16Be);
    let rgb565 = native(PixelFormat::Rgb565Le, PixelFormat::Rgb565Be);
    let rgb48 = native(PixelFormat::Rgb48Le, PixelFormat::Rgb48Be);
    let bgr565 = native(PixelFormat::Bgr565Le, PixelFormat::Bgr565Be);

    let base = vec![
        PixelFormat::MonoWhite,
        PixelFormat::Gray8,
        gray10,
        gray16,
        PixelFormat::Yuv420p,
        yuv420p10,
        yuv420p16,
        PixelFormat::Yuv422p,
        yuv422p10,
        yuv422p16,
        PixelFormat::Yuv444p,
        yuv444p10,
        yuv444p16,
        rgb565,
        PixelFormat::Rgb24,
        rgb48,
        PixelFormat::Vdpau,
        PixelFormat::Vaapi,
    ];

    assert_same_best_rows(rows, "base", &base);
    assert_best_cases(
        rows,
        "base",
        &base,
        &[
            ("monob", PixelFormat::MonoBlack),
            ("nv12", PixelFormat::Nv12),
            ("p010", native(PixelFormat::P010Le, PixelFormat::P010Be)),
            ("p012", native(PixelFormat::P012Le, PixelFormat::P012Be)),
            ("p016", native(PixelFormat::P016Le, PixelFormat::P016Be)),
            ("p210", native(PixelFormat::P210Le, PixelFormat::P210Be)),
            ("p212", native(PixelFormat::P212Le, PixelFormat::P212Be)),
            ("p216", native(PixelFormat::P216Le, PixelFormat::P216Be)),
            ("p410", native(PixelFormat::P410Le, PixelFormat::P410Be)),
            ("p412", native(PixelFormat::P412Le, PixelFormat::P412Be)),
            ("p416", native(PixelFormat::P416Le, PixelFormat::P416Be)),
            ("nv16", PixelFormat::Nv16),
            ("nv20", native(PixelFormat::Nv20Le, PixelFormat::Nv20Be)),
            ("nv24", PixelFormat::Nv24),
            ("yuyv422", PixelFormat::Yuyv422),
            ("uyvy422", PixelFormat::Uyvy422),
            ("vyu444", PixelFormat::Vyu444),
            ("bgr565", bgr565),
            ("bgr24", PixelFormat::Bgr24),
            ("gbrp", PixelFormat::Gbrp),
            ("0rgb", PixelFormat::ZeroRgb),
            (
                "gbrp16",
                native(PixelFormat::Gbrp16Le, PixelFormat::Gbrp16Be),
            ),
            ("vuyx", PixelFormat::Vuyx),
            ("ya8", PixelFormat::Ya8),
            ("ya16", native(PixelFormat::Ya16Le, PixelFormat::Ya16Be)),
            ("yuva420p", PixelFormat::Yuva420p),
            ("yuva422p", PixelFormat::Yuva422p),
            ("yuva444p", PixelFormat::Yuva444p),
            ("vuya", PixelFormat::Vuya),
            ("ayuv", PixelFormat::Ayuv),
            ("uyva", PixelFormat::Uyva),
            (
                "ayuv64",
                native(PixelFormat::Ayuv64Le, PixelFormat::Ayuv64Be),
            ),
            ("rgba", PixelFormat::Rgba),
            ("abgr", PixelFormat::Abgr),
            ("gbrap", PixelFormat::Gbrap),
            (
                "rgba64",
                native(PixelFormat::Rgba64Le, PixelFormat::Rgba64Be),
            ),
            (
                "bgra64",
                native(PixelFormat::Bgra64Le, PixelFormat::Bgra64Be),
            ),
            (
                "gbrap16",
                native(PixelFormat::Gbrap16Le, PixelFormat::Gbrap16Be),
            ),
            (
                "gray12",
                native(PixelFormat::Gray12Le, PixelFormat::Gray12Be),
            ),
            ("yuv410p", PixelFormat::Yuv410p),
            ("yuv411p", PixelFormat::Yuv411p),
            ("uyyvyy411", PixelFormat::Uyyvyy411),
            ("yuv440p", PixelFormat::Yuv440p),
            (
                "yuv440p10",
                native(PixelFormat::Yuv440p10Le, PixelFormat::Yuv440p10Be),
            ),
            (
                "yuv440p12",
                native(PixelFormat::Yuv440p12Le, PixelFormat::Yuv440p12Be),
            ),
            (
                "yuv420p9",
                native(PixelFormat::Yuv420p9Le, PixelFormat::Yuv420p9Be),
            ),
            (
                "yuv420p12",
                native(PixelFormat::Yuv420p12Le, PixelFormat::Yuv420p12Be),
            ),
            (
                "yuv444p9",
                native(PixelFormat::Yuv444p9Le, PixelFormat::Yuv444p9Be),
            ),
            (
                "yuv444p12",
                native(PixelFormat::Yuv444p12Le, PixelFormat::Yuv444p12Be),
            ),
            ("bgr4", PixelFormat::Bgr4),
            (
                "rgb444",
                native(PixelFormat::Rgb444Le, PixelFormat::Rgb444Be),
            ),
            (
                "rgb555",
                native(PixelFormat::Rgb555Le, PixelFormat::Rgb555Be),
            ),
            (
                "gbrp10",
                native(PixelFormat::Gbrp10Le, PixelFormat::Gbrp10Be),
            ),
            (
                "gbrap10",
                native(PixelFormat::Gbrap10Le, PixelFormat::Gbrap10Be),
            ),
            (
                "gbrap12",
                native(PixelFormat::Gbrap12Le, PixelFormat::Gbrap12Be),
            ),
            ("gray10be", PixelFormat::Gray10Be),
            ("gray10le", PixelFormat::Gray10Le),
            ("gray16be", PixelFormat::Gray16Be),
            ("gray16le", PixelFormat::Gray16Le),
            ("yuv422p10be", PixelFormat::Yuv422p10Be),
            ("yuv422p10le", PixelFormat::Yuv422p10Le),
            ("yuv444p16be", PixelFormat::Yuv444p16Be),
            ("yuv444p16le", PixelFormat::Yuv444p16Le),
            ("rgb565be", PixelFormat::Rgb565Be),
            ("rgb565le", PixelFormat::Rgb565Le),
            ("rgb48be", PixelFormat::Rgb48Be),
            ("rgb48le", PixelFormat::Rgb48Le),
            ("dxva2_vld", PixelFormat::Dxva2Vld),
        ],
    );

    let p010 = native(PixelFormat::P010Le, PixelFormat::P010Be);
    let p012 = native(PixelFormat::P012Le, PixelFormat::P012Be);
    let p016 = native(PixelFormat::P016Le, PixelFormat::P016Be);
    let p210 = native(PixelFormat::P210Le, PixelFormat::P210Be);
    let p216 = native(PixelFormat::P216Le, PixelFormat::P216Be);
    let p410 = native(PixelFormat::P410Le, PixelFormat::P410Be);
    let p416 = native(PixelFormat::P416Le, PixelFormat::P416Be);
    let semiplanar = vec![
        p016,
        p012,
        p010,
        p216,
        p210,
        PixelFormat::Nv16,
        p416,
        p410,
        PixelFormat::Nv24,
        PixelFormat::Nv12,
    ];
    assert_same_best_rows(rows, "semiplanar", &semiplanar);
    assert_best_cases(
        rows,
        "semiplanar",
        &semiplanar,
        &[
            ("yuv420p", PixelFormat::Yuv420p),
            ("yuv420p10", yuv420p10),
            (
                "yuv420p12",
                native(PixelFormat::Yuv420p12Le, PixelFormat::Yuv420p12Be),
            ),
            ("yuv420p16", yuv420p16),
            (
                "yuv420p9",
                native(PixelFormat::Yuv420p9Le, PixelFormat::Yuv420p9Be),
            ),
            ("yuv422p", PixelFormat::Yuv422p),
            ("yuv422p10", yuv422p10),
            (
                "yuv422p12",
                native(PixelFormat::Yuv422p12Le, PixelFormat::Yuv422p12Be),
            ),
            ("yuv422p16", yuv422p16),
            ("yuv444p", PixelFormat::Yuv444p),
            ("yuv444p10", yuv444p10),
            (
                "yuv444p12",
                native(PixelFormat::Yuv444p12Le, PixelFormat::Yuv444p12Be),
            ),
            ("yuv444p16", yuv444p16),
        ],
    );

    let xv48 = native(PixelFormat::Xv48Le, PixelFormat::Xv48Be);
    let xv36 = native(PixelFormat::Xv36Le, PixelFormat::Xv36Be);
    let xv30 = native(PixelFormat::Xv30Le, PixelFormat::Xv30Be);
    let y216 = native(PixelFormat::Y216Le, PixelFormat::Y216Be);
    let y212 = native(PixelFormat::Y212Le, PixelFormat::Y212Be);
    let y210 = native(PixelFormat::Y210Le, PixelFormat::Y210Be);
    let packed = vec![
        xv48,
        xv36,
        xv30,
        PixelFormat::Vuyx,
        y216,
        y212,
        y210,
        PixelFormat::Yuyv422,
    ];
    assert_same_best_rows(rows, "packed", &packed);
    assert_best_cases(
        rows,
        "packed",
        &packed,
        &[
            ("yuv444p", PixelFormat::Yuv444p),
            ("yuv444p10", yuv444p10),
            (
                "yuv444p12",
                native(PixelFormat::Yuv444p12Le, PixelFormat::Yuv444p12Be),
            ),
            ("yuv444p16", yuv444p16),
            ("yuv422p", PixelFormat::Yuv422p),
            ("yuv422p10", yuv422p10),
            (
                "yuv422p12",
                native(PixelFormat::Yuv422p12Le, PixelFormat::Yuv422p12Be),
            ),
            ("yuv422p16", yuv422p16),
        ],
    );

    let subsampled = [
        PixelFormat::Yuv411p,
        PixelFormat::Yuv420p,
        PixelFormat::Yuv422p,
        PixelFormat::Yuv444p,
    ];
    assert_same_best_rows(rows, "subsampled", &subsampled);
    assert_best_cases(
        rows,
        "subsampled",
        &subsampled,
        &[("yuv410p", PixelFormat::Yuv410p)],
    );

    let depthchroma = [
        native(PixelFormat::Yuv420p14Le, PixelFormat::Yuv420p14Be),
        native(PixelFormat::Yuv422p14Le, PixelFormat::Yuv422p14Be),
        yuv444p16,
    ];
    assert_same_best_rows(rows, "depthchroma", &depthchroma);
    assert_best_cases(
        rows,
        "depthchroma",
        &depthchroma,
        &[("yuv420p16", yuv420p16), ("yuv422p16", yuv422p16)],
    );
}

fn assert_same_best_rows(
    rows: &BTreeMap<String, Vec<String>>,
    group: &str,
    candidates: &[PixelFormat],
) {
    for (index, &input) in candidates.iter().enumerate() {
        assert_best_row(rows, group, &format!("same-{index}"), candidates, input);
    }
}

fn assert_best_cases(
    rows: &BTreeMap<String, Vec<String>>,
    group: &str,
    candidates: &[PixelFormat],
    cases: &[(&str, PixelFormat)],
) {
    for &(id, input) in cases {
        assert_best_row(rows, group, id, candidates, input);
    }
}

fn assert_best_row(
    rows: &BTreeMap<String, Vec<String>>,
    group: &str,
    id: &str,
    candidates: &[PixelFormat],
    input: PixelFormat,
) {
    let expected = PixelFormat::find_best(candidates.iter().copied(), input, false)
        .unwrap_or_else(|| panic!("missing bounded best pixel format for `{}`", input.name()));
    let key = format!("best:{group}:{id}");
    let expected_fields = [input.name().to_string(), expected.name().to_string()];
    assert_eq!(row_fields(rows, &key), &expected_fields, "{key} diverged");
}

fn native(le: PixelFormat, be: PixelFormat) -> PixelFormat {
    if cfg!(target_endian = "little") {
        le
    } else {
        be
    }
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

#define ARRAY_ELEMS(array) ((int)(sizeof(array) / sizeof((array)[0])))

static const enum AVPixelFormat pixfmt_list[] = {
    AV_PIX_FMT_MONOWHITE,
    AV_PIX_FMT_GRAY8,
    AV_PIX_FMT_GRAY10,
    AV_PIX_FMT_GRAY16,
    AV_PIX_FMT_YUV420P,
    AV_PIX_FMT_YUV420P10,
    AV_PIX_FMT_YUV420P16,
    AV_PIX_FMT_YUV422P,
    AV_PIX_FMT_YUV422P10,
    AV_PIX_FMT_YUV422P16,
    AV_PIX_FMT_YUV444P,
    AV_PIX_FMT_YUV444P10,
    AV_PIX_FMT_YUV444P16,
    AV_PIX_FMT_RGB565,
    AV_PIX_FMT_RGB24,
    AV_PIX_FMT_RGB48,
    AV_PIX_FMT_VDPAU,
    AV_PIX_FMT_VAAPI,
};

static const enum AVPixelFormat semiplanar_list[] = {
    AV_PIX_FMT_P016,
    AV_PIX_FMT_P012,
    AV_PIX_FMT_P010,
    AV_PIX_FMT_P216,
    AV_PIX_FMT_P210,
    AV_PIX_FMT_NV16,
    AV_PIX_FMT_P416,
    AV_PIX_FMT_P410,
    AV_PIX_FMT_NV24,
    AV_PIX_FMT_NV12,
};

static const enum AVPixelFormat packed_list[] = {
    AV_PIX_FMT_XV48,
    AV_PIX_FMT_XV36,
    AV_PIX_FMT_XV30,
    AV_PIX_FMT_VUYX,
    AV_PIX_FMT_Y216,
    AV_PIX_FMT_Y212,
    AV_PIX_FMT_Y210,
    AV_PIX_FMT_YUYV422,
};

static const enum AVPixelFormat subsampled_list[] = {
    AV_PIX_FMT_YUV411P,
    AV_PIX_FMT_YUV420P,
    AV_PIX_FMT_YUV422P,
    AV_PIX_FMT_YUV444P,
};

static const enum AVPixelFormat depthchroma_list[] = {
    AV_PIX_FMT_YUV420P14,
    AV_PIX_FMT_YUV422P14,
    AV_PIX_FMT_YUV444P16,
};

static enum AVPixelFormat find_best(const enum AVPixelFormat *list,
                                    int count,
                                    enum AVPixelFormat input) {
    enum AVPixelFormat best = AV_PIX_FMT_NONE;
    for (int i = 0; i < count; i++)
        best = av_find_best_pix_fmt_of_2(best, list[i], input, 0, NULL);
    return best;
}

static void print_best_case(const char *group,
                            const char *id,
                            enum AVPixelFormat input,
                            const enum AVPixelFormat *list,
                            int count) {
    enum AVPixelFormat best = find_best(list, count, input);
    const char *input_name = av_get_pix_fmt_name(input);
    const char *best_name = av_get_pix_fmt_name(best);
    printf("best:%s:%s|%s|%s\n",
           group,
           id,
           input_name ? input_name : "",
           best_name ? best_name : "");
}

static void print_best_same(const char *group,
                            const enum AVPixelFormat *list,
                            int count) {
    char id[32];
    for (int i = 0; i < count; i++) {
        snprintf(id, sizeof(id), "same-%d", i);
        print_best_case(group, id, list[i], list, count);
    }
}

#define PRINT_BEST(group, id, input, list) \
    print_best_case(group, id, input, list, ARRAY_ELEMS(list))

int main(void) {
    print_lookup("gray");
    print_lookup("gray8");
    print_lookup("gray8a");
    print_lookup("y400a");
    print_lookup("x2rgb10");
    print_lookup("x2bgr10");
    print_lookup("x2rgb10be");
    print_lookup("x2bgr10be");
    print_lookup("x2rgb10le");
    print_lookup("x2bgr10le");
    print_lookup("X2RGB10BE");
    print_lookup("rgb24 ");
    print_lookup(" rgb24");
    print_lookup(" ");
    print_lookup("");
    print_lookup("\t");
    print_lookup("rgb24");
    print_lookup("RGB24");
    print_lookup("y32le");
    print_lookup("yf32le");
    print_lookup("vaapi");
    print_lookup("not_a_pix_fmt");

    print_best_same("base", pixfmt_list, ARRAY_ELEMS(pixfmt_list));
    PRINT_BEST("base", "monob", AV_PIX_FMT_MONOBLACK, pixfmt_list);
    PRINT_BEST("base", "nv12", AV_PIX_FMT_NV12, pixfmt_list);
    PRINT_BEST("base", "p010", AV_PIX_FMT_P010, pixfmt_list);
    PRINT_BEST("base", "p012", AV_PIX_FMT_P012, pixfmt_list);
    PRINT_BEST("base", "p016", AV_PIX_FMT_P016, pixfmt_list);
    PRINT_BEST("base", "p210", AV_PIX_FMT_P210, pixfmt_list);
    PRINT_BEST("base", "p212", AV_PIX_FMT_P212, pixfmt_list);
    PRINT_BEST("base", "p216", AV_PIX_FMT_P216, pixfmt_list);
    PRINT_BEST("base", "p410", AV_PIX_FMT_P410, pixfmt_list);
    PRINT_BEST("base", "p412", AV_PIX_FMT_P412, pixfmt_list);
    PRINT_BEST("base", "p416", AV_PIX_FMT_P416, pixfmt_list);
    PRINT_BEST("base", "nv16", AV_PIX_FMT_NV16, pixfmt_list);
    PRINT_BEST("base", "nv20", AV_PIX_FMT_NV20, pixfmt_list);
    PRINT_BEST("base", "nv24", AV_PIX_FMT_NV24, pixfmt_list);
    PRINT_BEST("base", "yuyv422", AV_PIX_FMT_YUYV422, pixfmt_list);
    PRINT_BEST("base", "uyvy422", AV_PIX_FMT_UYVY422, pixfmt_list);
    PRINT_BEST("base", "vyu444", AV_PIX_FMT_VYU444, pixfmt_list);
    PRINT_BEST("base", "bgr565", AV_PIX_FMT_BGR565, pixfmt_list);
    PRINT_BEST("base", "bgr24", AV_PIX_FMT_BGR24, pixfmt_list);
    PRINT_BEST("base", "gbrp", AV_PIX_FMT_GBRP, pixfmt_list);
    PRINT_BEST("base", "0rgb", AV_PIX_FMT_0RGB, pixfmt_list);
    PRINT_BEST("base", "gbrp16", AV_PIX_FMT_GBRP16, pixfmt_list);
    PRINT_BEST("base", "vuyx", AV_PIX_FMT_VUYX, pixfmt_list);
    PRINT_BEST("base", "ya8", AV_PIX_FMT_YA8, pixfmt_list);
    PRINT_BEST("base", "ya16", AV_PIX_FMT_YA16, pixfmt_list);
    PRINT_BEST("base", "yuva420p", AV_PIX_FMT_YUVA420P, pixfmt_list);
    PRINT_BEST("base", "yuva422p", AV_PIX_FMT_YUVA422P, pixfmt_list);
    PRINT_BEST("base", "yuva444p", AV_PIX_FMT_YUVA444P, pixfmt_list);
    PRINT_BEST("base", "vuya", AV_PIX_FMT_VUYA, pixfmt_list);
    PRINT_BEST("base", "ayuv", AV_PIX_FMT_AYUV, pixfmt_list);
    PRINT_BEST("base", "uyva", AV_PIX_FMT_UYVA, pixfmt_list);
    PRINT_BEST("base", "ayuv64", AV_PIX_FMT_AYUV64, pixfmt_list);
    PRINT_BEST("base", "rgba", AV_PIX_FMT_RGBA, pixfmt_list);
    PRINT_BEST("base", "abgr", AV_PIX_FMT_ABGR, pixfmt_list);
    PRINT_BEST("base", "gbrap", AV_PIX_FMT_GBRAP, pixfmt_list);
    PRINT_BEST("base", "rgba64", AV_PIX_FMT_RGBA64, pixfmt_list);
    PRINT_BEST("base", "bgra64", AV_PIX_FMT_BGRA64, pixfmt_list);
    PRINT_BEST("base", "gbrap16", AV_PIX_FMT_GBRAP16, pixfmt_list);
    PRINT_BEST("base", "gray12", AV_PIX_FMT_GRAY12, pixfmt_list);
    PRINT_BEST("base", "yuv410p", AV_PIX_FMT_YUV410P, pixfmt_list);
    PRINT_BEST("base", "yuv411p", AV_PIX_FMT_YUV411P, pixfmt_list);
    PRINT_BEST("base", "uyyvyy411", AV_PIX_FMT_UYYVYY411, pixfmt_list);
    PRINT_BEST("base", "yuv440p", AV_PIX_FMT_YUV440P, pixfmt_list);
    PRINT_BEST("base", "yuv440p10", AV_PIX_FMT_YUV440P10, pixfmt_list);
    PRINT_BEST("base", "yuv440p12", AV_PIX_FMT_YUV440P12, pixfmt_list);
    PRINT_BEST("base", "yuv420p9", AV_PIX_FMT_YUV420P9, pixfmt_list);
    PRINT_BEST("base", "yuv420p12", AV_PIX_FMT_YUV420P12, pixfmt_list);
    PRINT_BEST("base", "yuv444p9", AV_PIX_FMT_YUV444P9, pixfmt_list);
    PRINT_BEST("base", "yuv444p12", AV_PIX_FMT_YUV444P12, pixfmt_list);
    PRINT_BEST("base", "bgr4", AV_PIX_FMT_BGR4, pixfmt_list);
    PRINT_BEST("base", "rgb444", AV_PIX_FMT_RGB444, pixfmt_list);
    PRINT_BEST("base", "rgb555", AV_PIX_FMT_RGB555, pixfmt_list);
    PRINT_BEST("base", "gbrp10", AV_PIX_FMT_GBRP10, pixfmt_list);
    PRINT_BEST("base", "gbrap10", AV_PIX_FMT_GBRAP10, pixfmt_list);
    PRINT_BEST("base", "gbrap12", AV_PIX_FMT_GBRAP12, pixfmt_list);
    PRINT_BEST("base", "gray10be", AV_PIX_FMT_GRAY10BE, pixfmt_list);
    PRINT_BEST("base", "gray10le", AV_PIX_FMT_GRAY10LE, pixfmt_list);
    PRINT_BEST("base", "gray16be", AV_PIX_FMT_GRAY16BE, pixfmt_list);
    PRINT_BEST("base", "gray16le", AV_PIX_FMT_GRAY16LE, pixfmt_list);
    PRINT_BEST("base", "yuv422p10be", AV_PIX_FMT_YUV422P10BE, pixfmt_list);
    PRINT_BEST("base", "yuv422p10le", AV_PIX_FMT_YUV422P10LE, pixfmt_list);
    PRINT_BEST("base", "yuv444p16be", AV_PIX_FMT_YUV444P16BE, pixfmt_list);
    PRINT_BEST("base", "yuv444p16le", AV_PIX_FMT_YUV444P16LE, pixfmt_list);
    PRINT_BEST("base", "rgb565be", AV_PIX_FMT_RGB565BE, pixfmt_list);
    PRINT_BEST("base", "rgb565le", AV_PIX_FMT_RGB565LE, pixfmt_list);
    PRINT_BEST("base", "rgb48be", AV_PIX_FMT_RGB48BE, pixfmt_list);
    PRINT_BEST("base", "rgb48le", AV_PIX_FMT_RGB48LE, pixfmt_list);
    PRINT_BEST("base", "dxva2_vld", AV_PIX_FMT_DXVA2_VLD, pixfmt_list);

    print_best_same("semiplanar", semiplanar_list, ARRAY_ELEMS(semiplanar_list));
    PRINT_BEST("semiplanar", "yuv420p", AV_PIX_FMT_YUV420P, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv420p10", AV_PIX_FMT_YUV420P10, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv420p12", AV_PIX_FMT_YUV420P12, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv420p16", AV_PIX_FMT_YUV420P16, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv420p9", AV_PIX_FMT_YUV420P9, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv422p", AV_PIX_FMT_YUV422P, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv422p10", AV_PIX_FMT_YUV422P10, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv422p12", AV_PIX_FMT_YUV422P12, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv422p16", AV_PIX_FMT_YUV422P16, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv444p", AV_PIX_FMT_YUV444P, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv444p10", AV_PIX_FMT_YUV444P10, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv444p12", AV_PIX_FMT_YUV444P12, semiplanar_list);
    PRINT_BEST("semiplanar", "yuv444p16", AV_PIX_FMT_YUV444P16, semiplanar_list);

    print_best_same("packed", packed_list, ARRAY_ELEMS(packed_list));
    PRINT_BEST("packed", "yuv444p", AV_PIX_FMT_YUV444P, packed_list);
    PRINT_BEST("packed", "yuv444p10", AV_PIX_FMT_YUV444P10, packed_list);
    PRINT_BEST("packed", "yuv444p12", AV_PIX_FMT_YUV444P12, packed_list);
    PRINT_BEST("packed", "yuv444p16", AV_PIX_FMT_YUV444P16, packed_list);
    PRINT_BEST("packed", "yuv422p", AV_PIX_FMT_YUV422P, packed_list);
    PRINT_BEST("packed", "yuv422p10", AV_PIX_FMT_YUV422P10, packed_list);
    PRINT_BEST("packed", "yuv422p12", AV_PIX_FMT_YUV422P12, packed_list);
    PRINT_BEST("packed", "yuv422p16", AV_PIX_FMT_YUV422P16, packed_list);

    print_best_same("subsampled", subsampled_list, ARRAY_ELEMS(subsampled_list));
    PRINT_BEST("subsampled", "yuv410p", AV_PIX_FMT_YUV410P, subsampled_list);

    print_best_same("depthchroma", depthchroma_list, ARRAY_ELEMS(depthchroma_list));
    PRINT_BEST("depthchroma", "yuv420p16", AV_PIX_FMT_YUV420P16, depthchroma_list);
    PRINT_BEST("depthchroma", "yuv422p16", AV_PIX_FMT_YUV422P16, depthchroma_list);
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
