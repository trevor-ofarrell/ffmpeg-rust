use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{ByteReader, ByteWriter};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle headers under third_party/ffmpeg-oracle/wsl"]
fn libavutil_intreadwrite_helpers_match_byteio_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");

    assert!(
        include_dir.join("libavutil/intreadwrite.h").is_file(),
        "missing pinned FFmpeg libavutil intreadwrite header under `{}`",
        include_dir.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-byteio");
    fs::create_dir_all(&work_dir).expect("create avutil-byteio oracle work dir");
    let source = work_dir.join("byteio_oracle.c");
    let executable = work_dir.join("byteio_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-byteio oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &source, &executable);
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

#[derive(Debug, Clone, Copy)]
struct WriteCase {
    name: &'static str,
    u8_value: u8,
    u16_value: u16,
    u24_value: u32,
    u32_value: u32,
    u48_value: u64,
    u64_value: u64,
}

#[derive(Debug, Clone, Copy)]
struct SignedWriteCase {
    name: &'static str,
    i8_value: i8,
    i16_value: i16,
    i24_value: i32,
    i32_value: i32,
    i48_value: i64,
    i64_value: i64,
}

fn expected_rows() -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();

    for (name, data) in read_cases() {
        rows.insert(format!("read:{name}"), read_fields(&data));
    }

    for case in write_cases() {
        insert_unsigned_write_rows(&mut rows, case);
    }

    for case in signed_write_cases() {
        insert_signed_write_rows(&mut rows, case);
    }

    rows
}

fn read_cases() -> [(&'static str, [u8; 8]); 4] {
    [
        (
            "ascending",
            [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
        ),
        ("highbit", [0xff, 0xfe, 0xfd, 0x80, 0x7f, 0x00, 0x55, 0xaa]),
        ("tag", *b"RIFFWAVE"),
        ("zeros", [0x00; 8]),
    ]
}

fn write_cases() -> [WriteCase; 2] {
    [
        WriteCase {
            name: "low",
            u8_value: 0x7f,
            u16_value: 0x1234,
            u24_value: 0x00_ab_cd,
            u32_value: 0x89_ab_cd_ef,
            u48_value: 0x01_02_03_04_05_06,
            u64_value: 0x01_02_03_04_05_06_07_08,
        },
        WriteCase {
            name: "high",
            u8_value: 0xff,
            u16_value: 0xfe_dc,
            u24_value: 0xfe_dc_ba,
            u32_value: 0xfe_dc_ba_98,
            u48_value: 0xff_ff_ff_ff_ff_fe,
            u64_value: 0xff_ee_dd_cc_bb_aa_99_88,
        },
    ]
}

fn signed_write_cases() -> [SignedWriteCase; 2] {
    [
        SignedWriteCase {
            name: "negative",
            i8_value: -1,
            i16_value: -2,
            i24_value: -8_388_608,
            i32_value: -123_456,
            i48_value: -140_737_488_355_328,
            i64_value: -2,
        },
        SignedWriteCase {
            name: "positive",
            i8_value: 0x7f,
            i16_value: 0x1234,
            i24_value: 0x7f_ff_ff,
            i32_value: 0x1234_5678,
            i48_value: 0x7fff_ffff_ffff,
            i64_value: 0x0123_4567_89ab_cdef,
        },
    ]
}

fn read_fields(data: &[u8; 8]) -> Vec<String> {
    vec![
        read_value(data, |reader| reader.read_u8()),
        read_value(data, |reader| reader.read_u8()),
        read_value(data, |reader| reader.read_u16_be()),
        read_value(data, |reader| reader.read_u16_le()),
        read_value(data, |reader| reader.read_u24_be()),
        read_value(data, |reader| reader.read_u24_le()),
        read_value(data, |reader| reader.read_u32_be()),
        read_value(data, |reader| reader.read_u32_le()),
        read_value(data, |reader| reader.read_u48_be()),
        read_value(data, |reader| reader.read_u48_le()),
        read_value(data, |reader| reader.read_u64_be()),
        read_value(data, |reader| reader.read_u64_le()),
        read_value(data, |reader| reader.read_i8()),
        read_value(data, |reader| reader.read_i16_be()),
        read_value(data, |reader| reader.read_i16_le()),
        read_value(data, |reader| reader.read_i24_be()),
        read_value(data, |reader| reader.read_i24_le()),
        read_value(data, |reader| reader.read_i32_be()),
        read_value(data, |reader| reader.read_i32_le()),
        read_value(data, |reader| reader.read_i48_be()),
        read_value(data, |reader| reader.read_i48_le()),
        read_value(data, |reader| reader.read_i64_be()),
        read_value(data, |reader| reader.read_i64_le()),
    ]
}

fn read_value<T: ToString>(
    data: &[u8; 8],
    read: impl FnOnce(&mut ByteReader<'_>) -> avutil::AvResult<T>,
) -> String {
    let mut reader = ByteReader::new(data);
    read(&mut reader)
        .expect("byte reader oracle vector should be valid")
        .to_string()
}

fn insert_unsigned_write_rows(rows: &mut BTreeMap<String, Vec<String>>, case: WriteCase) {
    insert_write_row(rows, case.name, "u8_be", |writer| {
        writer.write_u8(case.u8_value)
    });
    insert_write_row(rows, case.name, "u8_le", |writer| {
        writer.write_u8(case.u8_value)
    });
    insert_write_row(rows, case.name, "u16_be", |writer| {
        writer.write_u16_be(case.u16_value)
    });
    insert_write_row(rows, case.name, "u16_le", |writer| {
        writer.write_u16_le(case.u16_value)
    });
    insert_write_row(rows, case.name, "u24_be", |writer| {
        writer.write_u24_be(case.u24_value).unwrap();
    });
    insert_write_row(rows, case.name, "u24_le", |writer| {
        writer.write_u24_le(case.u24_value).unwrap();
    });
    insert_write_row(rows, case.name, "u32_be", |writer| {
        writer.write_u32_be(case.u32_value)
    });
    insert_write_row(rows, case.name, "u32_le", |writer| {
        writer.write_u32_le(case.u32_value)
    });
    insert_write_row(rows, case.name, "u48_be", |writer| {
        writer.write_u48_be(case.u48_value).unwrap();
    });
    insert_write_row(rows, case.name, "u48_le", |writer| {
        writer.write_u48_le(case.u48_value).unwrap();
    });
    insert_write_row(rows, case.name, "u64_be", |writer| {
        writer.write_u64_be(case.u64_value)
    });
    insert_write_row(rows, case.name, "u64_le", |writer| {
        writer.write_u64_le(case.u64_value)
    });
}

fn insert_signed_write_rows(rows: &mut BTreeMap<String, Vec<String>>, case: SignedWriteCase) {
    insert_write_row(rows, case.name, "i8_be", |writer| {
        writer.write_i8(case.i8_value)
    });
    insert_write_row(rows, case.name, "i8_le", |writer| {
        writer.write_i8(case.i8_value)
    });
    insert_write_row(rows, case.name, "i16_be", |writer| {
        writer.write_i16_be(case.i16_value)
    });
    insert_write_row(rows, case.name, "i16_le", |writer| {
        writer.write_i16_le(case.i16_value)
    });
    insert_write_row(rows, case.name, "i24_be", |writer| {
        writer.write_i24_be(case.i24_value).unwrap();
    });
    insert_write_row(rows, case.name, "i24_le", |writer| {
        writer.write_i24_le(case.i24_value).unwrap();
    });
    insert_write_row(rows, case.name, "i32_be", |writer| {
        writer.write_i32_be(case.i32_value)
    });
    insert_write_row(rows, case.name, "i32_le", |writer| {
        writer.write_i32_le(case.i32_value)
    });
    insert_write_row(rows, case.name, "i48_be", |writer| {
        writer.write_i48_be(case.i48_value).unwrap();
    });
    insert_write_row(rows, case.name, "i48_le", |writer| {
        writer.write_i48_le(case.i48_value).unwrap();
    });
    insert_write_row(rows, case.name, "i64_be", |writer| {
        writer.write_i64_be(case.i64_value)
    });
    insert_write_row(rows, case.name, "i64_le", |writer| {
        writer.write_i64_le(case.i64_value)
    });
}

fn insert_write_row(
    rows: &mut BTreeMap<String, Vec<String>>,
    case_name: &str,
    field_name: &str,
    write: impl FnOnce(&mut ByteWriter),
) {
    let mut writer = ByteWriter::new();
    write(&mut writer);
    rows.insert(
        format!("write:{case_name}:{field_name}"),
        vec![hex_bytes(writer.as_slice())],
    );
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

fn compile_and_run_oracle(include_dir: &Path, source: &Path, executable: &Path) -> String {
    let output = if cfg!(windows) {
        let script = format!(
            "gcc -I {} {} -o {} && {}",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable))
        );
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run WSL libavutil intreadwrite oracle")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gcc -I {} {} -o {} && {}",
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string())
            ))
            .output()
            .expect("run libavutil intreadwrite oracle")
    };

    assert!(
        output.status.success(),
        "libavutil intreadwrite oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
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
#include <libavutil/intreadwrite.h>

struct write_case {
    const char *name;
    uint8_t u8_value;
    uint16_t u16_value;
    uint32_t u24_value;
    uint32_t u32_value;
    uint64_t u48_value;
    uint64_t u64_value;
};

struct signed_write_case {
    const char *name;
    int8_t i8_value;
    int16_t i16_value;
    int32_t i24_value;
    int32_t i32_value;
    int64_t i48_value;
    int64_t i64_value;
};

static int64_t sign_extend_u64(uint64_t value, unsigned bits) {
    uint64_t sign = 1ULL << (bits - 1);
    uint64_t mask = bits == 64 ? UINT64_MAX : ((1ULL << bits) - 1ULL);
    value &= mask;
    if (value & sign) {
        return bits == 64
            ? (int64_t)value
            : (int64_t)(value | ~mask);
    }
    return (int64_t)value;
}

static void print_signed_u64(uint64_t value) {
    if (value <= 9223372036854775807ULL) {
        printf("%llu", (unsigned long long)value);
    } else if (value == 9223372036854775808ULL) {
        printf("-9223372036854775808");
    } else {
        printf("-%llu", (unsigned long long)(~value + 1ULL));
    }
}

static void print_bytes(const uint8_t *buffer, unsigned count) {
    for (unsigned i = 0; i < count; i++) {
        printf("%02x", buffer[i]);
    }
}

static void print_write_row(const char *case_name, const char *field_name, const uint8_t *buffer, unsigned count) {
    printf("write:%s:%s|", case_name, field_name);
    print_bytes(buffer, count);
    printf("\n");
}

static void print_read_case(const char *name, const uint8_t *data) {
    printf("read:%s", name);
    printf("|%u", (unsigned)AV_RB8(data));
    printf("|%u", (unsigned)AV_RL8(data));
    printf("|%u", (unsigned)AV_RB16(data));
    printf("|%u", (unsigned)AV_RL16(data));
    printf("|%u", (unsigned)AV_RB24(data));
    printf("|%u", (unsigned)AV_RL24(data));
    printf("|%u", (unsigned)AV_RB32(data));
    printf("|%u", (unsigned)AV_RL32(data));
    printf("|%llu", (unsigned long long)AV_RB48(data));
    printf("|%llu", (unsigned long long)AV_RL48(data));
    printf("|%llu", (unsigned long long)AV_RB64(data));
    printf("|%llu", (unsigned long long)AV_RL64(data));
    printf("|%lld", (long long)sign_extend_u64(AV_RB8(data), 8));
    printf("|%lld", (long long)sign_extend_u64(AV_RB16(data), 16));
    printf("|%lld", (long long)sign_extend_u64(AV_RL16(data), 16));
    printf("|%lld", (long long)sign_extend_u64(AV_RB24(data), 24));
    printf("|%lld", (long long)sign_extend_u64(AV_RL24(data), 24));
    printf("|%lld", (long long)sign_extend_u64(AV_RB32(data), 32));
    printf("|%lld", (long long)sign_extend_u64(AV_RL32(data), 32));
    printf("|%lld", (long long)sign_extend_u64(AV_RB48(data), 48));
    printf("|%lld", (long long)sign_extend_u64(AV_RL48(data), 48));
    printf("|");
    print_signed_u64(AV_RB64(data));
    printf("|");
    print_signed_u64(AV_RL64(data));
    printf("\n");
}

static void print_unsigned_writes(const struct write_case *value) {
    uint8_t buffer[8];
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB8(buffer, value->u8_value);
    print_write_row(value->name, "u8_be", buffer, 1);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL8(buffer, value->u8_value);
    print_write_row(value->name, "u8_le", buffer, 1);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB16(buffer, value->u16_value);
    print_write_row(value->name, "u16_be", buffer, 2);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL16(buffer, value->u16_value);
    print_write_row(value->name, "u16_le", buffer, 2);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB24(buffer, value->u24_value);
    print_write_row(value->name, "u24_be", buffer, 3);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL24(buffer, value->u24_value);
    print_write_row(value->name, "u24_le", buffer, 3);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB32(buffer, value->u32_value);
    print_write_row(value->name, "u32_be", buffer, 4);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL32(buffer, value->u32_value);
    print_write_row(value->name, "u32_le", buffer, 4);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB48(buffer, value->u48_value);
    print_write_row(value->name, "u48_be", buffer, 6);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL48(buffer, value->u48_value);
    print_write_row(value->name, "u48_le", buffer, 6);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB64(buffer, value->u64_value);
    print_write_row(value->name, "u64_be", buffer, 8);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL64(buffer, value->u64_value);
    print_write_row(value->name, "u64_le", buffer, 8);
}

static void print_signed_writes(const struct signed_write_case *value) {
    uint8_t buffer[8];
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB8(buffer, (uint8_t)value->i8_value);
    print_write_row(value->name, "i8_be", buffer, 1);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL8(buffer, (uint8_t)value->i8_value);
    print_write_row(value->name, "i8_le", buffer, 1);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB16(buffer, (uint16_t)value->i16_value);
    print_write_row(value->name, "i16_be", buffer, 2);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL16(buffer, (uint16_t)value->i16_value);
    print_write_row(value->name, "i16_le", buffer, 2);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB24(buffer, ((uint32_t)value->i24_value) & 0xffffffU);
    print_write_row(value->name, "i24_be", buffer, 3);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL24(buffer, ((uint32_t)value->i24_value) & 0xffffffU);
    print_write_row(value->name, "i24_le", buffer, 3);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB32(buffer, (uint32_t)value->i32_value);
    print_write_row(value->name, "i32_be", buffer, 4);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL32(buffer, (uint32_t)value->i32_value);
    print_write_row(value->name, "i32_le", buffer, 4);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB48(buffer, ((uint64_t)value->i48_value) & 0xffffffffffffULL);
    print_write_row(value->name, "i48_be", buffer, 6);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL48(buffer, ((uint64_t)value->i48_value) & 0xffffffffffffULL);
    print_write_row(value->name, "i48_le", buffer, 6);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WB64(buffer, (uint64_t)value->i64_value);
    print_write_row(value->name, "i64_be", buffer, 8);
    memset(buffer, 0xcc, sizeof(buffer));
    AV_WL64(buffer, (uint64_t)value->i64_value);
    print_write_row(value->name, "i64_le", buffer, 8);
}

int main(void) {
    const uint8_t ascending[8] = { 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef };
    const uint8_t highbit[8] = { 0xff, 0xfe, 0xfd, 0x80, 0x7f, 0x00, 0x55, 0xaa };
    const uint8_t tag[8] = { 'R', 'I', 'F', 'F', 'W', 'A', 'V', 'E' };
    const uint8_t zeros[8] = { 0 };
    const struct write_case writes[] = {
        { "low", 0x7f, 0x1234, 0x00abcd, 0x89abcdef, 0x010203040506ULL, 0x0102030405060708ULL },
        { "high", 0xff, 0xfedc, 0xfedcba, 0xfedcba98, 0xfffffffffffeULL, 0xffeeddccbbaa9988ULL },
    };
    const struct signed_write_case signed_writes[] = {
        { "negative", -1, -2, -8388608, -123456, -140737488355328LL, -2 },
        { "positive", 0x7f, 0x1234, 0x7fffff, 0x12345678, 0x7fffffffffffLL, 0x123456789abcdefLL },
    };

    print_read_case("ascending", ascending);
    print_read_case("highbit", highbit);
    print_read_case("tag", tag);
    print_read_case("zeros", zeros);
    for (unsigned i = 0; i < sizeof(writes) / sizeof(writes[0]); i++) {
        print_unsigned_writes(&writes[i]);
    }
    for (unsigned i = 0; i < sizeof(signed_writes) / sizeof(signed_writes[0]); i++) {
        print_signed_writes(&signed_writes[i]);
    }
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
