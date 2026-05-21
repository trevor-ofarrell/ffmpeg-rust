use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::BitReader;

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 source/build cache plus libavcodec oracle under third_party/ffmpeg-oracle/wsl"]
fn libavcodec_get_bits_helpers_match_bitreader_model() {
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

    let work_dir = repo_root.join("target/oracle/avutil-bitreader");
    fs::create_dir_all(&work_dir).expect("create avutil-bitreader oracle work dir");
    let source = work_dir.join("bitreader_oracle.c");
    let executable = work_dir.join("bitreader_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-bitreader oracle C source");

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
    rows.insert("read:basic".to_string(), basic_fields());
    rows.insert("read:align".to_string(), align_fields());
    rows.insert("read:long".to_string(), long_fields());
    rows.insert("read:signed".to_string(), signed_fields());
    rows.insert("read:ue-golomb".to_string(), ue_golomb_fields());
    rows.insert("read:se-golomb".to_string(), se_golomb_fields());
    rows
}

fn basic_fields() -> Vec<String> {
    let data = [0b1011_0010, 0b0110_0001];
    let mut reader = BitReader::new(&data);
    let mut fields = vec![
        "0".to_string(),
        reader.bit_position().to_string(),
        reader.bits_remaining().to_string(),
        reader.peek_bits(3).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.read_bits(3).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.peek_bits(5).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.read_bits(5).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.read_bits(4).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.read_bits(4).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.bits_remaining().to_string(),
    ];
    fields.push((if reader.is_eof() { 1 } else { 0 }).to_string());
    fields
}

fn align_fields() -> Vec<String> {
    let data = [0xff, 0x80, 0xab];
    let mut reader = BitReader::new(&data);
    reader.skip_bits(3).unwrap();
    let mut fields = vec![
        reader.bit_position().to_string(),
        reader.bits_remaining().to_string(),
    ];
    reader.byte_align().unwrap();
    fields.extend([
        reader.bit_position().to_string(),
        reader.bits_remaining().to_string(),
        (if reader.read_bit().unwrap() { 1 } else { 0 }).to_string(),
        reader.bit_position().to_string(),
    ]);
    fields
}

fn long_fields() -> Vec<String> {
    let data = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
    let mut first = BitReader::new(&data);
    let mut second = BitReader::new(&data);
    let mut fields = vec![
        first.read_bits(32).unwrap().to_string(),
        first.bit_position().to_string(),
        first.read_bits(32).unwrap().to_string(),
        first.bit_position().to_string(),
        second.read_bits(64).unwrap().to_string(),
        second.bit_position().to_string(),
    ];
    second.rewind();
    second.skip_bits(12).unwrap();
    fields.extend([
        second.bit_position().to_string(),
        second.read_bits(20).unwrap().to_string(),
        second.bit_position().to_string(),
    ]);
    fields
}

fn signed_fields() -> Vec<String> {
    let data = [0b1110_0101, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let mut reader = BitReader::new(&data);
    vec![
        reader.read_signed_bits(4).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.read_signed_bits(4).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.read_signed_bits(1).unwrap().to_string(),
        reader.bit_position().to_string(),
        reader.read_signed_bits(63).unwrap().to_string(),
        reader.bit_position().to_string(),
    ]
}

fn ue_golomb_fields() -> Vec<String> {
    let bytes = bytes_from_bits("101001100100001010011000111");
    let mut reader = BitReader::new(&bytes);
    let mut fields = Vec::new();
    for _ in 0..7 {
        fields.push(reader.read_ue_golomb().unwrap().to_string());
        fields.push(reader.bit_position().to_string());
    }
    fields
}

fn se_golomb_fields() -> Vec<String> {
    let bytes = bytes_from_bits("101001100100001010011000111");
    let mut reader = BitReader::new(&bytes);
    let mut fields = Vec::new();
    for _ in 0..7 {
        fields.push(reader.read_se_golomb().unwrap().to_string());
        fields.push(reader.bit_position().to_string());
    }
    fields
}

fn bytes_from_bits(bits: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut current = 0_u8;
    for (index, bit) in bits.bytes().enumerate() {
        current <<= 1;
        if bit == b'1' {
            current |= 1;
        }
        if index % 8 == 7 {
            bytes.push(current);
            current = 0;
        }
    }
    let remainder = bits.len() % 8;
    if remainder != 0 {
        current <<= 8 - remainder;
        bytes.push(current);
    }
    bytes
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
             test -f \"$source_root/libavcodec/get_bits.h\" && \
             test -f \"$source_root/libavcodec/golomb.h\" && \
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
            .expect("run WSL libavcodec get_bits oracle")
    } else {
        let source_root = env::var("FFMPEGRUST_FFMPEG_SOURCE")
            .unwrap_or_else(|_| "third_party/ffmpeg-oracle/src".to_string());
        let build_root = env::var("FFMPEGRUST_FFMPEG_BUILD")
            .unwrap_or_else(|_| "third_party/ffmpeg-oracle/build-src".to_string());
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "test -f {}/libavcodec/get_bits.h && \
                 test -f {}/libavcodec/golomb.h && \
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
            .expect("run libavcodec get_bits oracle")
    };

    assert!(
        output.status.success(),
        "libavcodec get_bits oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <stdint.h>
#include <stdio.h>
#include "libavutil/internal.h"
#include "libavcodec/get_bits.h"
#include "libavcodec/golomb.h"

static void print_basic(void) {
    const uint8_t data[2] = { 0xb2, 0x61 };
    GetBitContext gb;
    unsigned value;
    int ret = init_get_bits8(&gb, data, sizeof(data));
    printf("read:basic|%d|%d|%d", ret, get_bits_count(&gb), get_bits_left(&gb));
    value = show_bits(&gb, 3);
    printf("|%u|%d", value, get_bits_count(&gb));
    value = get_bits(&gb, 3);
    printf("|%u|%d", value, get_bits_count(&gb));
    value = show_bits(&gb, 5);
    printf("|%u|%d", value, get_bits_count(&gb));
    value = get_bits(&gb, 5);
    printf("|%u|%d", value, get_bits_count(&gb));
    value = get_bits(&gb, 4);
    printf("|%u|%d", value, get_bits_count(&gb));
    value = get_bits(&gb, 4);
    printf("|%u|%d|%d|%d\n", value, get_bits_count(&gb), get_bits_left(&gb), get_bits_left(&gb) == 0);
}

static void print_align(void) {
    const uint8_t data[3] = { 0xff, 0x80, 0xab };
    GetBitContext gb;
    unsigned value;
    init_get_bits8(&gb, data, sizeof(data));
    skip_bits(&gb, 3);
    printf("read:align|%d|%d", get_bits_count(&gb), get_bits_left(&gb));
    align_get_bits(&gb);
    printf("|%d|%d", get_bits_count(&gb), get_bits_left(&gb));
    value = get_bits1(&gb);
    printf("|%u|%d\n", value, get_bits_count(&gb));
}

static void print_long(void) {
    const uint8_t data[8] = { 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef };
    GetBitContext first;
    GetBitContext second;
    unsigned value32;
    uint64_t value64;
    init_get_bits8(&first, data, sizeof(data));
    init_get_bits8(&second, data, sizeof(data));
    printf("read:long");
    value32 = get_bits_long(&first, 32);
    printf("|%u|%d", value32, get_bits_count(&first));
    value32 = get_bits_long(&first, 32);
    printf("|%u|%d", value32, get_bits_count(&first));
    value64 = get_bits64(&second, 64);
    printf("|%llu|%d", (unsigned long long)value64, get_bits_count(&second));
    init_get_bits8(&second, data, sizeof(data));
    skip_bits_long(&second, 12);
    printf("|%d", get_bits_count(&second));
    value32 = get_bits_long(&second, 20);
    printf("|%u|%d\n", value32, get_bits_count(&second));
}

static void print_signed(void) {
    const uint8_t data[9] = { 0xe5, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 };
    GetBitContext gb;
    int value32;
    int64_t value64;
    init_get_bits8(&gb, data, sizeof(data));
    printf("read:signed");
    value32 = get_sbits(&gb, 4);
    printf("|%d|%d", value32, get_bits_count(&gb));
    value32 = get_sbits(&gb, 4);
    printf("|%d|%d", value32, get_bits_count(&gb));
    value32 = get_sbits(&gb, 1);
    printf("|%d|%d", value32, get_bits_count(&gb));
    value64 = get_sbits64(&gb, 63);
    printf("|%lld|%d\n", (long long)value64, get_bits_count(&gb));
}

static void print_golomb(void) {
    const uint8_t data[4] = { 0xa6, 0x42, 0x98, 0xe0 };
    GetBitContext ue;
    GetBitContext se;
    init_get_bits8(&ue, data, sizeof(data));
    init_get_bits8(&se, data, sizeof(data));
    printf("read:ue-golomb");
    for (int i = 0; i < 7; i++) {
        int value = get_ue_golomb(&ue);
        printf("|%d|%d", value, get_bits_count(&ue));
    }
    printf("\n");
    printf("read:se-golomb");
    for (int i = 0; i < 7; i++) {
        int value = get_se_golomb(&se);
        printf("|%d|%d", value, get_bits_count(&se));
    }
    printf("\n");
}

int main(void) {
    print_basic();
    print_align();
    print_long();
    print_signed();
    print_golomb();
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
