use core::cmp::Ordering;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::Rational;

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_rational_helpers_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/rational.h").is_file(),
        "missing pinned FFmpeg libavutil rational headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-rational");
    fs::create_dir_all(&work_dir).expect("create avutil-rational oracle work dir");
    let source = work_dir.join("rational_oracle.c");
    let executable = work_dir.join("rational_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-rational oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);

    assert_i32(
        &oracle,
        "cmp:half_vs_two_thirds",
        cmp_value(
            Rational::new(1, 2)
                .unwrap()
                .av_cmp(Rational::new(2, 3).unwrap()),
        ),
    );
    assert_i32(
        &oracle,
        "cmp:positive_infinity_vs_finite",
        cmp_value(Rational::from_raw(1, 0).av_cmp(Rational::new(24, 1).unwrap())),
    );
    assert_i32(
        &oracle,
        "cmp:nan_vs_finite",
        cmp_value(Rational::from_raw(0, 0).av_cmp(Rational::new(1, 1).unwrap())),
    );

    assert_double_bits(&oracle, "q2d:half", Rational::new(1, 2).unwrap());
    assert_double_bits(
        &oracle,
        "q2d:negative_quarter",
        Rational::new(-1, 4).unwrap(),
    );
    assert_double_bits(&oracle, "q2d:positive_infinity", Rational::from_raw(1, 0));

    assert_reduce(&oracle, "reduce:ntsc", 30000, -1001, i32::MAX);
    assert_reduce(&oracle, "reduce:zero", 0, 4000, 100);
    assert_reduce(&oracle, "reduce:positive_infinity", 42, 0, 100);
    assert_reduce(&oracle, "reduce:nan", 0, 0, 100);
    assert_reduce(&oracle, "reduce:ntsc_limited", 1001, 30000, 100);
    assert_reduce(&oracle, "reduce:two_thirds_max_one", 2, 3, 1);
    assert_reduce(&oracle, "reduce:negative_two_thirds_max_one", -2, 3, 1);

    assert_q(
        &oracle,
        "d2q:half",
        Rational::from_f64_limited(0.5, 100).unwrap(),
    );
    assert_q(
        &oracle,
        "d2q:negative_quarter",
        Rational::from_f64_limited(-0.25, 100).unwrap(),
    );
    assert_q(
        &oracle,
        "d2q:ntsc_decimal",
        Rational::from_f64_limited(29.97, 3000).unwrap(),
    );
    assert_q(
        &oracle,
        "d2q:ntsc_limited",
        Rational::from_f64_limited(29.97, 1001).unwrap(),
    );
    assert_q(
        &oracle,
        "d2q:nan",
        Rational::from_f64_limited(f64::NAN, 100).unwrap(),
    );
    assert_q(
        &oracle,
        "d2q:positive_infinity",
        Rational::from_f64_limited(f64::INFINITY, 100).unwrap(),
    );
    assert_q(
        &oracle,
        "d2q:negative_infinity",
        Rational::from_f64_limited(f64::NEG_INFINITY, 100).unwrap(),
    );

    assert_i32(
        &oracle,
        "nearer:first",
        nearer_value(
            Rational::new(25, 1)
                .unwrap()
                .nearer_to(Rational::new(24, 1).unwrap(), Rational::new(30, 1).unwrap())
                .unwrap(),
        ),
    );
    assert_i32(
        &oracle,
        "nearer:second",
        nearer_value(
            Rational::new(25, 1)
                .unwrap()
                .nearer_to(Rational::new(20, 1).unwrap(), Rational::new(26, 1).unwrap())
                .unwrap(),
        ),
    );
    assert_i32(
        &oracle,
        "nearer:tie",
        nearer_value(
            Rational::new(27, 1)
                .unwrap()
                .nearer_to(Rational::new(24, 1).unwrap(), Rational::new(30, 1).unwrap())
                .unwrap(),
        ),
    );

    assert_i32(
        &oracle,
        "find:twenty_six",
        Rational::new(26, 1)
            .unwrap()
            .find_nearest_index(&[
                Rational::new(24, 1).unwrap(),
                Rational::new(30, 1).unwrap(),
                Rational::new(25, 1).unwrap(),
            ])
            .unwrap()
            .unwrap() as i32,
    );
    assert_i32(
        &oracle,
        "find:tie_keeps_first",
        Rational::new(27, 1)
            .unwrap()
            .find_nearest_index(&[Rational::new(24, 1).unwrap(), Rational::new(30, 1).unwrap()])
            .unwrap()
            .unwrap() as i32,
    );

    for (name, q) in [
        ("q2int:zero_zero", Rational::from_raw(0, 0)),
        ("q2int:zero", Rational::from_raw(0, 1)),
        ("q2int:positive_infinity", Rational::from_raw(1, 0)),
        ("q2int:negative_infinity", Rational::from_raw(-1, 0)),
        ("q2int:one", Rational::new(1, 1).unwrap()),
        ("q2int:half", Rational::new(1, 2).unwrap()),
        ("q2int:negative_half", Rational::new(-1, 2).unwrap()),
        ("q2int:third", Rational::new(1, 3).unwrap()),
        ("q2int:negative_denominator", Rational::from_raw(1, -2)),
    ] {
        assert_u32_hex(&oracle, name, q.to_int_float_bits().unwrap());
    }

    assert_q(
        &oracle,
        "gcd:one_thirtieth_one_sixtieth",
        Rational::new(1, 30)
            .unwrap()
            .gcd_with_limit(Rational::new(1, 60).unwrap(), 100, Rational::ONE)
            .unwrap(),
    );
    assert_q(
        &oracle,
        "gcd:two_thirds_four_ninths",
        Rational::new(2, 3)
            .unwrap()
            .gcd_with_limit(Rational::new(4, 9).unwrap(), 10, Rational::ONE)
            .unwrap(),
    );
    assert_q(
        &oracle,
        "gcd:raw_unreduced",
        Rational::from_raw(2, 4)
            .gcd_with_limit(Rational::from_raw(4, 6), 100, Rational::ONE)
            .unwrap(),
    );
    let default = Rational::from_raw(30000, 1001);
    assert_q(
        &oracle,
        "gcd:strict_limit_default",
        Rational::new(1, 30)
            .unwrap()
            .gcd_with_limit(Rational::new(1, 60).unwrap(), 60, default)
            .unwrap(),
    );

    let half = Rational::new(1, 2).unwrap();
    let third = Rational::new(1, 3).unwrap();
    assert_q(&oracle, "arith:add", half.checked_add(third).unwrap());
    assert_q(&oracle, "arith:sub", half.checked_sub(third).unwrap());
    assert_q(
        &oracle,
        "arith:mul",
        Rational::new(2, 3)
            .unwrap()
            .checked_mul(Rational::new(9, 4).unwrap())
            .unwrap(),
    );
    assert_q(
        &oracle,
        "arith:div",
        Rational::new(3, 2)
            .unwrap()
            .checked_div(Rational::new(9, 4).unwrap())
            .unwrap(),
    );
    assert_q(
        &oracle,
        "arith:inv",
        Rational::new(2, 3).unwrap().reciprocal().unwrap(),
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

fn assert_u32_hex(rows: &BTreeMap<String, Vec<String>>, name: &str, expected: u32) {
    assert_eq!(
        row_fields(rows, name),
        &[format!("{expected:08x}")],
        "{name}"
    );
}

fn assert_double_bits(rows: &BTreeMap<String, Vec<String>>, name: &str, expected: Rational) {
    assert_eq!(
        row_fields(rows, name),
        &[format!("{:016x}", expected.to_f64().to_bits())],
        "{name}"
    );
}

fn assert_q(rows: &BTreeMap<String, Vec<String>>, name: &str, expected: Rational) {
    assert_eq!(
        row_fields(rows, name),
        &[expected.num().to_string(), expected.den().to_string()],
        "{name}"
    );
}

fn assert_reduce(rows: &BTreeMap<String, Vec<String>>, name: &str, num: i64, den: i64, max: i32) {
    let (rational, exact) = Rational::reduce_i64(num, den, max).unwrap();
    assert_eq!(
        row_fields(rows, name),
        &[
            i32::from(exact).to_string(),
            rational.num().to_string(),
            rational.den().to_string()
        ],
        "{name}"
    );
}

fn row_fields<'a>(rows: &'a BTreeMap<String, Vec<String>>, name: &str) -> &'a [String] {
    rows.get(name)
        .unwrap_or_else(|| panic!("missing oracle row `{name}`"))
}

fn cmp_value(ordering: Option<Ordering>) -> i32 {
    match ordering {
        Some(Ordering::Greater) => 1,
        Some(Ordering::Equal) => 0,
        Some(Ordering::Less) => -1,
        None => i32::MIN,
    }
}

fn nearer_value(ordering: Ordering) -> i32 {
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
            .expect("run WSL libavutil rational oracle")
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
            .expect("run libavutil rational oracle")
    };

    assert!(
        output.status.success(),
        "libavutil rational oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <inttypes.h>
#include <limits.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <libavutil/rational.h>

#define QR(num, den) ((AVRational){ (num), (den) })
#define QA(num, den) { (num), (den) }

static void print_i32(const char *name, int value) {
    printf("%s|%d\n", name, value);
}

static void print_u32(const char *name, uint32_t value) {
    printf("%s|%08" PRIx32 "\n", name, value);
}

static void print_q(const char *name, AVRational value) {
    printf("%s|%d|%d\n", name, value.num, value.den);
}

static void print_q2d(const char *name, AVRational value) {
    union {
        double d;
        uint64_t u;
    } bits;
    bits.d = av_q2d(value);
    printf("%s|%016" PRIx64 "\n", name, bits.u);
}

static void print_reduce(const char *name, int64_t num, int64_t den, int64_t max) {
    int dst_num = 0;
    int dst_den = 0;
    int exact = av_reduce(&dst_num, &dst_den, num, den, max);
    printf("%s|%d|%d|%d\n", name, exact, dst_num, dst_den);
}

int main(void) {
    print_i32("cmp:half_vs_two_thirds", av_cmp_q(QR(1, 2), QR(2, 3)));
    print_i32("cmp:positive_infinity_vs_finite", av_cmp_q(QR(1, 0), QR(24, 1)));
    print_i32("cmp:nan_vs_finite", av_cmp_q(QR(0, 0), QR(1, 1)));

    print_q2d("q2d:half", QR(1, 2));
    print_q2d("q2d:negative_quarter", QR(-1, 4));
    print_q2d("q2d:positive_infinity", QR(1, 0));

    print_reduce("reduce:ntsc", 30000, -1001, INT_MAX);
    print_reduce("reduce:zero", 0, 4000, 100);
    print_reduce("reduce:positive_infinity", 42, 0, 100);
    print_reduce("reduce:nan", 0, 0, 100);
    print_reduce("reduce:ntsc_limited", 1001, 30000, 100);
    print_reduce("reduce:two_thirds_max_one", 2, 3, 1);
    print_reduce("reduce:negative_two_thirds_max_one", -2, 3, 1);

    print_q("d2q:half", av_d2q(0.5, 100));
    print_q("d2q:negative_quarter", av_d2q(-0.25, 100));
    print_q("d2q:ntsc_decimal", av_d2q(29.97, 3000));
    print_q("d2q:ntsc_limited", av_d2q(29.97, 1001));
    print_q("d2q:nan", av_d2q(NAN, 100));
    print_q("d2q:positive_infinity", av_d2q(INFINITY, 100));
    print_q("d2q:negative_infinity", av_d2q(-INFINITY, 100));

    print_i32("nearer:first", av_nearer_q(QR(25, 1), QR(24, 1), QR(30, 1)));
    print_i32("nearer:second", av_nearer_q(QR(25, 1), QR(20, 1), QR(26, 1)));
    print_i32("nearer:tie", av_nearer_q(QR(27, 1), QR(24, 1), QR(30, 1)));

    {
        const AVRational values[] = { QA(24, 1), QA(30, 1), QA(25, 1), QA(0, 0) };
        print_i32("find:twenty_six", av_find_nearest_q_idx(QR(26, 1), values));
    }
    {
        const AVRational values[] = { QA(24, 1), QA(30, 1), QA(0, 0) };
        print_i32("find:tie_keeps_first", av_find_nearest_q_idx(QR(27, 1), values));
    }

    print_u32("q2int:zero_zero", av_q2intfloat(QR(0, 0)));
    print_u32("q2int:zero", av_q2intfloat(QR(0, 1)));
    print_u32("q2int:positive_infinity", av_q2intfloat(QR(1, 0)));
    print_u32("q2int:negative_infinity", av_q2intfloat(QR(-1, 0)));
    print_u32("q2int:one", av_q2intfloat(QR(1, 1)));
    print_u32("q2int:half", av_q2intfloat(QR(1, 2)));
    print_u32("q2int:negative_half", av_q2intfloat(QR(-1, 2)));
    print_u32("q2int:third", av_q2intfloat(QR(1, 3)));
    print_u32("q2int:negative_denominator", av_q2intfloat(QR(1, -2)));

    print_q("gcd:one_thirtieth_one_sixtieth", av_gcd_q(QR(1, 30), QR(1, 60), 100, QR(1, 1)));
    print_q("gcd:two_thirds_four_ninths", av_gcd_q(QR(2, 3), QR(4, 9), 10, QR(1, 1)));
    print_q("gcd:raw_unreduced", av_gcd_q(QR(2, 4), QR(4, 6), 100, QR(1, 1)));
    print_q("gcd:strict_limit_default", av_gcd_q(QR(1, 30), QR(1, 60), 60, QR(30000, 1001)));

    print_q("arith:add", av_add_q(QR(1, 2), QR(1, 3)));
    print_q("arith:sub", av_sub_q(QR(1, 2), QR(1, 3)));
    print_q("arith:mul", av_mul_q(QR(2, 3), QR(9, 4)));
    print_q("arith:div", av_div_q(QR(3, 2), QR(9, 4)));
    print_q("arith:inv", av_inv_q(QR(2, 3)));

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
