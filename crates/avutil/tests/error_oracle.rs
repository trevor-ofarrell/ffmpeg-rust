use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{av_make_error_string, AvErrorCode, AV_ERROR_MAX_STRING_SIZE};

#[derive(Clone, Copy)]
struct ErrorCase {
    c_name: &'static str,
    rust_code: AvErrorCode,
}

const ERROR_CASES: &[ErrorCase] = &[
    ErrorCase {
        c_name: "AVERROR_BSF_NOT_FOUND",
        rust_code: AvErrorCode::BSF_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_BUG",
        rust_code: AvErrorCode::BUG,
    },
    ErrorCase {
        c_name: "AVERROR_BUFFER_TOO_SMALL",
        rust_code: AvErrorCode::BUFFER_TOO_SMALL,
    },
    ErrorCase {
        c_name: "AVERROR_DECODER_NOT_FOUND",
        rust_code: AvErrorCode::DECODER_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_DEMUXER_NOT_FOUND",
        rust_code: AvErrorCode::DEMUXER_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_ENCODER_NOT_FOUND",
        rust_code: AvErrorCode::ENCODER_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_EOF",
        rust_code: AvErrorCode::EOF,
    },
    ErrorCase {
        c_name: "AVERROR_EXIT",
        rust_code: AvErrorCode::EXIT,
    },
    ErrorCase {
        c_name: "AVERROR_EXTERNAL",
        rust_code: AvErrorCode::EXTERNAL,
    },
    ErrorCase {
        c_name: "AVERROR_FILTER_NOT_FOUND",
        rust_code: AvErrorCode::FILTER_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_INPUT_CHANGED",
        rust_code: AvErrorCode::INPUT_CHANGED,
    },
    ErrorCase {
        c_name: "AVERROR_INVALIDDATA",
        rust_code: AvErrorCode::INVALIDDATA,
    },
    ErrorCase {
        c_name: "AVERROR_MUXER_NOT_FOUND",
        rust_code: AvErrorCode::MUXER_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_OPTION_NOT_FOUND",
        rust_code: AvErrorCode::OPTION_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_OUTPUT_CHANGED",
        rust_code: AvErrorCode::OUTPUT_CHANGED,
    },
    ErrorCase {
        c_name: "AVERROR_PATCHWELCOME",
        rust_code: AvErrorCode::PATCHWELCOME,
    },
    ErrorCase {
        c_name: "AVERROR_PROTOCOL_NOT_FOUND",
        rust_code: AvErrorCode::PROTOCOL_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_STREAM_NOT_FOUND",
        rust_code: AvErrorCode::STREAM_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_BUG2",
        rust_code: AvErrorCode::BUG2,
    },
    ErrorCase {
        c_name: "AVERROR_UNKNOWN",
        rust_code: AvErrorCode::UNKNOWN,
    },
    ErrorCase {
        c_name: "AVERROR_EXPERIMENTAL",
        rust_code: AvErrorCode::EXPERIMENTAL,
    },
    ErrorCase {
        c_name: "AVERROR_INPUT_AND_OUTPUT_CHANGED",
        rust_code: AvErrorCode::INPUT_AND_OUTPUT_CHANGED,
    },
    ErrorCase {
        c_name: "AVERROR_HTTP_BAD_REQUEST",
        rust_code: AvErrorCode::HTTP_BAD_REQUEST,
    },
    ErrorCase {
        c_name: "AVERROR_HTTP_UNAUTHORIZED",
        rust_code: AvErrorCode::HTTP_UNAUTHORIZED,
    },
    ErrorCase {
        c_name: "AVERROR_HTTP_FORBIDDEN",
        rust_code: AvErrorCode::HTTP_FORBIDDEN,
    },
    ErrorCase {
        c_name: "AVERROR_HTTP_NOT_FOUND",
        rust_code: AvErrorCode::HTTP_NOT_FOUND,
    },
    ErrorCase {
        c_name: "AVERROR_HTTP_TOO_MANY_REQUESTS",
        rust_code: AvErrorCode::HTTP_TOO_MANY_REQUESTS,
    },
    ErrorCase {
        c_name: "AVERROR_HTTP_OTHER_4XX",
        rust_code: AvErrorCode::HTTP_OTHER_4XX,
    },
    ErrorCase {
        c_name: "AVERROR_HTTP_SERVER_ERROR",
        rust_code: AvErrorCode::HTTP_SERVER_ERROR,
    },
    ErrorCase {
        c_name: "UNKNOWN_NEG_123456",
        rust_code: AvErrorCode::from_raw(-123_456),
    },
];

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_error_strings_match_current_error_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/error.h").is_file(),
        "missing pinned FFmpeg libavutil headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-error");
    fs::create_dir_all(&work_dir).expect("create avutil-error oracle work dir");
    let source = work_dir.join("error_oracle.c");
    let executable = work_dir.join("error_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-error oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);

    let max = oracle
        .get("AV_ERROR_MAX_STRING_SIZE")
        .expect("oracle did not print AV_ERROR_MAX_STRING_SIZE");
    assert_eq!(max.code, AV_ERROR_MAX_STRING_SIZE as i32);
    assert_eq!(max.message, AV_ERROR_MAX_STRING_SIZE.to_string());

    for case in ERROR_CASES {
        let actual = oracle
            .get(case.c_name)
            .unwrap_or_else(|| panic!("missing oracle row for {}", case.c_name));
        assert_eq!(
            actual.code,
            case.rust_code.raw(),
            "raw code mismatch for {}",
            case.c_name
        );
        assert_eq!(
            actual.message,
            av_make_error_string(case.rust_code.raw()),
            "av_strerror string mismatch for {}",
            case.c_name
        );
    }
}

#[derive(Debug)]
struct OracleRow {
    code: i32,
    message: String,
}

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, OracleRow> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let mut parts = line.splitn(3, '|');
        let name = parts.next().expect("row name").to_string();
        let code = parts
            .next()
            .unwrap_or_else(|| panic!("missing code in oracle row `{line}`"))
            .parse::<i32>()
            .unwrap_or_else(|err| panic!("invalid code in oracle row `{line}`: {err}"));
        let message = parts
            .next()
            .unwrap_or_else(|| panic!("missing message in oracle row `{line}`"))
            .to_string();
        assert!(
            rows.insert(name, OracleRow { code, message }).is_none(),
            "duplicate oracle row `{line}`"
        );
    }
    rows
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
            .expect("run WSL libavutil error oracle")
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
            .expect("run libavutil error oracle")
    };

    assert!(
        output.status.success(),
        "libavutil error oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <stdio.h>
#include <libavutil/error.h>

struct error_case {
    const char *name;
    int code;
};

#define CASE(name) { #name, name }

int main(void) {
    const struct error_case cases[] = {
        CASE(AVERROR_BSF_NOT_FOUND),
        CASE(AVERROR_BUG),
        CASE(AVERROR_BUFFER_TOO_SMALL),
        CASE(AVERROR_DECODER_NOT_FOUND),
        CASE(AVERROR_DEMUXER_NOT_FOUND),
        CASE(AVERROR_ENCODER_NOT_FOUND),
        CASE(AVERROR_EOF),
        CASE(AVERROR_EXIT),
        CASE(AVERROR_EXTERNAL),
        CASE(AVERROR_FILTER_NOT_FOUND),
        CASE(AVERROR_INPUT_CHANGED),
        CASE(AVERROR_INVALIDDATA),
        CASE(AVERROR_MUXER_NOT_FOUND),
        CASE(AVERROR_OPTION_NOT_FOUND),
        CASE(AVERROR_OUTPUT_CHANGED),
        CASE(AVERROR_PATCHWELCOME),
        CASE(AVERROR_PROTOCOL_NOT_FOUND),
        CASE(AVERROR_STREAM_NOT_FOUND),
        CASE(AVERROR_BUG2),
        CASE(AVERROR_UNKNOWN),
        CASE(AVERROR_EXPERIMENTAL),
        { "AVERROR_INPUT_AND_OUTPUT_CHANGED", AVERROR_INPUT_CHANGED | AVERROR_OUTPUT_CHANGED },
        CASE(AVERROR_HTTP_BAD_REQUEST),
        CASE(AVERROR_HTTP_UNAUTHORIZED),
        CASE(AVERROR_HTTP_FORBIDDEN),
        CASE(AVERROR_HTTP_NOT_FOUND),
        CASE(AVERROR_HTTP_TOO_MANY_REQUESTS),
        CASE(AVERROR_HTTP_OTHER_4XX),
        CASE(AVERROR_HTTP_SERVER_ERROR),
        { "UNKNOWN_NEG_123456", -123456 },
    };
    char buffer[AV_ERROR_MAX_STRING_SIZE];
    printf("AV_ERROR_MAX_STRING_SIZE|%d|%d\n", AV_ERROR_MAX_STRING_SIZE, AV_ERROR_MAX_STRING_SIZE);
    for (unsigned i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        av_strerror(cases[i].code, buffer, sizeof(buffer));
        printf("%s|%d|%s\n", cases[i].name, cases[i].code, buffer);
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
