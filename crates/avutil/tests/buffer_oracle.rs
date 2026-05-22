use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use avutil::BufferRef;

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_buffer_refs_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/buffer.h").is_file(),
        "missing pinned FFmpeg libavutil buffer headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-buffer");
    fs::create_dir_all(&work_dir).expect("create avutil-buffer oracle work dir");
    let source = work_dir.join("buffer_oracle.c");
    let executable = work_dir.join("buffer_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-buffer oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
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
        "buffer:alloc".to_string(),
        vec!["4".to_string(), "1".to_string(), "1".to_string()],
    );

    let allocz = BufferRef::zeroed(4).unwrap();
    rows.insert("buffer:allocz".to_string(), buffer_fields(&allocz));

    let ref_src = BufferRef::from_vec(vec![1, 2, 3]);
    let ref_dst = ref_src.clone();
    rows.insert("buffer:ref-src".to_string(), buffer_fields(&ref_src));
    rows.insert("buffer:ref-dst".to_string(), buffer_fields(&ref_dst));
    rows.insert(
        "buffer:ref-shares".to_string(),
        vec![bool_field(ref_src.shares_storage(&ref_dst))],
    );

    let mut unique = BufferRef::from_vec(vec![4, 5, 6]);
    let unique_before = unique.as_ptr();
    unique.make_mut();
    rows.insert(
        "buffer:make-writable-unique-ret".to_string(),
        vec![
            "0".to_string(),
            bool_field(std::ptr::eq(unique_before, unique.as_ptr())),
        ],
    );
    rows.insert(
        "buffer:make-writable-unique".to_string(),
        buffer_fields(&unique),
    );

    let shared_src = BufferRef::from_vec(vec![9, 8, 7]);
    let mut shared_dst = shared_src.clone();
    shared_dst.make_mut();
    rows.insert(
        "buffer:make-writable-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:make-writable-shared-src".to_string(),
        buffer_fields(&shared_src),
    );
    rows.insert(
        "buffer:make-writable-shared-dst".to_string(),
        buffer_fields(&shared_dst),
    );
    rows.insert(
        "buffer:make-writable-shared-shares".to_string(),
        vec![bool_field(shared_src.shares_storage(&shared_dst))],
    );

    let released = Arc::new(Mutex::new(Vec::<usize>::new()));
    let capture = Arc::clone(&released);
    let mut readonly = BufferRef::from_external_slice_with_opaque_readonly(
        vec![5, 6, 7].into(),
        77usize,
        move |opaque| {
            capture.lock().unwrap().push(opaque);
        },
    );
    rows.insert(
        "buffer:readonly".to_string(),
        buffer_fields_with_opaque(&readonly),
    );
    readonly.make_mut();
    let released_values = released.lock().unwrap();
    rows.insert(
        "buffer:readonly-make-writable-ret".to_string(),
        vec![
            "0".to_string(),
            released_values.len().to_string(),
            released_values[0].to_string(),
        ],
    );
    drop(released_values);
    rows.insert(
        "buffer:readonly-after".to_string(),
        buffer_fields_with_opaque(&readonly),
    );

    let mut grow = BufferRef::from_vec(vec![1, 2, 3]);
    grow.resize(5).unwrap();
    rows.insert("buffer:realloc-grow-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "buffer:realloc-grow".to_string(),
        buffer_prefix_fields(&grow, 3),
    );
    grow.resize(2).unwrap();
    rows.insert(
        "buffer:realloc-shrink-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert("buffer:realloc-shrink".to_string(), buffer_fields(&grow));

    let realloc_src = BufferRef::from_vec(vec![7, 7, 7]);
    let mut realloc_dst = realloc_src.clone();
    realloc_dst.resize(5).unwrap();
    rows.insert(
        "buffer:realloc-shared-ret".to_string(),
        vec!["0".to_string()],
    );
    rows.insert(
        "buffer:realloc-shared-src".to_string(),
        buffer_fields(&realloc_src),
    );
    rows.insert(
        "buffer:realloc-shared-dst".to_string(),
        buffer_prefix_fields(&realloc_dst, 3),
    );
    rows.insert(
        "buffer:realloc-shared-shares".to_string(),
        vec![bool_field(realloc_src.shares_storage(&realloc_dst))],
    );

    let replace_src = BufferRef::from_vec(vec![3, 4, 5]);
    let replace_dst = replace_src.clone();
    rows.insert("buffer:replace-ret".to_string(), vec!["0".to_string()]);
    rows.insert(
        "buffer:replace-src".to_string(),
        buffer_fields(&replace_src),
    );
    rows.insert(
        "buffer:replace-dst".to_string(),
        buffer_fields(&replace_dst),
    );
    rows.insert(
        "buffer:replace-shares".to_string(),
        vec![bool_field(replace_src.shares_storage(&replace_dst))],
    );

    rows.insert("buffer:unref-null".to_string(), vec!["1".to_string()]);

    rows
}

fn buffer_fields(buffer: &BufferRef) -> Vec<String> {
    vec![
        buffer.len().to_string(),
        hex(buffer.as_slice()),
        buffer.strong_count().to_string(),
        bool_field(buffer.is_writable()),
    ]
}

fn buffer_prefix_fields(buffer: &BufferRef, prefix_len: usize) -> Vec<String> {
    vec![
        buffer.len().to_string(),
        hex(&buffer.as_slice()[..prefix_len]),
        buffer.strong_count().to_string(),
        bool_field(buffer.is_writable()),
    ]
}

fn buffer_fields_with_opaque(buffer: &BufferRef) -> Vec<String> {
    let mut fields = buffer_fields(buffer);
    fields.push(
        buffer
            .opaque_ref::<usize>()
            .copied()
            .unwrap_or_default()
            .to_string(),
    );
    fields
}

fn hex(data: &[u8]) -> String {
    let mut output = String::with_capacity(data.len() * 2);
    for byte in data {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn bool_field(value: bool) -> String {
    if value { "1" } else { "0" }.to_string()
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
            .expect("run WSL libavutil buffer oracle")
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
            .expect("run libavutil buffer oracle")
    };

    assert!(
        output.status.success(),
        "libavutil buffer oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
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
#include <libavutil/buffer.h>
#include <libavutil/mem.h>

static int release_count = 0;
static uintptr_t last_opaque = 0;

static void fail_if(int condition, const char *message) {
    if (condition) {
        fprintf(stderr, "%s\n", message);
        exit(1);
    }
}

static void test_free(void *opaque, uint8_t *data) {
    release_count++;
    last_opaque = (uintptr_t)opaque;
    av_free(data);
}

static void fill_bytes(AVBufferRef *buf, const uint8_t *data, size_t size) {
    fail_if(!buf || buf->size < size, "short buffer in fill_bytes");
    for (size_t i = 0; i < size; i++)
        buf->data[i] = data[i];
}

static void print_hex(const uint8_t *data, size_t size) {
    for (size_t i = 0; i < size; i++)
        printf("%02x", data[i]);
}

static void print_status(const char *label, const AVBufferRef *buf) {
    printf("%s|%zu|%d|%d\n",
           label,
           buf ? buf->size : 0,
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0);
}

static void print_buffer(const char *label, const AVBufferRef *buf) {
    printf("%s|%zu|", label, buf ? buf->size : 0);
    if (buf)
        print_hex(buf->data, buf->size);
    printf("|%d|%d\n",
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0);
}

static void print_buffer_prefix(const char *label, const AVBufferRef *buf, size_t prefix) {
    printf("%s|%zu|", label, buf ? buf->size : 0);
    if (buf)
        print_hex(buf->data, prefix);
    printf("|%d|%d\n",
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0);
}

static void print_buffer_opaque(const char *label, const AVBufferRef *buf) {
    printf("%s|%zu|", label, buf ? buf->size : 0);
    if (buf)
        print_hex(buf->data, buf->size);
    printf("|%d|%d|%llu\n",
           buf ? av_buffer_get_ref_count(buf) : 0,
           buf ? av_buffer_is_writable(buf) : 0,
           (unsigned long long)(uintptr_t)(buf ? av_buffer_get_opaque(buf) : NULL));
}

int main(void) {
    AVBufferRef *buf = av_buffer_alloc(4);
    fail_if(!buf, "av_buffer_alloc failed");
    print_status("buffer:alloc", buf);
    av_buffer_unref(&buf);

    buf = av_buffer_allocz(4);
    fail_if(!buf, "av_buffer_allocz failed");
    print_buffer("buffer:allocz", buf);
    av_buffer_unref(&buf);

    static const uint8_t ref_bytes[] = { 1, 2, 3 };
    AVBufferRef *ref_src = av_buffer_allocz(3);
    fail_if(!ref_src, "av_buffer_allocz ref_src failed");
    fill_bytes(ref_src, ref_bytes, sizeof(ref_bytes));
    AVBufferRef *ref_dst = av_buffer_ref(ref_src);
    fail_if(!ref_dst, "av_buffer_ref failed");
    print_buffer("buffer:ref-src", ref_src);
    print_buffer("buffer:ref-dst", ref_dst);
    printf("buffer:ref-shares|%d\n", ref_src->data == ref_dst->data);
    av_buffer_unref(&ref_dst);
    av_buffer_unref(&ref_src);

    static const uint8_t unique_bytes[] = { 4, 5, 6 };
    AVBufferRef *unique = av_buffer_allocz(3);
    fail_if(!unique, "av_buffer_allocz unique failed");
    fill_bytes(unique, unique_bytes, sizeof(unique_bytes));
    uint8_t *unique_before = unique->data;
    int ret = av_buffer_make_writable(&unique);
    printf("buffer:make-writable-unique-ret|%d|%d\n",
           ret, unique_before == unique->data);
    print_buffer("buffer:make-writable-unique", unique);
    av_buffer_unref(&unique);

    static const uint8_t shared_bytes[] = { 9, 8, 7 };
    AVBufferRef *shared_src = av_buffer_allocz(3);
    fail_if(!shared_src, "av_buffer_allocz shared_src failed");
    fill_bytes(shared_src, shared_bytes, sizeof(shared_bytes));
    AVBufferRef *shared_dst = av_buffer_ref(shared_src);
    fail_if(!shared_dst, "av_buffer_ref shared failed");
    ret = av_buffer_make_writable(&shared_dst);
    printf("buffer:make-writable-shared-ret|%d\n", ret);
    print_buffer("buffer:make-writable-shared-src", shared_src);
    print_buffer("buffer:make-writable-shared-dst", shared_dst);
    printf("buffer:make-writable-shared-shares|%d\n",
           shared_src->data == shared_dst->data);
    av_buffer_unref(&shared_dst);
    av_buffer_unref(&shared_src);

    uint8_t *readonly_data = av_malloc(3);
    fail_if(!readonly_data, "av_malloc readonly failed");
    readonly_data[0] = 5;
    readonly_data[1] = 6;
    readonly_data[2] = 7;
    AVBufferRef *readonly = av_buffer_create(readonly_data, 3, test_free,
                                             (void *)(uintptr_t)77,
                                             AV_BUFFER_FLAG_READONLY);
    fail_if(!readonly, "av_buffer_create readonly failed");
    print_buffer_opaque("buffer:readonly", readonly);
    ret = av_buffer_make_writable(&readonly);
    printf("buffer:readonly-make-writable-ret|%d|%d|%llu\n",
           ret, release_count, (unsigned long long)last_opaque);
    print_buffer_opaque("buffer:readonly-after", readonly);
    av_buffer_unref(&readonly);

    static const uint8_t grow_bytes[] = { 1, 2, 3 };
    AVBufferRef *grow = av_buffer_allocz(3);
    fail_if(!grow, "av_buffer_allocz grow failed");
    fill_bytes(grow, grow_bytes, sizeof(grow_bytes));
    ret = av_buffer_realloc(&grow, 5);
    printf("buffer:realloc-grow-ret|%d\n", ret);
    print_buffer_prefix("buffer:realloc-grow", grow, 3);
    ret = av_buffer_realloc(&grow, 2);
    printf("buffer:realloc-shrink-ret|%d\n", ret);
    print_buffer("buffer:realloc-shrink", grow);
    av_buffer_unref(&grow);

    static const uint8_t realloc_shared_bytes[] = { 7, 7, 7 };
    AVBufferRef *realloc_src = av_buffer_allocz(3);
    fail_if(!realloc_src, "av_buffer_allocz realloc_src failed");
    fill_bytes(realloc_src, realloc_shared_bytes, sizeof(realloc_shared_bytes));
    AVBufferRef *realloc_dst = av_buffer_ref(realloc_src);
    fail_if(!realloc_dst, "av_buffer_ref realloc failed");
    ret = av_buffer_realloc(&realloc_dst, 5);
    printf("buffer:realloc-shared-ret|%d\n", ret);
    print_buffer("buffer:realloc-shared-src", realloc_src);
    print_buffer_prefix("buffer:realloc-shared-dst", realloc_dst, 3);
    printf("buffer:realloc-shared-shares|%d\n",
           realloc_src->data == realloc_dst->data);
    av_buffer_unref(&realloc_dst);
    av_buffer_unref(&realloc_src);

    static const uint8_t replace_src_bytes[] = { 3, 4, 5 };
    AVBufferRef *replace_src = av_buffer_allocz(3);
    AVBufferRef *replace_dst = av_buffer_allocz(2);
    fail_if(!replace_src || !replace_dst, "av_buffer_allocz replace failed");
    fill_bytes(replace_src, replace_src_bytes, sizeof(replace_src_bytes));
    ret = av_buffer_replace(&replace_dst, replace_src);
    printf("buffer:replace-ret|%d\n", ret);
    print_buffer("buffer:replace-src", replace_src);
    print_buffer("buffer:replace-dst", replace_dst);
    printf("buffer:replace-shares|%d\n", replace_src->data == replace_dst->data);
    av_buffer_unref(&replace_dst);
    av_buffer_unref(&replace_src);

    buf = av_buffer_allocz(1);
    fail_if(!buf, "av_buffer_allocz unref failed");
    av_buffer_unref(&buf);
    printf("buffer:unref-null|%d\n", buf == NULL);

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
