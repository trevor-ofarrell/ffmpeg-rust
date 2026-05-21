use core::cmp::Ordering;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    add_stable, compare_mod, compare_ts, rescale, rescale_delta, rescale_q, rescale_q_rnd,
    rescale_q_rnd_pass_minmax, rescale_rnd, rescale_rnd_pass_minmax, Rational, Rounding,
    AV_TIME_BASE, AV_TIME_BASE_Q,
};

#[derive(Clone, Copy)]
struct DeltaCase {
    in_tb: Rational,
    in_ts: i64,
    fs_tb: Rational,
    duration: i64,
    initial_last: i64,
    out_tb: Rational,
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_timebase_helpers_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/mathematics.h").is_file(),
        "missing pinned FFmpeg libavutil mathematics headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-timebase");
    fs::create_dir_all(&work_dir).expect("create avutil-timebase oracle work dir");
    let source = work_dir.join("timebase_oracle.c");
    let executable = work_dir.join("timebase_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-timebase oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);

    assert_i64(&oracle, "const:AV_TIME_BASE", AV_TIME_BASE);
    assert_q(&oracle, "const:AV_TIME_BASE_Q", AV_TIME_BASE_Q);

    assert_i64(&oracle, "rescale:three_halves", rescale(3, 1, 2).unwrap());
    assert_i64(
        &oracle,
        "rescale:negative_three_halves",
        rescale(-3, 1, 2).unwrap(),
    );
    assert_i64(
        &oracle,
        "rescale:ntsc_ticks",
        rescale(3_003, 1_000, 90_000).unwrap(),
    );

    assert_i64(
        &oracle,
        "rnd:zero_pos",
        rescale_rnd(1, 1, 3, Rounding::Zero).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:zero_neg",
        rescale_rnd(-1, 1, 3, Rounding::Zero).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:inf_pos",
        rescale_rnd(1, 1, 3, Rounding::Inf).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:inf_neg",
        rescale_rnd(-1, 1, 3, Rounding::Inf).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:down_neg",
        rescale_rnd(-1, 1, 3, Rounding::Down).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:up_neg",
        rescale_rnd(-1, 1, 3, Rounding::Up).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:near_half_pos",
        rescale_rnd(1, 1, 2, Rounding::NearInf).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:near_half_neg",
        rescale_rnd(-1, 1, 2, Rounding::NearInf).unwrap(),
    );

    assert_i64(
        &oracle,
        "rnd:min_pass",
        rescale_rnd_pass_minmax(i64::MIN, 1, 2, Rounding::Up).unwrap(),
    );
    assert_i64(
        &oracle,
        "rnd:max_pass",
        rescale_rnd_pass_minmax(i64::MAX, 1, 2, Rounding::Up).unwrap(),
    );

    let ninety_khz = Rational::new(1, 90_000).unwrap();
    let milliseconds = Rational::new(1, 1_000).unwrap();
    let seconds = Rational::ONE;
    let samples_48k = Rational::new(1, 48_000).unwrap();

    assert_i64(
        &oracle,
        "q:90k_to_ms",
        rescale_q(90_000, ninety_khz, milliseconds).unwrap(),
    );
    assert_i64(
        &oracle,
        "q:ntsc_to_ms",
        rescale_q(3_003, ninety_khz, milliseconds).unwrap(),
    );
    assert_i64(
        &oracle,
        "q_rnd:one_third_zero",
        rescale_q_rnd(1, Rational::new(1, 3).unwrap(), seconds, Rounding::Zero).unwrap(),
    );
    assert_i64(
        &oracle,
        "q_rnd:negative_third_up",
        rescale_q_rnd(-1, Rational::new(1, 3).unwrap(), seconds, Rounding::Up).unwrap(),
    );
    assert_i64(
        &oracle,
        "q_rnd:min_pass",
        rescale_q_rnd_pass_minmax(i64::MIN, milliseconds, seconds, Rounding::NearInf).unwrap(),
    );
    assert_i64(
        &oracle,
        "q_rnd:max_pass",
        rescale_q_rnd_pass_minmax(i64::MAX, milliseconds, seconds, Rounding::NearInf).unwrap(),
    );

    assert_i32(
        &oracle,
        "compare_ts:equal",
        ordering_value(compare_ts(1_000, milliseconds, 1, seconds).unwrap()),
    );
    assert_i32(
        &oracle,
        "compare_ts:greater",
        ordering_value(compare_ts(3_003, ninety_khz, 33, milliseconds).unwrap()),
    );
    assert_i32(
        &oracle,
        "compare_ts:less",
        ordering_value(compare_ts(-500, milliseconds, 0, seconds).unwrap()),
    );
    assert_i32(
        &oracle,
        "compare_ts:wide",
        ordering_value(
            compare_ts(
                i64::MAX / 4,
                Rational::new(1, 3).unwrap(),
                i64::MAX / 2,
                Rational::new(1, 2).unwrap(),
            )
            .unwrap(),
        ),
    );

    assert_i64(
        &oracle,
        "compare_mod:negative",
        compare_mod(0x11, 0x02, 0x10).unwrap(),
    );
    assert_i64(
        &oracle,
        "compare_mod:positive",
        compare_mod(0x11, 0x02, 0x20).unwrap(),
    );
    assert_i64(
        &oracle,
        "compare_mod:wrapped",
        compare_mod(u64::MAX, 0, 1 << 4).unwrap(),
    );
    assert_i64(
        &oracle,
        "compare_mod:zero",
        compare_mod(0x12, 0x02, 0x10).unwrap(),
    );

    assert_rescale_delta(
        &oracle,
        "delta:first",
        DeltaCase {
            in_tb: milliseconds,
            in_ts: 100,
            fs_tb: samples_48k,
            duration: 1_024,
            initial_last: i64::MIN,
            out_tb: ninety_khz,
        },
    );
    assert_rescale_delta(
        &oracle,
        "delta:zero_duration",
        DeltaCase {
            in_tb: milliseconds,
            in_ts: 250,
            fs_tb: samples_48k,
            duration: 0,
            initial_last: 123,
            out_tb: ninety_khz,
        },
    );
    assert_rescale_delta(
        &oracle,
        "delta:stateful",
        DeltaCase {
            in_tb: milliseconds,
            in_ts: 1_000,
            fs_tb: samples_48k,
            duration: 1_024,
            initial_last: 48_010,
            out_tb: samples_48k,
        },
    );
    assert_rescale_delta(
        &oracle,
        "delta:clip",
        DeltaCase {
            in_tb: milliseconds,
            in_ts: 1_000,
            fs_tb: samples_48k,
            duration: 1_024,
            initial_last: 48_050,
            out_tb: samples_48k,
        },
    );
    assert_rescale_delta(
        &oracle,
        "delta:fallback",
        DeltaCase {
            in_tb: milliseconds,
            in_ts: 1_000,
            fs_tb: samples_48k,
            duration: 1_024,
            initial_last: 47_000,
            out_tb: samples_48k,
        },
    );

    assert_i64(
        &oracle,
        "stable:exact_positive",
        add_stable(milliseconds, 1_000, milliseconds, 40).unwrap(),
    );
    assert_i64(
        &oracle,
        "stable:exact_negative",
        add_stable(milliseconds, 1_000, milliseconds, -40).unwrap(),
    );
    assert_i64(
        &oracle,
        "stable:sub_tick",
        add_stable(milliseconds, 123, samples_48k, 1).unwrap(),
    );
    assert_i64(
        &oracle,
        "stable:fractional_negative",
        add_stable(milliseconds, 1_000, Rational::new(1, 30).unwrap(), -1).unwrap(),
    );

    let mut ts = 0;
    for idx in 0..10 {
        ts = add_stable(milliseconds, ts, Rational::new(1, 30).unwrap(), 1).unwrap();
        assert_i64(&oracle, &format!("stable:sequence:{idx}"), ts);
    }
}

fn assert_rescale_delta(rows: &BTreeMap<String, Vec<String>>, name: &str, case: DeltaCase) {
    let mut last = case.initial_last;
    let output = rescale_delta(
        case.in_tb,
        case.in_ts,
        case.fs_tb,
        case.duration,
        &mut last,
        case.out_tb,
    )
    .unwrap();
    assert_eq!(
        row_fields(rows, name),
        &[output.to_string(), last.to_string()],
        "{name}"
    );
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

fn assert_i32(rows: &BTreeMap<String, Vec<String>>, name: &str, expected: i32) {
    assert_eq!(row_fields(rows, name), &[expected.to_string()], "{name}");
}

fn assert_i64(rows: &BTreeMap<String, Vec<String>>, name: &str, expected: i64) {
    assert_eq!(row_fields(rows, name), &[expected.to_string()], "{name}");
}

fn assert_q(rows: &BTreeMap<String, Vec<String>>, name: &str, expected: Rational) {
    assert_eq!(
        row_fields(rows, name),
        &[expected.num().to_string(), expected.den().to_string()],
        "{name}"
    );
}

fn row_fields<'a>(rows: &'a BTreeMap<String, Vec<String>>, name: &str) -> &'a [String] {
    rows.get(name)
        .unwrap_or_else(|| panic!("missing oracle row `{name}`"))
}

fn ordering_value(ordering: Ordering) -> i32 {
    match ordering {
        Ordering::Greater => 1,
        Ordering::Equal => 0,
        Ordering::Less => -1,
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
            .expect("run WSL libavutil timebase oracle")
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
            .expect("run libavutil timebase oracle")
    };

    assert!(
        output.status.success(),
        "libavutil timebase oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <libavutil/avutil.h>
#include <libavutil/mathematics.h>
#include <libavutil/rational.h>

#define QR(num, den) ((AVRational){ (num), (den) })

static void print_i32(const char *name, int value) {
    printf("%s|%d\n", name, value);
}

static void print_i64(const char *name, int64_t value) {
    printf("%s|%" PRId64 "\n", name, value);
}

static void print_q(const char *name, AVRational value) {
    printf("%s|%d|%d\n", name, value.num, value.den);
}

static void print_rescale_delta(
    const char *name,
    AVRational in_tb,
    int64_t in_ts,
    AVRational fs_tb,
    int duration,
    int64_t initial_last,
    AVRational out_tb
) {
    int64_t last = initial_last;
    int64_t out = av_rescale_delta(in_tb, in_ts, fs_tb, duration, &last, out_tb);
    printf("%s|%" PRId64 "|%" PRId64 "\n", name, out, last);
}

int main(void) {
    print_i64("const:AV_TIME_BASE", AV_TIME_BASE);
    print_q("const:AV_TIME_BASE_Q", AV_TIME_BASE_Q);

    print_i64("rescale:three_halves", av_rescale(3, 1, 2));
    print_i64("rescale:negative_three_halves", av_rescale(-3, 1, 2));
    print_i64("rescale:ntsc_ticks", av_rescale(3003, 1000, 90000));

    print_i64("rnd:zero_pos", av_rescale_rnd(1, 1, 3, AV_ROUND_ZERO));
    print_i64("rnd:zero_neg", av_rescale_rnd(-1, 1, 3, AV_ROUND_ZERO));
    print_i64("rnd:inf_pos", av_rescale_rnd(1, 1, 3, AV_ROUND_INF));
    print_i64("rnd:inf_neg", av_rescale_rnd(-1, 1, 3, AV_ROUND_INF));
    print_i64("rnd:down_neg", av_rescale_rnd(-1, 1, 3, AV_ROUND_DOWN));
    print_i64("rnd:up_neg", av_rescale_rnd(-1, 1, 3, AV_ROUND_UP));
    print_i64("rnd:near_half_pos", av_rescale_rnd(1, 1, 2, AV_ROUND_NEAR_INF));
    print_i64("rnd:near_half_neg", av_rescale_rnd(-1, 1, 2, AV_ROUND_NEAR_INF));
    print_i64("rnd:min_pass", av_rescale_rnd(INT64_MIN, 1, 2, AV_ROUND_UP | AV_ROUND_PASS_MINMAX));
    print_i64("rnd:max_pass", av_rescale_rnd(INT64_MAX, 1, 2, AV_ROUND_UP | AV_ROUND_PASS_MINMAX));

    print_i64("q:90k_to_ms", av_rescale_q(90000, QR(1, 90000), QR(1, 1000)));
    print_i64("q:ntsc_to_ms", av_rescale_q(3003, QR(1, 90000), QR(1, 1000)));
    print_i64("q_rnd:one_third_zero", av_rescale_q_rnd(1, QR(1, 3), QR(1, 1), AV_ROUND_ZERO));
    print_i64("q_rnd:negative_third_up", av_rescale_q_rnd(-1, QR(1, 3), QR(1, 1), AV_ROUND_UP));
    print_i64("q_rnd:min_pass", av_rescale_q_rnd(INT64_MIN, QR(1, 1000), QR(1, 1), AV_ROUND_NEAR_INF | AV_ROUND_PASS_MINMAX));
    print_i64("q_rnd:max_pass", av_rescale_q_rnd(INT64_MAX, QR(1, 1000), QR(1, 1), AV_ROUND_NEAR_INF | AV_ROUND_PASS_MINMAX));

    print_i32("compare_ts:equal", av_compare_ts(1000, QR(1, 1000), 1, QR(1, 1)));
    print_i32("compare_ts:greater", av_compare_ts(3003, QR(1, 90000), 33, QR(1, 1000)));
    print_i32("compare_ts:less", av_compare_ts(-500, QR(1, 1000), 0, QR(1, 1)));
    print_i32("compare_ts:wide", av_compare_ts(INT64_MAX / 4, QR(1, 3), INT64_MAX / 2, QR(1, 2)));

    print_i64("compare_mod:negative", av_compare_mod(0x11, 0x02, 0x10));
    print_i64("compare_mod:positive", av_compare_mod(0x11, 0x02, 0x20));
    print_i64("compare_mod:wrapped", av_compare_mod(UINT64_MAX, 0, 1ULL << 4));
    print_i64("compare_mod:zero", av_compare_mod(0x12, 0x02, 0x10));

    print_rescale_delta("delta:first", QR(1, 1000), 100, QR(1, 48000), 1024, INT64_MIN, QR(1, 90000));
    print_rescale_delta("delta:zero_duration", QR(1, 1000), 250, QR(1, 48000), 0, 123, QR(1, 90000));
    print_rescale_delta("delta:stateful", QR(1, 1000), 1000, QR(1, 48000), 1024, 48010, QR(1, 48000));
    print_rescale_delta("delta:clip", QR(1, 1000), 1000, QR(1, 48000), 1024, 48050, QR(1, 48000));
    print_rescale_delta("delta:fallback", QR(1, 1000), 1000, QR(1, 48000), 1024, 47000, QR(1, 48000));

    print_i64("stable:exact_positive", av_add_stable(QR(1, 1000), 1000, QR(1, 1000), 40));
    print_i64("stable:exact_negative", av_add_stable(QR(1, 1000), 1000, QR(1, 1000), -40));
    print_i64("stable:sub_tick", av_add_stable(QR(1, 1000), 123, QR(1, 48000), 1));
    print_i64("stable:fractional_negative", av_add_stable(QR(1, 1000), 1000, QR(1, 30), -1));

    {
        int i;
        int64_t ts = 0;
        char name[64];
        for (i = 0; i < 10; i++) {
            ts = av_add_stable(QR(1, 1000), ts, QR(1, 30), 1);
            snprintf(name, sizeof(name), "stable:sequence:%d", i);
            print_i64(name, ts);
        }
    }

    return 0;
}
"#
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
