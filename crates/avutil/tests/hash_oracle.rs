use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    adler32, crc32_ieee, digest_to_base64, digest_to_hex, md5, murmur3, ripemd128, ripemd160,
    ripemd256, ripemd320, sha1, sha224, sha256, sha384, sha512, sha512_224, sha512_256,
    HashAlgorithm,
};

const ALGORITHMS: &[HashAlgorithm] = &HashAlgorithm::ALL;

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
    let expected_names = ALGORITHMS
        .iter()
        .map(|algorithm| algorithm.name().to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, expected_names.as_slice());

    for algorithm in ALGORITHMS {
        let algorithm_name = algorithm.name();
        assert_eq!(
            row_fields(&oracle, &format!("meta:{algorithm_name}")),
            &[algorithm_name.to_string(), algorithm.size().to_string()]
        );

        for (case_name, data) in CASES {
            assert_eq!(
                row_fields(&oracle, &format!("hash:{algorithm_name}:{case_name}")),
                &[
                    algorithm.size().to_string(),
                    expected_digest_hex(*algorithm, data)
                ],
                "{algorithm_name} {case_name}"
            );
        }

        let digest = expected_digest(*algorithm, b"abc");
        assert_eq!(
            row_fields(&oracle, &format!("final:{algorithm_name}:abc")),
            &[bytes_to_hex(&digest)],
            "{algorithm_name} final bytes"
        );
        for size in bin_sizes(algorithm.size()) {
            assert_eq!(
                row_fields(&oracle, &format!("bin:{algorithm_name}:abc:{size}")),
                &[bytes_to_hex(&expected_bin_buffer(&digest, size))],
                "{algorithm_name} bin size {size}"
            );
        }

        for size in hex_buffer_sizes(algorithm.size()) {
            assert_eq!(
                row_fields(&oracle, &format!("hexbuf:{algorithm_name}:abc:{size}")),
                &[bytes_to_hex(&expected_c_string_buffer(
                    digest_to_hex(&digest).as_bytes(),
                    size
                ))],
                "{algorithm_name} hex buffer size {size}"
            );
        }

        for size in base64_buffer_sizes(algorithm.size()) {
            assert_eq!(
                row_fields(&oracle, &format!("b64buf:{algorithm_name}:abc:{size}")),
                &[bytes_to_hex(&expected_base64_c_string_buffer(
                    digest_to_base64(&digest).as_bytes(),
                    size
                ))],
                "{algorithm_name} base64 buffer size {size}"
            );
        }
    }
}

fn expected_digest_hex(algorithm: HashAlgorithm, data: &[u8]) -> String {
    digest_to_hex(&expected_digest(algorithm, data))
}

fn expected_digest(algorithm: HashAlgorithm, data: &[u8]) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Adler32 => adler32(data).to_be_bytes().to_vec(),
        HashAlgorithm::Crc32 => crc32_ieee(data).to_be_bytes().to_vec(),
        HashAlgorithm::Murmur3 => murmur3(data).to_vec(),
        HashAlgorithm::Md5 => md5(data).to_vec(),
        HashAlgorithm::Ripemd128 => ripemd128(data).to_vec(),
        HashAlgorithm::Ripemd160 => ripemd160(data).to_vec(),
        HashAlgorithm::Ripemd256 => ripemd256(data).to_vec(),
        HashAlgorithm::Ripemd320 => ripemd320(data).to_vec(),
        HashAlgorithm::Sha160 => sha1(data).to_vec(),
        HashAlgorithm::Sha224 => sha224(data).to_vec(),
        HashAlgorithm::Sha256 => sha256(data).to_vec(),
        HashAlgorithm::Sha384 => sha384(data).to_vec(),
        HashAlgorithm::Sha512 => sha512(data).to_vec(),
        HashAlgorithm::Sha512Trunc224 => sha512_224(data).to_vec(),
        HashAlgorithm::Sha512Trunc256 => sha512_256(data).to_vec(),
    }
}

fn bin_sizes(hash_size: usize) -> [usize; 4] {
    [
        1,
        hash_size.saturating_sub(1).max(1),
        hash_size,
        hash_size + 2,
    ]
}

fn hex_buffer_sizes(hash_size: usize) -> [usize; 4] {
    let exact = (hash_size * 2) + 1;
    [1, 6, exact, exact + 3]
}

fn base64_buffer_sizes(hash_size: usize) -> [usize; 4] {
    let exact = hash_size.div_ceil(3) * 4 + 1;
    [1, 5, exact, exact + 2]
}

fn expected_bin_buffer(digest: &[u8], size: usize) -> Vec<u8> {
    let mut output = vec![0; size];
    let copy_len = digest.len().min(size);
    output[..copy_len].copy_from_slice(&digest[..copy_len]);
    output
}

fn expected_c_string_buffer(content: &[u8], size: usize) -> Vec<u8> {
    let mut output = vec![0xcc; size];
    if size <= 1 {
        return output;
    }
    let copy_len = content.len().min(size - 1);
    output[..copy_len].copy_from_slice(&content[..copy_len]);
    output[copy_len] = 0;
    output
}

fn expected_base64_c_string_buffer(content: &[u8], size: usize) -> Vec<u8> {
    let mut output = vec![0xcc; size];
    if size == 0 {
        return output;
    }
    if size == 1 {
        output[0] = 0;
        return output;
    }
    let copy_len = content.len().min(size - 1);
    output[..copy_len].copy_from_slice(&content[..copy_len]);
    output[copy_len] = 0;
    output
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    digest_to_hex(bytes)
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

static struct AVHashContext *new_hash(const char *algorithm, const uint8_t *data, size_t len) {
    struct AVHashContext *ctx = NULL;
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
    return ctx;
}

static void print_bytes(const uint8_t *data, size_t len) {
    for (size_t i = 0; i < len; i++)
        printf("%02x", data[i]);
}

static void print_meta(const char *algorithm) {
    struct AVHashContext *ctx = new_hash(algorithm, (const uint8_t *)"", 0);
    printf("meta:%s|%s|%d\n", algorithm, av_hash_get_name(ctx), av_hash_get_size(ctx));
    av_hash_freep(&ctx);
}

static void print_hash(const char *algorithm, const char *case_name,
                       const uint8_t *data, size_t len) {
    struct AVHashContext *ctx = new_hash(algorithm, data, len);
    uint8_t hex[2 * AV_HASH_MAX_SIZE + 1];
    memset(hex, 0, sizeof(hex));
    av_hash_final_hex(ctx, hex, sizeof(hex));
    printf("hash:%s:%s|%d|%s\n", algorithm, case_name, av_hash_get_size(ctx), hex);
    av_hash_freep(&ctx);
}

static void print_final(const char *algorithm, const char *case_name,
                        const uint8_t *data, size_t len) {
    struct AVHashContext *ctx = new_hash(algorithm, data, len);
    uint8_t buffer[AV_HASH_MAX_SIZE];
    int size = av_hash_get_size(ctx);
    memset(buffer, 0xcc, sizeof(buffer));
    av_hash_final(ctx, buffer);
    printf("final:%s:%s|", algorithm, case_name);
    print_bytes(buffer, size);
    printf("\n");
    av_hash_freep(&ctx);
}

static void print_bin(const char *algorithm, const char *case_name,
                      const uint8_t *data, size_t len, int size) {
    struct AVHashContext *ctx = new_hash(algorithm, data, len);
    uint8_t buffer[AV_HASH_MAX_SIZE + 8];
    memset(buffer, 0xcc, sizeof(buffer));
    av_hash_final_bin(ctx, buffer, size);
    printf("bin:%s:%s:%d|", algorithm, case_name, size);
    print_bytes(buffer, size);
    printf("\n");
    av_hash_freep(&ctx);
}

static void print_hexbuf(const char *algorithm, const char *case_name,
                         const uint8_t *data, size_t len, int size) {
    struct AVHashContext *ctx = new_hash(algorithm, data, len);
    uint8_t buffer[2 * AV_HASH_MAX_SIZE + 8];
    memset(buffer, 0xcc, sizeof(buffer));
    av_hash_final_hex(ctx, buffer, size);
    printf("hexbuf:%s:%s:%d|", algorithm, case_name, size);
    print_bytes(buffer, size);
    printf("\n");
    av_hash_freep(&ctx);
}

static void print_b64buf(const char *algorithm, const char *case_name,
                         const uint8_t *data, size_t len, int size) {
    struct AVHashContext *ctx = new_hash(algorithm, data, len);
    uint8_t buffer[AV_HASH_MAX_SIZE * 2];
    memset(buffer, 0xcc, sizeof(buffer));
    av_hash_final_b64(ctx, buffer, size);
    printf("b64buf:%s:%s:%d|", algorithm, case_name, size);
    print_bytes(buffer, size);
    printf("\n");
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
        "MD5", "murmur3", "RIPEMD128", "RIPEMD160", "RIPEMD256", "RIPEMD320",
        "SHA160", "SHA224", "SHA256", "SHA512/224", "SHA512/256", "SHA384",
        "SHA512", "CRC32", "adler32",
    };

    print_names();
    for (size_t i = 0; i < sizeof(algorithms) / sizeof(algorithms[0]); i++) {
        const char *algorithm = algorithms[i];
        struct AVHashContext *ctx = new_hash(algorithm, (const uint8_t *)"", 0);
        int hash_size = av_hash_get_size(ctx);
        int hex_size = 2 * hash_size + 1;
        int b64_size = ((hash_size + 2) / 3) * 4 + 1;
        av_hash_freep(&ctx);

        print_meta(algorithm);
        print_hash(algorithms[i], "empty", (const uint8_t *)"", 0);
        print_hash(algorithms[i], "abc", (const uint8_t *)abc, 3);
        print_hash(algorithms[i], "quick", (const uint8_t *)quick, sizeof(quick) - 1);
        print_hash(algorithms[i], "binary", binary, sizeof(binary));
        print_final(algorithm, "abc", (const uint8_t *)abc, 3);
        print_bin(algorithm, "abc", (const uint8_t *)abc, 3, 1);
        print_bin(algorithm, "abc", (const uint8_t *)abc, 3, hash_size - 1);
        print_bin(algorithm, "abc", (const uint8_t *)abc, 3, hash_size);
        print_bin(algorithm, "abc", (const uint8_t *)abc, 3, hash_size + 2);
        print_hexbuf(algorithm, "abc", (const uint8_t *)abc, 3, 1);
        print_hexbuf(algorithm, "abc", (const uint8_t *)abc, 3, 6);
        print_hexbuf(algorithm, "abc", (const uint8_t *)abc, 3, hex_size);
        print_hexbuf(algorithm, "abc", (const uint8_t *)abc, 3, hex_size + 3);
        print_b64buf(algorithm, "abc", (const uint8_t *)abc, 3, 1);
        print_b64buf(algorithm, "abc", (const uint8_t *)abc, 3, 5);
        print_b64buf(algorithm, "abc", (const uint8_t *)abc, 3, b64_size);
        print_b64buf(algorithm, "abc", (const uint8_t *)abc, 3, b64_size + 2);
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
