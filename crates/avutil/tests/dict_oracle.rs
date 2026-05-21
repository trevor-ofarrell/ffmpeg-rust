use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{AvErrorCode, Dictionary, MatchMode, SetMode};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_dictionary_helpers_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/dict.h").is_file(),
        "missing pinned FFmpeg libavutil dictionary headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-dict");
    fs::create_dir_all(&work_dir).expect("create avutil-dict oracle work dir");
    let source = work_dir.join("dict_oracle.c");
    let executable = work_dir.join("dict_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-dict oracle C source");

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
    insert_row(&mut rows, "flags", ["1", "2", "16", "32", "64", "128"]);
    insert_row(&mut rows, "dict:null", ["0"]);

    let mut default = Dictionary::new();
    let default_ret = [
        ret(default.set("Title", "First")),
        ret(default.set("title", "Second")),
    ];
    insert_row(&mut rows, "ret:set-default", default_ret);
    rows.insert("dict:set-default".to_string(), dict_fields(&default));

    let mut case_sensitive = Dictionary::new();
    let case_sensitive_ret = [
        ret(case_sensitive.set_with_mode(
            "TITLE",
            "upper",
            MatchMode::CaseSensitive,
            SetMode::Overwrite,
        )),
        ret(case_sensitive.set_with_mode(
            "title",
            "lower",
            MatchMode::CaseSensitive,
            SetMode::Overwrite,
        )),
    ];
    insert_row(&mut rows, "ret:set-case-sensitive", case_sensitive_ret);
    rows.insert(
        "dict:set-case-sensitive".to_string(),
        dict_fields(&case_sensitive),
    );

    let mut keep = Dictionary::new();
    let keep_ret = [
        ret(keep.set("artist", "first")),
        ret(keep.set_with_mode(
            "ARTIST",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::KeepExisting,
        )),
    ];
    insert_row(&mut rows, "ret:dont-overwrite", keep_ret);
    rows.insert("dict:dont-overwrite".to_string(), dict_fields(&keep));

    let mut append = Dictionary::new();
    let append_ret = [
        ret(append.set("comment", "part1")),
        ret(append.set_with_mode(
            "COMMENT",
            "+part2",
            MatchMode::CaseInsensitive,
            SetMode::Append,
        )),
    ];
    insert_row(&mut rows, "ret:append", append_ret);
    rows.insert("dict:append".to_string(), dict_fields(&append));

    let mut dedup = Dictionary::new();
    let dedup_ret = [
        ret(dedup.set_with_mode(
            "artist",
            "first",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )),
        ret(dedup.set_with_mode(
            "ARTIST",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )),
        ret(dedup.set_with_mode(
            "Artist",
            "first",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultipleDedup,
        )),
        ret(dedup.set_with_mode(
            "Artist",
            "first",
            MatchMode::CaseSensitive,
            SetMode::AllowMultipleDedup,
        )),
    ];
    insert_row(&mut rows, "ret:dedup", dedup_ret);
    rows.insert("dict:dedup".to_string(), dict_fields(&dedup));

    let mut get = Dictionary::new();
    get.set_with_mode(
        "artist",
        "name",
        MatchMode::CaseInsensitive,
        SetMode::AllowMultiple,
    )
    .unwrap();
    get.set_with_mode(
        "ARTIST-sort",
        "sort",
        MatchMode::CaseInsensitive,
        SetMode::AllowMultiple,
    )
    .unwrap();
    get.set_with_mode(
        "album",
        "record",
        MatchMode::CaseInsensitive,
        SetMode::AllowMultiple,
    )
    .unwrap();
    rows.insert("dict:iter".to_string(), dict_fields(&get));
    rows.insert(
        "get:artist".to_string(),
        entries_fields(get.matching_entries("artist", MatchMode::CaseInsensitive)),
    );
    rows.insert(
        "get:artist-case".to_string(),
        entries_fields(get.matching_entries("artist", MatchMode::CaseSensitive)),
    );
    rows.insert(
        "get:artist-prefix".to_string(),
        entries_fields(get.prefixed_entries("artist", MatchMode::CaseInsensitive)),
    );
    rows.insert(
        "get:artist-prefix-case".to_string(),
        entries_fields(get.prefixed_entries("artist", MatchMode::CaseSensitive)),
    );
    rows.insert(
        "get:empty-prefix".to_string(),
        entries_fields(get.prefixed_entries("", MatchMode::CaseInsensitive)),
    );

    let mut delete = Dictionary::new();
    let delete_ret = [
        ret(delete.set("language", "eng")),
        ret(delete.set("album", "record")),
        ret_removed(delete.remove("LANGUAGE", MatchMode::CaseInsensitive)),
    ];
    insert_row(&mut rows, "ret:delete-null", delete_ret);
    rows.insert("dict:delete-null".to_string(), dict_fields(&delete));

    let mut set_int = Dictionary::new();
    let set_int_ret = [
        ret(set_int.set_int("count", -42)),
        ret(set_int.set_int_with_mode(
            "COUNT",
            i64::MAX,
            MatchMode::CaseInsensitive,
            SetMode::Overwrite,
        )),
    ];
    insert_row(&mut rows, "ret:set-int", set_int_ret);
    rows.insert("dict:set-int".to_string(), dict_fields(&set_int));

    let mut special = Dictionary::new();
    special
        .set_with_mode(
            "title=name",
            "one;two\\three",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
    special
        .set_with_mode(
            "TITLE=NAME",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
    special.set("artist", "Alice").unwrap();
    insert_row(
        &mut rows,
        "ret:get-string",
        vec!["0".to_string(), special.to_pairs_string('=', ';').unwrap()],
    );

    let parsed = Dictionary::parse_pairs(
        "title\\=name=one\\;two\\\\three;TITLE\\=NAME=second;artist=Alice",
        "=",
        ";",
        MatchMode::CaseInsensitive,
        SetMode::AllowMultiple,
    )
    .unwrap();
    insert_row(&mut rows, "ret:parse", ["0"]);
    rows.insert("dict:parse".to_string(), dict_fields(&parsed));

    let mut parse_append = Dictionary::new();
    parse_append.set("artist", "old").unwrap();
    parse_append
        .parse_pairs_into(
            "ARTIST=new;comment=ok;comment=!",
            "=",
            ";",
            MatchMode::CaseInsensitive,
            SetMode::Append,
        )
        .unwrap();
    insert_row(&mut rows, "ret:parse-append", ["0"]);
    rows.insert("dict:parse-append".to_string(), dict_fields(&parse_append));

    let mut partial = Dictionary::new();
    let partial_ret = partial
        .parse_pairs_into(
            "ok=value;bad",
            "=",
            ";",
            MatchMode::CaseInsensitive,
            SetMode::Overwrite,
        )
        .unwrap_err()
        .code()
        .map_or(-1, AvErrorCode::raw);
    insert_row(&mut rows, "ret:parse-partial", [partial_ret.to_string()]);
    rows.insert("dict:parse-partial".to_string(), dict_fields(&partial));

    let mut source = Dictionary::new();
    source
        .set_with_mode(
            "artist",
            "first",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
    source
        .set_with_mode(
            "ARTIST",
            "second",
            MatchMode::CaseInsensitive,
            SetMode::AllowMultiple,
        )
        .unwrap();
    let mut copy = Dictionary::new();
    copy.set("artist", "old").unwrap();
    copy.copy_from(
        &source,
        MatchMode::CaseInsensitive,
        SetMode::AllowMultipleDedup,
    )
    .unwrap();
    insert_row(&mut rows, "ret:copy", ["0"]);
    rows.insert("dict:copy".to_string(), dict_fields(&copy));

    rows
}

fn ret<T>(result: avutil::AvResult<T>) -> String {
    result
        .err()
        .map_or(0, |err| err.code().map_or(-1, AvErrorCode::raw))
        .to_string()
}

fn ret_removed(entry: Option<avutil::DictionaryEntry>) -> String {
    if entry.is_some() {
        "0".to_string()
    } else {
        AvErrorCode::ENOENT.raw().to_string()
    }
}

fn insert_row<I, S>(rows: &mut BTreeMap<String, Vec<String>>, name: &str, fields: I)
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let old = rows.insert(
        name.to_string(),
        fields.into_iter().map(Into::into).collect(),
    );
    assert!(old.is_none(), "duplicate expected row {name}");
}

fn dict_fields(dict: &Dictionary) -> Vec<String> {
    entries_fields(dict.entries().iter())
}

fn entries_fields<'a>(entries: impl Iterator<Item = &'a avutil::DictionaryEntry>) -> Vec<String> {
    let entries = entries.collect::<Vec<_>>();
    let mut fields = Vec::with_capacity(1 + entries.len() * 2);
    fields.push(entries.len().to_string());
    for entry in entries {
        fields.push(entry.key().to_string());
        fields.push(entry.value().to_string());
    }
    fields
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
            .expect("run WSL libavutil dictionary oracle")
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
            .expect("run libavutil dictionary oracle")
    };

    assert!(
        output.status.success(),
        "libavutil dictionary oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <libavutil/dict.h>
#include <libavutil/mem.h>

static void print_dict(const char *label, const AVDictionary *dict) {
    const AVDictionaryEntry *entry = NULL;
    printf("%s|%d", label, av_dict_count(dict));
    while ((entry = av_dict_iterate(dict, entry)))
        printf("|%s|%s", entry->key, entry->value);
    printf("\n");
}

static int count_get(const AVDictionary *dict, const char *key, int flags) {
    AVDictionaryEntry *entry = NULL;
    int count = 0;
    while ((entry = av_dict_get(dict, key, entry, flags)))
        count++;
    return count;
}

static void print_get(const char *label, const AVDictionary *dict,
                      const char *key, int flags) {
    AVDictionaryEntry *entry = NULL;
    printf("%s|%d", label, count_get(dict, key, flags));
    while ((entry = av_dict_get(dict, key, entry, flags)))
        printf("|%s|%s", entry->key, entry->value);
    printf("\n");
}

static void print_ret2(const char *label, int a, int b) {
    printf("%s|%d|%d\n", label, a, b);
}

static void print_ret3(const char *label, int a, int b, int c) {
    printf("%s|%d|%d|%d\n", label, a, b, c);
}

static void print_ret4(const char *label, int a, int b, int c, int d) {
    printf("%s|%d|%d|%d|%d\n", label, a, b, c, d);
}

int main(void) {
    printf("flags|%d|%d|%d|%d|%d|%d\n",
           AV_DICT_MATCH_CASE,
           AV_DICT_IGNORE_SUFFIX,
           AV_DICT_DONT_OVERWRITE,
           AV_DICT_APPEND,
           AV_DICT_MULTIKEY,
           AV_DICT_DEDUP);
    print_dict("dict:null", NULL);

    AVDictionary *dict = NULL;
    int r1 = av_dict_set(&dict, "Title", "First", 0);
    int r2 = av_dict_set(&dict, "title", "Second", 0);
    print_ret2("ret:set-default", r1, r2);
    print_dict("dict:set-default", dict);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_set(&dict, "TITLE", "upper", AV_DICT_MATCH_CASE);
    r2 = av_dict_set(&dict, "title", "lower", AV_DICT_MATCH_CASE);
    print_ret2("ret:set-case-sensitive", r1, r2);
    print_dict("dict:set-case-sensitive", dict);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_set(&dict, "artist", "first", 0);
    r2 = av_dict_set(&dict, "ARTIST", "second", AV_DICT_DONT_OVERWRITE);
    print_ret2("ret:dont-overwrite", r1, r2);
    print_dict("dict:dont-overwrite", dict);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_set(&dict, "comment", "part1", 0);
    r2 = av_dict_set(&dict, "COMMENT", "+part2", AV_DICT_APPEND);
    print_ret2("ret:append", r1, r2);
    print_dict("dict:append", dict);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_set(&dict, "artist", "first", AV_DICT_MULTIKEY);
    r2 = av_dict_set(&dict, "ARTIST", "second", AV_DICT_MULTIKEY);
    int r3 = av_dict_set(&dict, "Artist", "first", AV_DICT_MULTIKEY | AV_DICT_DEDUP);
    int r4 = av_dict_set(&dict, "Artist", "first",
                         AV_DICT_MULTIKEY | AV_DICT_DEDUP | AV_DICT_MATCH_CASE);
    print_ret4("ret:dedup", r1, r2, r3, r4);
    print_dict("dict:dedup", dict);
    av_dict_free(&dict);

    dict = NULL;
    av_dict_set(&dict, "artist", "name", AV_DICT_MULTIKEY);
    av_dict_set(&dict, "ARTIST-sort", "sort", AV_DICT_MULTIKEY);
    av_dict_set(&dict, "album", "record", AV_DICT_MULTIKEY);
    print_dict("dict:iter", dict);
    print_get("get:artist", dict, "artist", 0);
    print_get("get:artist-case", dict, "artist", AV_DICT_MATCH_CASE);
    print_get("get:artist-prefix", dict, "artist", AV_DICT_IGNORE_SUFFIX);
    print_get("get:artist-prefix-case", dict, "artist",
              AV_DICT_MATCH_CASE | AV_DICT_IGNORE_SUFFIX);
    print_get("get:empty-prefix", dict, "", AV_DICT_IGNORE_SUFFIX);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_set(&dict, "language", "eng", 0);
    r2 = av_dict_set(&dict, "album", "record", 0);
    r3 = av_dict_set(&dict, "LANGUAGE", NULL, 0);
    print_ret3("ret:delete-null", r1, r2, r3);
    print_dict("dict:delete-null", dict);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_set_int(&dict, "count", -42, 0);
    r2 = av_dict_set_int(&dict, "COUNT", INT64_MAX, 0);
    print_ret2("ret:set-int", r1, r2);
    print_dict("dict:set-int", dict);
    av_dict_free(&dict);

    dict = NULL;
    av_dict_set(&dict, "title=name", "one;two\\three", AV_DICT_MULTIKEY);
    av_dict_set(&dict, "TITLE=NAME", "second", AV_DICT_MULTIKEY);
    av_dict_set(&dict, "artist", "Alice", 0);
    char *string = NULL;
    r1 = av_dict_get_string(dict, &string, '=', ';');
    printf("ret:get-string|%d|%s\n", r1, string ? string : "");
    av_free(string);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_parse_string(&dict,
                              "title\\=name=one\\;two\\\\three;TITLE\\=NAME=second;artist=Alice",
                              "=", ";", AV_DICT_MULTIKEY);
    printf("ret:parse|%d\n", r1);
    print_dict("dict:parse", dict);
    av_dict_free(&dict);

    dict = NULL;
    av_dict_set(&dict, "artist", "old", 0);
    r1 = av_dict_parse_string(&dict, "ARTIST=new;comment=ok;comment=!",
                              "=", ";", AV_DICT_APPEND);
    printf("ret:parse-append|%d\n", r1);
    print_dict("dict:parse-append", dict);
    av_dict_free(&dict);

    dict = NULL;
    r1 = av_dict_parse_string(&dict, "ok=value;bad", "=", ";", 0);
    printf("ret:parse-partial|%d\n", r1);
    print_dict("dict:parse-partial", dict);
    av_dict_free(&dict);

    AVDictionary *source = NULL;
    AVDictionary *copy = NULL;
    av_dict_set(&source, "artist", "first", AV_DICT_MULTIKEY);
    av_dict_set(&source, "ARTIST", "second", AV_DICT_MULTIKEY);
    av_dict_set(&copy, "artist", "old", 0);
    r1 = av_dict_copy(&copy, source, AV_DICT_MULTIKEY | AV_DICT_DEDUP);
    printf("ret:copy|%d\n", r1);
    print_dict("dict:copy", copy);
    av_dict_free(&source);
    av_dict_free(&copy);

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
