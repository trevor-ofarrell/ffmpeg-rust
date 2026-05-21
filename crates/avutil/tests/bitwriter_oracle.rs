use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::BitWriter;

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 source/build cache plus libavcodec oracle under third_party/ffmpeg-oracle/wsl"]
fn libavcodec_put_bits_helpers_match_bitwriter_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavcodec = oracle_root.join("wsl/lib/libavcodec.a");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/intreadwrite.h").is_file(),
        "missing pinned FFmpeg libavutil headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavcodec.is_file(),
        "missing pinned FFmpeg libavcodec static library `{}`",
        libavcodec.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-bitwriter");
    fs::create_dir_all(&work_dir).expect("create avutil-bitwriter oracle work dir");
    let source = work_dir.join("bitwriter_oracle.c");
    let executable = work_dir.join("bitwriter_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-bitwriter oracle C source");

    let stdout =
        compile_and_run_oracle(&include_dir, &libavcodec, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);
    let expected = expected_rows();

    assert_eq!(
        oracle.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "oracle row set diverged"
    );

    for (name, expected_fields) in expected {
        assert_eq!(
            row_fields(&oracle, &name),
            expected_fields.as_slice(),
            "{name} diverged"
        );
    }
}

fn expected_rows() -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    rows.insert(
        "write:basic".to_string(),
        writer_fields(|writer| {
            writer.write_bits(0b101, 3).unwrap();
            writer.write_bit(true);
            writer.write_bits(0b0010, 4).unwrap();
            writer.write_bits(0x61, 8).unwrap();
        }),
    );
    rows.insert(
        "write:align-zero".to_string(),
        writer_fields(|writer| {
            writer.write_bits(0b101, 3).unwrap();
            writer.byte_align_zero();
        }),
    );
    rows.insert(
        "write:long32-pair".to_string(),
        writer_fields(|writer| {
            writer.write_bits(0x0123_4567, 32).unwrap();
            writer.write_bits(0x89ab_cdef, 32).unwrap();
        }),
    );
    rows.insert(
        "write:long64".to_string(),
        writer_fields(|writer| {
            writer.write_bits(0x0123_4567_89ab_cdef, 64).unwrap();
        }),
    );
    rows.insert(
        "write:wide63".to_string(),
        writer_fields(|writer| {
            writer.write_bits(0x7fff_ffff_ffff_ffff, 63).unwrap();
        }),
    );
    rows.insert(
        "write:signed".to_string(),
        writer_fields(|writer| {
            writer.write_signed_bits(-2, 4).unwrap();
            writer.write_signed_bits(5, 4).unwrap();
            writer.write_signed_bits(-1, 1).unwrap();
            writer.write_signed_bits(0, 63).unwrap();
        }),
    );
    rows.insert(
        "write:ue-golomb".to_string(),
        writer_fields(|writer| {
            for value in 0..=6 {
                writer.write_ue_golomb(value).unwrap();
            }
        }),
    );
    rows.insert(
        "write:se-golomb".to_string(),
        writer_fields(|writer| {
            for value in [0, 1, -1, 2, -2, 3, -3] {
                writer.write_se_golomb(value).unwrap();
            }
        }),
    );
    rows.insert(
        "write:ue-golomb-long".to_string(),
        writer_fields(|writer| {
            for value in [300, 65_535, 1_000_000] {
                writer.write_ue_golomb(value).unwrap();
            }
        }),
    );
    rows
}

fn writer_fields(write: impl FnOnce(&mut BitWriter)) -> Vec<String> {
    let mut writer = BitWriter::new();
    write(&mut writer);
    vec![
        writer.bits_written().to_string(),
        writer.as_slice().len().to_string(),
        hex_bytes(writer.as_slice()),
    ]
}

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.split('|');
        let name = parts
            .next()
            .unwrap_or_else(|| panic!("missing oracle row name in `{line}`"))
            .to_string();
        let fields = parts.map(ToOwned::to_owned).collect::<Vec<_>>();
        assert!(
            rows.insert(name.clone(), fields).is_none(),
            "duplicate oracle row `{name}`"
        );
    }
    rows
}

fn row_fields<'a>(rows: &'a BTreeMap<String, Vec<String>>, name: &str) -> &'a [String] {
    rows.get(name)
        .unwrap_or_else(|| panic!("missing oracle row `{name}`"))
        .as_slice()
}

fn compile_and_run_oracle(
    include_dir: &Path,
    libavcodec: &Path,
    libavutil: &Path,
    source: &Path,
    executable: &Path,
) -> String {
    let output = if cfg!(windows) {
        let script = format!(
            "source_root=\"${{FFMPEGRUST_FFMPEG_SOURCE:-$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1/src}}\"; \
             build_root=\"${{FFMPEGRUST_FFMPEG_BUILD:-$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1/build}}\"; \
             test -f \"$source_root/libavcodec/put_bits.h\" && \
             test -f \"$source_root/libavcodec/put_golomb.h\" && \
             test -f \"$build_root/config.h\" && \
             gcc -I \"$build_root\" -I \"$source_root\" -I {} {} {} {} -lm -pthread -ldl -o {} && {}",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(libavcodec)),
            shell_quote(&to_wsl_path(libavutil)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable))
        );
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run WSL libavcodec put_bits oracle")
    } else {
        let source_root = env::var("FFMPEGRUST_FFMPEG_SOURCE")
            .unwrap_or_else(|_| "third_party/ffmpeg-oracle/src".to_string());
        let build_root = env::var("FFMPEGRUST_FFMPEG_BUILD")
            .unwrap_or_else(|_| "third_party/ffmpeg-oracle/build-src".to_string());
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "test -f {}/libavcodec/put_bits.h && \
                 test -f {}/libavcodec/put_golomb.h && \
                 test -f {}/config.h && \
                 gcc -I {} -I {} -I {} {} {} {} -lm -pthread -ldl -o {} && {}",
                shell_quote(&source_root),
                shell_quote(&source_root),
                shell_quote(&build_root),
                shell_quote(&build_root),
                shell_quote(&source_root),
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&libavcodec.display().to_string()),
                shell_quote(&libavutil.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string())
            ))
            .output()
            .expect("run libavcodec put_bits oracle")
    };

    assert!(
        output.status.success(),
        "libavcodec put_bits oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "libavcodec/put_bits.h"
#include "libavcodec/put_golomb.h"

static void print_bytes(const uint8_t *buffer, int count) {
    for (int i = 0; i < count; i++) {
        printf("%02x", buffer[i]);
    }
}

static void print_result(const char *name, PutBitContext *pb, uint8_t *buffer) {
    int bits = put_bits_count(pb);
    int bytes = put_bytes_count(pb, 1);
    flush_put_bits(pb);
    printf("%s|%d|%d|", name, bits, bytes);
    print_bytes(buffer, bytes);
    printf("\n");
}

static void init_case(PutBitContext *pb, uint8_t *buffer, int size) {
    memset(buffer, 0xcc, size);
    init_put_bits(pb, buffer, size);
}

static void print_basic(void) {
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    put_bits(&pb, 3, 0x5);
    put_bits(&pb, 1, 0x1);
    put_bits(&pb, 4, 0x2);
    put_bits(&pb, 8, 0x61);
    print_result("write:basic", &pb, buffer);
}

static void print_align_zero(void) {
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    put_bits(&pb, 3, 0x5);
    align_put_bits(&pb);
    print_result("write:align-zero", &pb, buffer);
}

static void print_long32_pair(void) {
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    put_bits32(&pb, 0x01234567U);
    put_bits32(&pb, 0x89abcdefU);
    print_result("write:long32-pair", &pb, buffer);
}

static void print_long64(void) {
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    put_bits64(&pb, 64, 0x0123456789abcdefULL);
    print_result("write:long64", &pb, buffer);
}

static void print_wide63(void) {
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    put_bits63(&pb, 63, 0x7fffffffffffffffULL);
    print_result("write:wide63", &pb, buffer);
}

static void print_signed(void) {
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    put_sbits(&pb, 4, -2);
    put_sbits(&pb, 4, 5);
    put_sbits(&pb, 1, -1);
    put_sbits63(&pb, 63, 0);
    print_result("write:signed", &pb, buffer);
}

static void print_ue_golomb(void) {
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    for (int i = 0; i <= 6; i++) {
        set_ue_golomb(&pb, i);
    }
    print_result("write:ue-golomb", &pb, buffer);
}

static void print_se_golomb(void) {
    const int values[7] = { 0, 1, -1, 2, -2, 3, -3 };
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    for (int i = 0; i < 7; i++) {
        set_se_golomb(&pb, values[i]);
    }
    print_result("write:se-golomb", &pb, buffer);
}

static void print_ue_golomb_long(void) {
    const uint32_t values[3] = { 300U, 65535U, 1000000U };
    uint8_t buffer[32];
    PutBitContext pb;
    init_case(&pb, buffer, sizeof(buffer));
    for (int i = 0; i < 3; i++) {
        set_ue_golomb_long(&pb, values[i]);
    }
    print_result("write:ue-golomb-long", &pb, buffer);
}

int main(void) {
    print_basic();
    print_align_zero();
    print_long32_pair();
    print_long64();
    print_wide63();
    print_signed();
    print_ue_golomb();
    print_se_golomb();
    print_ue_golomb_long();
    return 0;
}
"#
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
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
