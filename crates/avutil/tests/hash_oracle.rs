use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    adler32, crc32_ieee, digest_to_hex, md5, sha1, sha224, sha256, sha384, sha512, sha512_224,
    sha512_256,
};

const ALGORITHMS: &[(&str, &str, usize)] = &[
    ("ADLER32", "adler32", 4),
    ("CRC32", "CRC32", 4),
    ("MD5", "MD5", 16),
    ("SHA160", "SHA160", 20),
    ("SHA224", "SHA224", 28),
    ("SHA256", "SHA256", 32),
    ("SHA384", "SHA384", 48),
    ("SHA512", "SHA512", 64),
    ("SHA512/224", "SHA512/224", 28),
    ("SHA512/256", "SHA512/256", 32),
];

const CASES: &[(&str, &[u8])] = &[
    ("empty", b""),
    ("abc", b"abc"),
    ("quick", b"The quick brown fox jumps over the lazy dog"),
    (
        "binary",
        &[
            0x00, 0x01, 0x02, 0x7f, 0x80, 0xfe, 0xff, b'f', b'f', b'm', b'p', b'e', b'g', 0x10,
            0x20, 0x40, 0x55, 0xaa,
        ],
    ),
];

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_hash_helpers_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/hash.h").is_file(),
        "missing pinned FFmpeg libavutil hash headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-hash");
    fs::create_dir_all(&work_dir).expect("create avutil-hash oracle work dir");
    let source = work_dir.join("hash_oracle.c");
    let executable = work_dir.join("hash_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-hash oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);

    let names = row_fields(&oracle, "names");
    for (_, oracle_name, _) in ALGORITHMS {
        assert!(
            names.iter().any(|name| name == oracle_name),
            "libavutil av_hash_names did not report {oracle_name}; names were {names:?}"
        );
    }

    for (algorithm, _, expected_size) in ALGORITHMS {
        for (case_name, data) in CASES {
            assert_eq!(
                row_fields(&oracle, &format!("hash:{algorithm}:{case_name}")),
                &[
                    expected_size.to_string(),
                    expected_digest_hex(algorithm, data)
                ],
                "{algorithm} {case_name}"
            );
        }
    }
}

fn expected_digest_hex(algorithm: &str, data: &[u8]) -> String {
    match algorithm {
        "ADLER32" => format!("{:08x}", adler32(data)),
        "CRC32" => format!("{:08x}", crc32_ieee(data)),
        "MD5" => digest_to_hex(&md5(data)),
        "SHA160" => digest_to_hex(&sha1(data)),
        "SHA224" => digest_to_hex(&sha224(data)),
        "SHA256" => digest_to_hex(&sha256(data)),
        "SHA384" => digest_to_hex(&sha384(data)),
        "SHA512" => digest_to_hex(&sha512(data)),
        "SHA512/224" => digest_to_hex(&sha512_224(data)),
        "SHA512/256" => digest_to_hex(&sha512_256(data)),
        other => panic!("unexpected hash algorithm `{other}`"),
    }
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
            .expect("run WSL libavutil hash oracle")
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
            .expect("run libavutil hash oracle")
    };

    assert!(
        output.status.success(),
        "libavutil hash oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
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
#include <libavutil/hash.h>

static void print_names(void) {
    printf("names");
    for (int i = 0;; i++) {
        const char *name = av_hash_names(i);
        if (!name)
            break;
        printf("|%s", name);
    }
    printf("\n");
}

static void print_hash(const char *algorithm, const char *case_name,
                       const uint8_t *data, size_t len) {
    struct AVHashContext *ctx = NULL;
    uint8_t hex[2 * AV_HASH_MAX_SIZE + 1];
    int ret = av_hash_alloc(&ctx, algorithm);
    if (ret < 0 || !ctx) {
        fprintf(stderr, "av_hash_alloc failed for %s: %d\n", algorithm, ret);
        exit(1);
    }

    av_hash_init(ctx);
    av_hash_update(ctx, data, 0);

    size_t first = len < 5 ? len : 5;
    size_t second = len > first ? (len - first) / 2 : 0;
    av_hash_update(ctx, data, first);
    av_hash_update(ctx, data + first, second);
    av_hash_update(ctx, data + first + second, len - first - second);

    memset(hex, 0, sizeof(hex));
    av_hash_final_hex(ctx, hex, sizeof(hex));
    printf("hash:%s:%s|%d|%s\n", algorithm, case_name, av_hash_get_size(ctx), hex);
    av_hash_freep(&ctx);
}

int main(void) {
    static const char abc[] = "abc";
    static const char quick[] = "The quick brown fox jumps over the lazy dog";
    static const uint8_t binary[] = {
        0x00, 0x01, 0x02, 0x7f, 0x80, 0xfe, 0xff, 'f', 'f', 'm', 'p', 'e',
        'g', 0x10, 0x20, 0x40, 0x55, 0xaa,
    };
    static const char *algorithms[] = {
        "ADLER32", "CRC32", "MD5", "SHA160", "SHA224", "SHA256", "SHA384", "SHA512",
        "SHA512/224", "SHA512/256",
    };

    print_names();
    for (size_t i = 0; i < sizeof(algorithms) / sizeof(algorithms[0]); i++) {
        print_hash(algorithms[i], "empty", (const uint8_t *)"", 0);
        print_hash(algorithms[i], "abc", (const uint8_t *)abc, 3);
        print_hash(algorithms[i], "quick", (const uint8_t *)quick, sizeof(quick) - 1);
        print_hash(algorithms[i], "binary", binary, sizeof(binary));
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
