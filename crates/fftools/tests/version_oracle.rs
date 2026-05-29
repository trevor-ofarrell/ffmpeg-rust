use fftools::{
    ffmpeg_output, ffprobe_output, option_parser::parse_log_level_directive, version_banner,
    TARGET_FFMPEG_VERSION, TARGET_LIBRARY_VERSIONS, TARGET_RELEASE_NAME,
};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_version_banner_matches_oracle_target_versions() {
    compare_version_surface("ffmpeg");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFPROBE_ORACLE, set FFMPEG_ORACLE with sibling ffprobe, or install third_party/ffmpeg-oracle/build/bin/ffprobe"]
fn ffprobe_version_banner_matches_oracle_target_versions() {
    compare_version_surface("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn double_dash_version_is_not_a_success_version_request() {
    compare_double_dash_version_rejection("ffmpeg");
    compare_double_dash_version_rejection("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn hide_banner_version_matches_plain_version_surface() {
    compare_hide_banner_version("ffmpeg");
    compare_hide_banner_version("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn buildconf_reports_configuration_without_library_versions() {
    compare_buildconf_surface("ffmpeg");
    compare_buildconf_surface("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn loglevel_directive_acceptance_matches_oracle_for_version_requests() {
    let accepted = [
        "repeat",
        "+repeat",
        "-repeat",
        "level",
        "+level",
        "-level",
        "repeat+level+error",
        "-repeat+level+error",
        "level+error",
        "time+datetime+level+error",
        "+error",
        "48",
        "-8",
        "23",
        "57",
        "-1",
        "999",
        "repeat+23",
        "level+23",
        "+23",
        "time+23",
        "repeat+level+23",
        "-repeat+23",
    ];
    let rejected = ["warn", "foo", "repeat+warn", "ERROR"];

    for value in accepted {
        compare_loglevel_acceptance(value, true);
    }
    for value in rejected {
        compare_loglevel_acceptance(value, false);
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn ffmpeg_repeated_diagnostics_match_default_repeat_summary_shape() {
    let input_pattern = invalid_jpeg_sequence_pattern();
    let input_pattern = input_pattern.to_string_lossy().into_owned();
    let oracle = oracle_tool("ffmpeg");

    let default_output = run_oracle(
        &oracle,
        "ffmpeg",
        &[
            "-hide_banner",
            "-loglevel",
            "error",
            "-framerate",
            "1",
            "-i",
            &input_pattern,
            "-f",
            "null",
            "-",
        ],
    );
    let default_stderr = normalize_newlines(&default_output.stderr);
    assert!(
        !default_output.status_success,
        "invalid MJPEG sequence should fail, got stdout:\n{}\nstderr:\n{}",
        default_output.stdout, default_output.stderr
    );
    assert!(
        default_stderr.contains("\n    Last message repeated 1 times\n"),
        "default FFmpeg stderr should include an indented repeat summary, got:\n{default_stderr}"
    );
    assert!(
        !default_stderr.contains("\nLast message repeated"),
        "default FFmpeg stderr should not emit an unindented repeat summary, got:\n{default_stderr}"
    );

    let repeat_output = run_oracle(
        &oracle,
        "ffmpeg",
        &[
            "-hide_banner",
            "-loglevel",
            "repeat+error",
            "-framerate",
            "1",
            "-i",
            &input_pattern,
            "-f",
            "null",
            "-",
        ],
    );
    let repeat_stderr = normalize_newlines(&repeat_output.stderr);
    assert!(
        !repeat_output.status_success,
        "invalid MJPEG sequence with repeat+error should fail, got stdout:\n{}\nstderr:\n{}",
        repeat_output.stdout, repeat_output.stderr
    );
    assert!(
        !repeat_stderr.contains("Last message repeated"),
        "repeat+error should print duplicate diagnostics instead of a summary, got:\n{repeat_stderr}"
    );
    assert!(
        repeat_stderr.matches("No JPEG data found in image").count() >= 2,
        "repeat+error should preserve duplicate decoder diagnostics, got:\n{repeat_stderr}"
    );
}

#[test]
fn parses_split_ffmpeg_library_version_lines() {
    let versions = parse_library_versions(
        "ffmpeg version 8.1.1 Copyright (c) 2000-2026 the FFmpeg developers\n\
         libavutil      60. 26.101 / 60. 26.101\n\
         libavcodec     62. 28.101 / 62. 28.101\n\
         libswscale      9.  5.101 /  9.  5.101\n",
    );

    assert_eq!(
        versions.get("libavutil").map(String::as_str),
        Some("60.26.101")
    );
    assert_eq!(
        versions.get("libavcodec").map(String::as_str),
        Some("62.28.101")
    );
    assert_eq!(
        versions.get("libswscale").map(String::as_str),
        Some("9.5.101")
    );
}

#[test]
fn rust_banner_library_versions_are_parseable() {
    let versions = parse_library_versions(&version_banner("ffmpeg"));

    for (name, version) in TARGET_LIBRARY_VERSIONS {
        assert_eq!(versions.get(*name).map(String::as_str), Some(*version));
    }
}

fn compare_version_surface(tool_name: &str) {
    let oracle = oracle_tool(tool_name);
    let oracle_stdout = run_oracle(&oracle, tool_name, &["-version"]).stdout;
    let oracle_first_line = oracle_stdout
        .lines()
        .next()
        .expect("oracle version output should include a first line");
    let expected_prefix = format!("{tool_name} version {TARGET_FFMPEG_VERSION}");
    assert!(
        oracle_first_line.starts_with(&expected_prefix),
        "oracle `{}` first line should start with `{expected_prefix}`, got `{oracle_first_line}`",
        oracle.display()
    );

    let rust_stdout = version_banner(tool_name);
    let rust_first_line = rust_stdout
        .lines()
        .next()
        .expect("Rust version banner should include a first line");
    assert!(
        rust_first_line.starts_with(&format!(
            "{tool_name} version {TARGET_FFMPEG_VERSION}-rust target FFmpeg {TARGET_FFMPEG_VERSION} \"{TARGET_RELEASE_NAME}\""
        )),
        "Rust first line did not name the pinned FFmpeg target: `{rust_first_line}`"
    );

    let oracle_versions = parse_library_versions(&oracle_stdout);
    let rust_versions = parse_library_versions(&rust_stdout);
    for (name, expected) in TARGET_LIBRARY_VERSIONS {
        assert_eq!(
            oracle_versions.get(*name).map(String::as_str),
            Some(*expected),
            "oracle `{}` did not report the pinned {name} ABI version",
            oracle.display()
        );
        assert_eq!(
            rust_versions.get(*name).map(String::as_str),
            Some(*expected),
            "Rust {tool_name} banner did not report the pinned {name} ABI version"
        );
    }
}

fn compare_double_dash_version_rejection(tool_name: &str) {
    let oracle = oracle_tool(tool_name);
    let oracle_output = run_oracle(&oracle, tool_name, &["--version"]);
    let combined = format!("{}{}", oracle_output.stdout, oracle_output.stderr);
    assert!(
        !oracle_output.status_success || version_error_output(&combined),
        "oracle `{}` should reject --version as a clean success, got status success={} output:\n{}",
        oracle.display(),
        oracle_output.status_success,
        combined
    );

    match tool_name {
        "ffmpeg" => {
            let err = ffmpeg_output(&strings(&["--version"])).unwrap_err();
            assert!(err.message().contains("unknown option"));
        }
        "ffprobe" => {
            let err = ffprobe_output(&strings(&["--version"])).unwrap_err();
            assert!(err.message().contains("unknown option"));
        }
        other => panic!("unsupported tool `{other}`"),
    }
}

fn compare_hide_banner_version(tool_name: &str) {
    let oracle = oracle_tool(tool_name);
    let plain_oracle = run_oracle(&oracle, tool_name, &["-version"]);
    let hide_oracle = run_oracle(&oracle, tool_name, &["-hide_banner", "-version"]);
    assert!(
        hide_oracle.status_success,
        "oracle `{}` should accept -hide_banner -version, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        hide_oracle.stdout,
        hide_oracle.stderr
    );
    assert_eq!(
        hide_oracle.stdout, plain_oracle.stdout,
        "oracle {tool_name} should print the same version surface with or without -hide_banner"
    );
    assert_eq!(
        hide_oracle.stderr, plain_oracle.stderr,
        "oracle {tool_name} should keep the same stderr with or without -hide_banner"
    );

    match tool_name {
        "ffmpeg" => {
            let plain = ffmpeg_output(&strings(&["-version"])).unwrap();
            let hide = ffmpeg_output(&strings(&["-hide_banner", "-version"])).unwrap();
            assert_eq!(hide.stdout(), plain.stdout());
            assert_eq!(hide.stderr(), plain.stderr());
            assert_eq!(hide.output_format(), None);
        }
        "ffprobe" => {
            let plain = ffprobe_output(&strings(&["-version"])).unwrap();
            let hide = ffprobe_output(&strings(&["-hide_banner", "-version"])).unwrap();
            assert_eq!(hide, plain);
        }
        other => panic!("unsupported tool `{other}`"),
    }
}

fn compare_buildconf_surface(tool_name: &str) {
    let oracle = oracle_tool(tool_name);
    let oracle_output = run_oracle(&oracle, tool_name, &["-buildconf"]);
    assert!(
        oracle_output.status_success,
        "oracle `{}` should accept -buildconf, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_output.stdout,
        oracle_output.stderr
    );
    let oracle_combined = format!("{}{}", oracle_output.stdout, oracle_output.stderr);
    let oracle_buildconf = normalized_buildconf_output(tool_name, &oracle_combined);
    let oracle_first_line = oracle_combined
        .lines()
        .find(|line| !line.trim().is_empty())
        .expect("oracle buildconf output should include a first line");
    assert!(
        oracle_first_line.trim_start().starts_with("configuration:"),
        "oracle {tool_name} buildconf first non-empty line should be the configuration header, got `{oracle_first_line}`"
    );
    assert_buildconf_shape(tool_name, oracle_buildconf, "oracle");

    let oracle_unknown = run_oracle(&oracle, tool_name, &["-buildconf", "-not_a_real_option"]);
    assert!(
        oracle_unknown.status_success,
        "oracle `{}` should let -buildconf preempt unknown options, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_unknown.stdout,
        oracle_unknown.stderr
    );
    let oracle_unknown_combined = format!("{}{}", oracle_unknown.stdout, oracle_unknown.stderr);
    assert_buildconf_shape(
        tool_name,
        normalized_buildconf_output(tool_name, &oracle_unknown_combined),
        "oracle unknown-option",
    );

    match tool_name {
        "ffmpeg" => {
            let rust = ffmpeg_output(&strings(&["-hide_banner", "-buildconf"])).unwrap();
            assert_buildconf_shape(tool_name, rust.stdout(), "Rust");
            assert!(rust.stderr().is_empty());
            assert_eq!(rust.output_format(), None);
            let rust_unknown = ffmpeg_output(&strings(&["-buildconf", "-not_a_real_option"]))
                .expect("Rust ffmpeg buildconf should preempt unknown options");
            assert_buildconf_shape(tool_name, rust_unknown.stdout(), "Rust unknown-option");
        }
        "ffprobe" => {
            let rust = ffprobe_output(&strings(&["-hide_banner", "-buildconf"])).unwrap();
            assert_buildconf_shape(tool_name, &rust, "Rust");
            let rust_unknown = ffprobe_output(&strings(&["-buildconf", "-not_a_real_option"]))
                .expect("Rust ffprobe buildconf should preempt unknown options");
            assert_buildconf_shape(tool_name, &rust_unknown, "Rust unknown-option");
        }
        other => panic!("unsupported tool `{other}`"),
    }
}

fn normalized_buildconf_output<'a>(tool_name: &str, output: &'a str) -> &'a str {
    let without_exit_trailer = output
        .split("Exiting with exit code")
        .next()
        .unwrap_or(output);
    let version_trailer = format!("{tool_name} version");
    without_exit_trailer
        .split(version_trailer.as_str())
        .next()
        .unwrap_or(without_exit_trailer)
}

fn assert_buildconf_shape(tool_name: &str, stdout: &str, source: &str) {
    assert!(
        stdout.contains("configuration:\n"),
        "{source} {tool_name} buildconf output should include a configuration header, got:\n{stdout}"
    );
    assert!(
        stdout
            .lines()
            .skip_while(|line| !line.trim_start().starts_with("configuration:"))
            .skip(1)
            .any(|line| line.trim_start().starts_with("--")),
        "{source} {tool_name} buildconf output should include at least one configure flag after the header, got:\n{stdout}"
    );
    assert!(
        !stdout.lines().any(|line| line.trim_start().starts_with("lib")),
        "{source} {tool_name} buildconf output should not include library version lines, got:\n{stdout}"
    );
}

fn version_error_output(output: &str) -> bool {
    output.contains("Unrecognized option")
        || output.contains("Option not found")
        || output.contains("Missing argument for option")
        || output.contains("Invalid loglevel")
}

fn compare_loglevel_acceptance(value: &str, expected_success: bool) {
    for tool_name in ["ffmpeg", "ffprobe"] {
        let oracle = oracle_tool(tool_name);
        let oracle_output = run_oracle(
            &oracle,
            tool_name,
            &["-hide_banner", "-loglevel", value, "-version"],
        );
        let combined = format!("{}{}", oracle_output.stdout, oracle_output.stderr);
        assert_eq!(
            oracle_output.status_success,
            expected_success,
            "oracle `{}` {tool_name} -loglevel {value:?} acceptance mismatch, output:\n{}",
            oracle.display(),
            combined
        );

        assert_eq!(
            parse_log_level_directive(value).is_some(),
            expected_success,
            "Rust parser acceptance mismatch for -loglevel {value:?}"
        );

        match tool_name {
            "ffmpeg" => {
                let rust =
                    ffmpeg_output(&strings(&["-hide_banner", "-loglevel", value, "-version"]));
                assert_eq!(
                    rust.is_ok(),
                    expected_success,
                    "Rust ffmpeg-rs -loglevel {value:?} version acceptance mismatch: {rust:?}"
                );
            }
            "ffprobe" => {
                let rust =
                    ffprobe_output(&strings(&["-hide_banner", "-loglevel", value, "-version"]));
                assert_eq!(
                    rust.is_ok(),
                    expected_success,
                    "Rust ffprobe-rs -loglevel {value:?} version acceptance mismatch: {rust:?}"
                );
            }
            other => panic!("unsupported tool `{other}`"),
        }
    }
}

struct OracleOutput {
    status_success: bool,
    stdout: String,
    stderr: String,
}

fn run_oracle(path: &Path, tool_name: &str, args: &[&str]) -> OracleOutput {
    let output = Command::new(path)
        .args(args)
        .output()
        .unwrap_or_else(|err| {
            panic!(
                "failed to run {tool_name} oracle `{}` with args {:?}: {err}",
                path.display(),
                args
            )
        });

    let stdout = String::from_utf8(output.stdout).unwrap_or_else(|err| {
        panic!(
            "{tool_name} oracle `{}` stdout must be UTF-8 for args {:?}: {err}",
            path.display(),
            args
        )
    });
    let stderr = String::from_utf8(output.stderr).unwrap_or_else(|err| {
        panic!(
            "{tool_name} oracle `{}` stderr must be UTF-8 for args {:?}: {err}",
            path.display(),
            args
        )
    });

    if args == ["-version"] {
        assert!(
            output.status.success(),
            "{tool_name} oracle `{}` failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            output.status.code(),
            stdout,
            stderr
        );
    }

    OracleOutput {
        status_success: output.status.success(),
        stdout,
        stderr,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn invalid_jpeg_sequence_pattern() -> PathBuf {
    let dir = repository_root()
        .join("target/oracle/fftools-cli-repeat-diagnostics")
        .join(std::process::id().to_string());
    fs::create_dir_all(&dir).unwrap_or_else(|err| {
        panic!(
            "create invalid MJPEG sequence oracle directory `{}`: {err}",
            dir.display()
        )
    });

    for index in 1..=2 {
        let path = dir.join(format!("img{index:03}.jpg"));
        fs::write(&path, [0_u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap_or_else(|err| {
            panic!(
                "write invalid MJPEG sequence file `{}`: {err}",
                path.display()
            )
        });
    }

    dir.join("img%03d.jpg")
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn parse_library_versions(stdout: &str) -> BTreeMap<String, String> {
    stdout
        .lines()
        .filter_map(parse_library_version_line)
        .collect()
}

fn parse_library_version_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("lib") {
        return None;
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let name = parts.next()?;
    let rest = parts.next()?.trim();
    let first_version = rest.split('/').next()?.trim();
    if first_version.is_empty() {
        return None;
    }

    let normalized = first_version.split_whitespace().collect::<String>();
    Some((name.to_owned(), normalized))
}

fn oracle_tool(tool_name: &str) -> PathBuf {
    let env_var = format!("{}_ORACLE", tool_name.to_ascii_uppercase());
    if let Ok(path) = env::var(&env_var) {
        return require_tool_path(PathBuf::from(path), &env_var);
    }

    if tool_name == "ffprobe" {
        if let Ok(ffmpeg_path) = env::var("FFMPEG_ORACLE") {
            let ffmpeg_path = resolve_tool_path(PathBuf::from(ffmpeg_path));
            for candidate in sibling_tool_candidates(&ffmpeg_path, "ffprobe") {
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }

    let root = repository_root();
    for candidate in default_tool_candidates(&root, tool_name) {
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "missing pinned {tool_name} oracle; set {env_var} or install `{}`",
        root.join("third_party/ffmpeg-oracle/build/bin")
            .join(tool_name)
            .display()
    );
}

fn require_tool_path(path: PathBuf, env_var: &str) -> PathBuf {
    let resolved = resolve_tool_path(path);
    assert!(
        resolved.is_file(),
        "{env_var} must point to the pinned FFmpeg 8.1.1 binary, got `{}`",
        resolved.display()
    );
    resolved
}

fn resolve_tool_path(path: PathBuf) -> PathBuf {
    if path.is_file() || path.is_absolute() {
        return path;
    }
    let candidate = repository_root().join(&path);
    if candidate.is_file() {
        return candidate;
    }
    path
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("fftools crate should be under crates/")
        .to_path_buf()
}

fn sibling_tool_candidates(ffmpeg_path: &Path, tool_name: &str) -> Vec<PathBuf> {
    let Some(parent) = ffmpeg_path.parent() else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    if ffmpeg_path
        .extension()
        .is_some_and(|extension| extension == "exe")
    {
        candidates.push(parent.join(format!("{tool_name}.exe")));
    }
    if ffmpeg_path
        .extension()
        .is_some_and(|extension| extension == "cmd")
    {
        candidates.push(parent.join(format!("{tool_name}.cmd")));
    }
    candidates.push(parent.join(tool_name));
    candidates.push(parent.join(format!("{tool_name}.exe")));
    candidates.push(parent.join(format!("{tool_name}.cmd")));
    candidates
}

fn default_tool_candidates(root: &Path, tool_name: &str) -> Vec<PathBuf> {
    let bin = root.join("third_party/ffmpeg-oracle/build/bin");
    if cfg!(windows) {
        vec![
            bin.join(format!("{tool_name}.exe")),
            bin.join(format!("{tool_name}.cmd")),
            bin.join(tool_name),
        ]
    } else {
        vec![
            bin.join(tool_name),
            bin.join(format!("{tool_name}.exe")),
            bin.join(format!("{tool_name}.cmd")),
        ]
    }
}
