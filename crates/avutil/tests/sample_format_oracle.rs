use avutil::SampleFormat;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleFormatRow {
    name: String,
    depth: usize,
}

#[derive(Debug, Clone, Copy)]
struct SampleBufferCase {
    id: &'static str,
    format: SampleFormat,
    channels: u16,
    samples: usize,
    alignment: usize,
}

const BUFFER_CASES: &[SampleBufferCase] = &[
    SampleBufferCase {
        id: "s16-c2-s3-a1",
        format: SampleFormat::S16,
        channels: 2,
        samples: 3,
        alignment: 1,
    },
    SampleBufferCase {
        id: "s16-c2-s3-a8",
        format: SampleFormat::S16,
        channels: 2,
        samples: 3,
        alignment: 8,
    },
    SampleBufferCase {
        id: "s16p-c2-s3-a1",
        format: SampleFormat::S16P,
        channels: 2,
        samples: 3,
        alignment: 1,
    },
    SampleBufferCase {
        id: "fltp-c2-s3-a16",
        format: SampleFormat::FltP,
        channels: 2,
        samples: 3,
        alignment: 16,
    },
    SampleBufferCase {
        id: "u8p-c2-s33-a0",
        format: SampleFormat::U8P,
        channels: 2,
        samples: 33,
        alignment: 0,
    },
    SampleBufferCase {
        id: "u8-c1-s1-a0",
        format: SampleFormat::U8,
        channels: 1,
        samples: 1,
        alignment: 0,
    },
];

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

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_sample_format_helpers_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/samplefmt.h").is_file(),
        "missing pinned FFmpeg libavutil sample format headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-sample-format");
    fs::create_dir_all(&work_dir).expect("create avutil-sample-format oracle work dir");
    let source = work_dir.join("sample_format_oracle.c");
    let executable = work_dir.join("sample_format_oracle");
    fs::write(&source, oracle_c_source()).expect("write sample format oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);

    for format in SampleFormat::ALL {
        assert_eq!(
            row_fields(&oracle, &format!("meta:{}", format.name())),
            &[
                format.name().to_string(),
                format.bytes_per_sample().to_string(),
                bool_int(format.is_planar()).to_string(),
                format.packed().name().to_string(),
                format.planar().name().to_string(),
                format.with_planar(false).name().to_string(),
                format.with_planar(true).name().to_string(),
                format.sample_fmt_string(),
            ],
            "{} metadata diverged",
            format.name()
        );
    }

    for case in BUFFER_CASES {
        let layout = case
            .format
            .buffer_layout(case.samples, case.channels, case.alignment)
            .unwrap();
        assert_eq!(
            row_fields(&oracle, &format!("buffer:{}", case.id)),
            &[
                layout.buffer_size().to_string(),
                layout.line_size().to_string()
            ],
            "{} buffer layout diverged",
            case.id
        );

        let fill = case
            .format
            .fill_arrays_layout(case.samples, case.channels, case.alignment)
            .unwrap();
        let plane0 = fill.plane_ranges()[0].byte_offset().to_string();
        let plane1 = fill
            .plane_ranges()
            .get(1)
            .map(|plane| plane.byte_offset().to_string())
            .unwrap_or_else(|| "-1".to_string());
        assert_eq!(
            row_fields(&oracle, &format!("fill:{}", case.id)),
            &[
                fill.buffer_size().to_string(),
                fill.line_size().to_string(),
                plane0,
                plane1,
            ],
            "{} fill-array layout diverged",
            case.id
        );
    }

    assert_eq!(
        row_fields(&oracle, "silence:s16-packed"),
        &[hex(&expected_s16_packed_silence())]
    );
    assert_eq!(
        row_fields(&oracle, "silence:u8-packed"),
        &[hex(&expected_u8_packed_silence())]
    );
    let u8p_silence = expected_u8p_planar_silence();
    assert_eq!(
        row_fields(&oracle, "silence:u8p-planar"),
        &[hex(&u8p_silence[0]), hex(&u8p_silence[1])]
    );

    assert_eq!(
        row_fields(&oracle, "copy:s16-packed"),
        &[hex(&expected_s16_packed_copy())]
    );
    let u8p_copy = expected_u8p_planar_copy();
    assert_eq!(
        row_fields(&oracle, "copy:u8p-planar"),
        &[hex(&u8p_copy[0]), hex(&u8p_copy[1])]
    );
    assert_eq!(
        row_fields(&oracle, "copy:u8-overlap-forward"),
        &[hex(&expected_u8_overlap_forward_copy())]
    );

    assert_eq!(
        row_fields(&oracle, "alloc:s16-packed"),
        &expected_alloc_fields(SampleFormat::S16, 2, 3, 1)
    );
    assert_eq!(
        row_fields(&oracle, "alloc:u8p-planar"),
        &expected_alloc_fields(SampleFormat::U8P, 2, 3, 8)
    );
    assert_eq!(
        row_fields(&oracle, "alloc:u8-auto"),
        &expected_alloc_fields(SampleFormat::U8, 1, 1, 0)
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

fn expected_s16_packed_silence() -> Vec<u8> {
    let mut planes = vec![vec![0x7f; 24]];
    SampleFormat::S16
        .fill_silence(&mut planes, 1, 2, 2)
        .unwrap();
    planes.remove(0)
}

fn expected_u8_packed_silence() -> Vec<u8> {
    let mut planes = vec![vec![0x7f; 16]];
    SampleFormat::U8.fill_silence(&mut planes, 0, 3, 2).unwrap();
    planes.remove(0)
}

fn expected_u8p_planar_silence() -> [Vec<u8>; 2] {
    let mut planes = vec![vec![0x11; 8], vec![0x22; 8]];
    SampleFormat::U8P
        .fill_silence(&mut planes, 2, 3, 2)
        .unwrap();
    [planes.remove(0), planes.remove(0)]
}

fn expected_s16_packed_copy() -> Vec<u8> {
    let src = vec![(0u8..24).collect::<Vec<_>>()];
    let mut dst = vec![vec![0xff; 24]];
    SampleFormat::S16
        .copy_samples(&mut dst, &src, 2, 1, 3, 2)
        .unwrap();
    dst.remove(0)
}

fn expected_u8p_planar_copy() -> [Vec<u8>; 2] {
    let src = vec![
        vec![0, 1, 2, 3, 4, 5, 6, 7],
        vec![10, 11, 12, 13, 14, 15, 16, 17],
    ];
    let mut dst = vec![vec![0xa0; 8], vec![0xb0; 8]];
    SampleFormat::U8P
        .copy_samples(&mut dst, &src, 3, 1, 3, 2)
        .unwrap();
    [dst.remove(0), dst.remove(0)]
}

fn expected_u8_overlap_forward_copy() -> Vec<u8> {
    let mut planes = vec![vec![0, 1, 2, 3, 4, 5, 6]];
    SampleFormat::U8
        .copy_samples_within(&mut planes, 2, 0, 4, 1)
        .unwrap();
    planes.remove(0)
}

fn expected_alloc_fields(
    format: SampleFormat,
    channels: u16,
    samples: usize,
    alignment: usize,
) -> Vec<String> {
    let allocation = format
        .alloc_samples(samples, channels, alignment)
        .expect("expected sample allocation");
    let planes = allocation.planes().expect("allocated planes");
    let block = if format.is_planar() {
        format.bytes_per_sample()
    } else {
        format.bytes_per_sample() * usize::from(channels)
    };
    let requested_len = samples * block;
    let mut fields = vec![
        allocation.buffer_size().to_string(),
        allocation.line_size().to_string(),
        hex(&planes[0][..requested_len]),
    ];
    if let Some(plane) = planes.get(1) {
        fields.push(hex(&plane[..requested_len]));
    } else {
        fields.push(String::new());
    }
    fields
}

fn bool_int(value: bool) -> u8 {
    u8::from(value)
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("write hex byte");
    }
    output
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
            .expect("run WSL libavutil sample format oracle")
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
            .expect("run libavutil sample format oracle")
    };

    assert!(
        output.status.success(),
        "libavutil sample format oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <libavutil/mem.h>
#include <libavutil/samplefmt.h>

static void print_bytes(const uint8_t *data, int len) {
    for (int i = 0; i < len; i++)
        printf("%02x", data[i]);
}

static enum AVSampleFormat checked_fmt(const char *name) {
    enum AVSampleFormat fmt = av_get_sample_fmt(name);
    if (fmt == AV_SAMPLE_FMT_NONE) {
        fprintf(stderr, "unknown sample format %s\n", name);
        exit(1);
    }
    return fmt;
}

static const char *fmt_name(enum AVSampleFormat fmt) {
    const char *name = av_get_sample_fmt_name(fmt);
    return name ? name : "";
}

static void print_meta(const char *name) {
    enum AVSampleFormat fmt = checked_fmt(name);
    char fmt_string[32];
    av_get_sample_fmt_string(fmt_string, sizeof(fmt_string), fmt);
    printf("meta:%s|%s|%d|%d|%s|%s|%s|%s|%s\n",
           name,
           fmt_name(fmt),
           av_get_bytes_per_sample(fmt),
           av_sample_fmt_is_planar(fmt),
           fmt_name(av_get_packed_sample_fmt(fmt)),
           fmt_name(av_get_planar_sample_fmt(fmt)),
           fmt_name(av_get_alt_sample_fmt(fmt, 0)),
           fmt_name(av_get_alt_sample_fmt(fmt, 1)),
           fmt_string);
}

static void print_buffer_case(const char *id, const char *name, int channels,
                              int samples, int align) {
    int linesize = -1;
    enum AVSampleFormat fmt = checked_fmt(name);
    int ret = av_samples_get_buffer_size(&linesize, channels, samples, fmt, align);
    printf("buffer:%s|%d|%d\n", id, ret, linesize);
}

static void print_fill_case(const char *id, const char *name, int channels,
                            int samples, int align) {
    uint8_t buffer[256];
    uint8_t *data[8] = { 0 };
    int linesize = -1;
    enum AVSampleFormat fmt = checked_fmt(name);
    int ret = av_samples_fill_arrays(data, &linesize, buffer, channels, samples, fmt, align);
    long off0 = data[0] ? (long)(data[0] - buffer) : -1;
    long off1 = data[1] ? (long)(data[1] - buffer) : -1;
    printf("fill:%s|%d|%d|%ld|%ld\n", id, ret, linesize, off0, off1);
}

static void print_alloc_case(const char *id, const char *name, int channels,
                             int samples, int align) {
    uint8_t *data[8] = { 0 };
    int linesize = -1;
    enum AVSampleFormat fmt = checked_fmt(name);
    int ret = av_samples_alloc(data, &linesize, channels, samples, fmt, align);
    if (ret < 0) {
        printf("alloc:%s|%d|%d||\n", id, ret, linesize);
        return;
    }

    int block = av_get_bytes_per_sample(fmt);
    if (!av_sample_fmt_is_planar(fmt))
        block *= channels;
    int requested_len = samples * block;
    printf("alloc:%s|%d|%d|", id, ret, linesize);
    print_bytes(data[0], requested_len);
    printf("|");
    if (data[1])
        print_bytes(data[1], requested_len);
    printf("\n");
    av_freep(&data[0]);
}

static void print_silence_cases(void) {
    uint8_t s16[24];
    memset(s16, 0x7f, sizeof(s16));
    uint8_t *s16_data[1] = { s16 };
    av_samples_set_silence(s16_data, 1, 2, 2, AV_SAMPLE_FMT_S16);
    printf("silence:s16-packed|");
    print_bytes(s16, sizeof(s16));
    printf("\n");

    uint8_t u8[16];
    memset(u8, 0x7f, sizeof(u8));
    uint8_t *u8_data[1] = { u8 };
    av_samples_set_silence(u8_data, 0, 3, 2, AV_SAMPLE_FMT_U8);
    printf("silence:u8-packed|");
    print_bytes(u8, sizeof(u8));
    printf("\n");

    uint8_t u8p0[8];
    uint8_t u8p1[8];
    memset(u8p0, 0x11, sizeof(u8p0));
    memset(u8p1, 0x22, sizeof(u8p1));
    uint8_t *u8p_data[2] = { u8p0, u8p1 };
    av_samples_set_silence(u8p_data, 2, 3, 2, AV_SAMPLE_FMT_U8P);
    printf("silence:u8p-planar|");
    print_bytes(u8p0, sizeof(u8p0));
    printf("|");
    print_bytes(u8p1, sizeof(u8p1));
    printf("\n");
}

static void print_copy_cases(void) {
    uint8_t s16_src[24];
    uint8_t s16_dst[24];
    for (int i = 0; i < 24; i++)
        s16_src[i] = (uint8_t)i;
    memset(s16_dst, 0xff, sizeof(s16_dst));
    uint8_t *s16_src_data[1] = { s16_src };
    uint8_t *s16_dst_data[1] = { s16_dst };
    av_samples_copy(s16_dst_data, s16_src_data, 2, 1, 3, 2, AV_SAMPLE_FMT_S16);
    printf("copy:s16-packed|");
    print_bytes(s16_dst, sizeof(s16_dst));
    printf("\n");

    uint8_t u8p_src0[8] = { 0, 1, 2, 3, 4, 5, 6, 7 };
    uint8_t u8p_src1[8] = { 10, 11, 12, 13, 14, 15, 16, 17 };
    uint8_t u8p_dst0[8];
    uint8_t u8p_dst1[8];
    memset(u8p_dst0, 0xa0, sizeof(u8p_dst0));
    memset(u8p_dst1, 0xb0, sizeof(u8p_dst1));
    uint8_t *u8p_src_data[2] = { u8p_src0, u8p_src1 };
    uint8_t *u8p_dst_data[2] = { u8p_dst0, u8p_dst1 };
    av_samples_copy(u8p_dst_data, u8p_src_data, 3, 1, 3, 2, AV_SAMPLE_FMT_U8P);
    printf("copy:u8p-planar|");
    print_bytes(u8p_dst0, sizeof(u8p_dst0));
    printf("|");
    print_bytes(u8p_dst1, sizeof(u8p_dst1));
    printf("\n");

    uint8_t overlap[7] = { 0, 1, 2, 3, 4, 5, 6 };
    uint8_t *overlap_data[1] = { overlap };
    av_samples_copy(overlap_data, overlap_data, 2, 0, 4, 1, AV_SAMPLE_FMT_U8);
    printf("copy:u8-overlap-forward|");
    print_bytes(overlap, sizeof(overlap));
    printf("\n");
}

int main(void) {
    static const char *formats[] = {
        "u8", "s16", "s32", "flt", "dbl", "u8p", "s16p", "s32p",
        "fltp", "dblp", "s64", "s64p",
    };

    for (size_t i = 0; i < sizeof(formats) / sizeof(formats[0]); i++)
        print_meta(formats[i]);

    print_buffer_case("s16-c2-s3-a1", "s16", 2, 3, 1);
    print_buffer_case("s16-c2-s3-a8", "s16", 2, 3, 8);
    print_buffer_case("s16p-c2-s3-a1", "s16p", 2, 3, 1);
    print_buffer_case("fltp-c2-s3-a16", "fltp", 2, 3, 16);
    print_buffer_case("u8p-c2-s33-a0", "u8p", 2, 33, 0);
    print_buffer_case("u8-c1-s1-a0", "u8", 1, 1, 0);

    print_fill_case("s16-c2-s3-a1", "s16", 2, 3, 1);
    print_fill_case("s16-c2-s3-a8", "s16", 2, 3, 8);
    print_fill_case("s16p-c2-s3-a1", "s16p", 2, 3, 1);
    print_fill_case("fltp-c2-s3-a16", "fltp", 2, 3, 16);
    print_fill_case("u8p-c2-s33-a0", "u8p", 2, 33, 0);
    print_fill_case("u8-c1-s1-a0", "u8", 1, 1, 0);

    print_silence_cases();
    print_copy_cases();
    print_alloc_case("s16-packed", "s16", 2, 3, 1);
    print_alloc_case("u8p-planar", "u8p", 2, 3, 8);
    print_alloc_case("u8-auto", "u8", 1, 1, 0);

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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/avutil should have a repo root grandparent")
        .to_path_buf()
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
