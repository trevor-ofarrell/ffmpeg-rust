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
fn version_and_buildconf_args_follow_first_occurrence() {
    compare_version_buildconf_precedence("ffmpeg");
    compare_version_buildconf_precedence("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn version_requests_ignore_later_unknown_options() {
    compare_version_unknown_order("ffmpeg");
    compare_version_unknown_order("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn version_and_buildconf_preempt_late_value_options() {
    compare_version_buildconf_ignores_value_option_tail("ffmpeg");
    compare_version_buildconf_ignores_value_option_tail("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn version_requests_warn_for_later_invalid_loglevel_but_still_succeed() {
    compare_trailing_invalid_loglevel_warning("ffmpeg", "-version", "warn");
    compare_trailing_invalid_loglevel_warning("ffprobe", "-version", "warn");
    compare_trailing_invalid_loglevel_warning("ffmpeg", "-buildconf", "foo");
    compare_trailing_invalid_loglevel_warning("ffprobe", "-buildconf", "foo");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn version_buildconf_fail_without_banner_for_preceding_invalid_loglevel() {
    compare_version_buildconf_invalid_loglevel_preempts_banner("ffmpeg");
    compare_version_buildconf_invalid_loglevel_preempts_banner("ffprobe");
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn dash_prefixed_values_are_consumed_before_special_requests() {
    compare_dash_prefixed_value_case(
        "ffmpeg",
        &["-f", "-version", "-i", "in.wav", "-f", "null", "-"],
        "Unknown input format",
        "input format",
    );
    compare_dash_prefixed_value_case(
        "ffprobe",
        &["-f", "-version", "-show_format", "in.wav"],
        "Unknown input format",
        "unsupported input format",
    );
    compare_dash_prefixed_value_case(
        "ffmpeg",
        &["-v", "-not-a-level", "-i", "in.wav", "out.wav"],
        "Invalid loglevel",
        "invalid loglevel",
    );
    compare_dash_prefixed_value_case(
        "ffprobe",
        &["-v", "-not-a-level", "-show_format", "in.wav"],
        "Invalid loglevel",
        "invalid loglevel",
    );
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 oracle; set FFMPEG_ORACLE or install third_party/ffmpeg-oracle/build/bin/ffmpeg"]
fn version_and_buildconf_trailing_hide_banner_is_ignored() {
    compare_version_buildconf_trailing_hide_banner("ffmpeg");
    compare_version_buildconf_trailing_hide_banner("ffprobe");
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

fn compare_version_buildconf_precedence(tool_name: &str) {
    let oracle = oracle_tool(tool_name);

    let oracle_version_then_buildconf = run_oracle(&oracle, tool_name, &["-version", "-buildconf"]);
    assert!(
        oracle_version_then_buildconf.status_success,
        "oracle `{}` should accept -version -buildconf, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_version_then_buildconf.stdout,
        oracle_version_then_buildconf.stderr
    );
    let oracle_version_output = format!(
        "{}{}",
        oracle_version_then_buildconf.stdout, oracle_version_then_buildconf.stderr
    );
    assert!(
        oracle_version_output.starts_with(&format!("{tool_name} version ")),
        "oracle `{}` should treat -version first when followed by -buildconf, got:\n{oracle_version_output}",
        oracle.display()
    );
    assert!(
        oracle_version_output.contains("libavutil"),
        "oracle `{}` should print library versions for -version -buildconf, got:\n{oracle_version_output}",
        oracle.display()
    );

    let oracle_buildconf_then_version = run_oracle(&oracle, tool_name, &["-buildconf", "-version"]);
    assert!(
        oracle_buildconf_then_version.status_success,
        "oracle `{}` should accept -buildconf -version, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_buildconf_then_version.stdout,
        oracle_buildconf_then_version.stderr
    );
    let oracle_buildconf_output = format!(
        "{}{}",
        oracle_buildconf_then_version.stdout, oracle_buildconf_then_version.stderr
    );
    let normalized_oracle_buildconf =
        normalized_buildconf_output(tool_name, &oracle_buildconf_output);
    assert_buildconf_shape(tool_name, normalized_oracle_buildconf, "oracle");

    match tool_name {
        "ffmpeg" => {
            let rust_version = ffmpeg_output(&strings(&["-version", "-buildconf"]))
                .expect("ffmpeg version request should honor first occurrence semantics");
            assert!(rust_version.stdout().starts_with("ffmpeg version"));
            assert!(rust_version.stdout().contains("libavutil"));
            assert_eq!(rust_version.output_format(), None);

            let rust_buildconf = ffmpeg_output(&strings(&["-buildconf", "-version"]))
                .expect("ffmpeg buildconf should honor first occurrence semantics");
            assert_buildconf_shape(tool_name, rust_buildconf.stdout(), "Rust");
            assert_eq!(rust_buildconf.output_format(), None);
        }
        "ffprobe" => {
            let rust_version = ffprobe_output(&strings(&["-version", "-buildconf"]))
                .expect("ffprobe version request should honor first occurrence semantics");
            assert!(rust_version.starts_with("ffprobe version"));
            assert!(rust_version.contains("libavutil"));

            let rust_buildconf = ffprobe_output(&strings(&["-buildconf", "-version"]))
                .expect("ffprobe buildconf should honor first occurrence semantics");
            assert_buildconf_shape(tool_name, &rust_buildconf, "Rust");
        }
        other => panic!("unsupported tool `{other}`"),
    }
}

fn compare_version_unknown_order(tool_name: &str) {
    let oracle = oracle_tool(tool_name);
    let oracle_version_prefix = format!("{tool_name} version {TARGET_FFMPEG_VERSION}");

    assert_version_request_case(
        tool_name,
        &oracle,
        &["-version", "-not_a_real_option"],
        true,
        oracle_version_prefix.as_str(),
        &version_banner(tool_name),
        "version request with trailing unknown option",
    );
    assert_version_request_case(
        tool_name,
        &oracle,
        &["-not_a_real_option", "-version"],
        false,
        oracle_version_prefix.as_str(),
        &version_banner(tool_name),
        "version request with preceding unknown option",
    );
}

fn compare_version_buildconf_ignores_value_option_tail(tool_name: &str) {
    let oracle = oracle_tool(tool_name);

    let oracle_version = run_oracle(&oracle, tool_name, &["-version", "-f"]);
    assert!(
        oracle_version.status_success,
        "oracle `{}` should accept -version with trailing value-taking option, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_version.stdout,
        oracle_version.stderr
    );
    assert!(
        oracle_version
            .stdout
            .contains(&format!("{tool_name} version {TARGET_FFMPEG_VERSION}")),
        "oracle `{}` -version -f should still emit version surface, got:\n{}",
        oracle.display(),
        oracle_version.stdout
    );

    let oracle_buildconf = run_oracle(&oracle, tool_name, &["-buildconf", "-f"]);
    assert!(
        oracle_buildconf.status_success,
        "oracle `{}` should accept -buildconf with trailing value-taking option, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_buildconf.stdout,
        oracle_buildconf.stderr
    );
    assert_buildconf_shape(
        tool_name,
        normalized_buildconf_output(tool_name, &oracle_buildconf.stdout),
        "oracle",
    );

    match tool_name {
        "ffmpeg" => {
            let rust_version = ffmpeg_output(&strings(&["-version", "-f"]))
                .expect("Rust ffmpeg should treat -version as first token");
            assert!(rust_version
                .stdout()
                .starts_with(&format!("{tool_name} version {TARGET_FFMPEG_VERSION}")));
            assert!(rust_version.stderr().is_empty());
            assert_eq!(rust_version.output_format(), None);

            let rust_buildconf = ffmpeg_output(&strings(&["-buildconf", "-f"]))
                .expect("Rust ffmpeg buildconf should ignore trailing -f");
            assert_buildconf_shape(tool_name, rust_buildconf.stdout(), "Rust");
            assert!(rust_buildconf.stderr().is_empty());
            assert_eq!(rust_buildconf.output_format(), None);
        }
        "ffprobe" => {
            let rust_version = ffprobe_output(&strings(&["-version", "-f"]))
                .expect("Rust ffprobe should treat -version as first token");
            assert!(
                rust_version.starts_with(&format!("{tool_name} version {TARGET_FFMPEG_VERSION}"))
            );
            let rust_buildconf = ffprobe_output(&strings(&["-buildconf", "-f"]))
                .expect("Rust ffprobe buildconf should ignore trailing -f");
            assert_buildconf_shape(tool_name, &rust_buildconf, "Rust");
        }
        other => panic!("unsupported tool `{other}`"),
    }
}

fn compare_version_buildconf_trailing_hide_banner(tool_name: &str) {
    let oracle = oracle_tool(tool_name);

    let oracle_version_plain = run_oracle(&oracle, tool_name, &["-version"]);
    assert!(
        oracle_version_plain.status_success,
        "oracle `{}` should accept -version for trailing hide_banner comparison, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_version_plain.stdout,
        oracle_version_plain.stderr
    );
    let oracle_version_tail_hide = run_oracle(&oracle, tool_name, &["-version", "-hide_banner"]);
    assert!(
        oracle_version_tail_hide.status_success,
        "oracle `{}` should accept -version -hide_banner, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_version_tail_hide.stdout,
        oracle_version_tail_hide.stderr
    );
    assert!(
        oracle_version_plain.stdout == oracle_version_tail_hide.stdout,
        "oracle `{}` should print the same -version output with trailing -hide_banner, got plain:\n{}\nwith tail hide_banner:\n{}",
        oracle.display(),
        oracle_version_plain.stdout,
        oracle_version_tail_hide.stdout
    );
    assert!(
        oracle_version_plain.stderr == oracle_version_tail_hide.stderr,
        "oracle `{}` should keep trailing -hide_banner from mutating version stderr, got plain:\n{}\nwith tail hide_banner:\n{}",
        oracle.display(),
        oracle_version_plain.stderr,
        oracle_version_tail_hide.stderr
    );

    let oracle_buildconf_plain = run_oracle(&oracle, tool_name, &["-buildconf"]);
    assert!(
        oracle_buildconf_plain.status_success,
        "oracle `{}` should accept -buildconf for trailing hide_banner comparison, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_buildconf_plain.stdout,
        oracle_buildconf_plain.stderr
    );
    let oracle_buildconf_tail_hide =
        run_oracle(&oracle, tool_name, &["-buildconf", "-hide_banner"]);
    assert!(
        oracle_buildconf_tail_hide.status_success,
        "oracle `{}` should accept -buildconf -hide_banner, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_buildconf_tail_hide.stdout,
        oracle_buildconf_tail_hide.stderr
    );
    assert!(
        normalized_buildconf_output(
            tool_name,
            &format!("{}{}", oracle_buildconf_plain.stdout, oracle_buildconf_plain.stderr),
        ) == normalized_buildconf_output(
            tool_name,
            &format!("{}{}", oracle_buildconf_tail_hide.stdout, oracle_buildconf_tail_hide.stderr),
        ),
        "oracle `{}` should print the same buildconf output with trailing -hide_banner, got plain:\n{}\nwith tail hide_banner:\n{}",
        oracle.display(),
        oracle_buildconf_plain.stdout,
        oracle_buildconf_tail_hide.stdout
    );

    match tool_name {
        "ffmpeg" => {
            let rust_version = ffmpeg_output(&strings(&["-version"]))
                .expect("ffmpeg version request should succeed");
            let rust_version_tail_hide = ffmpeg_output(&strings(&["-version", "-hide_banner"]))
                .expect("ffmpeg should keep trailing -hide_banner after -version");
            assert_eq!(rust_version_tail_hide.stdout(), rust_version.stdout());
            assert_eq!(rust_version_tail_hide.stderr(), rust_version.stderr());
            assert_eq!(rust_version.output_format(), None);

            let rust_buildconf = ffmpeg_output(&strings(&["-buildconf"]))
                .expect("ffmpeg buildconf request should succeed");
            let rust_buildconf_tail_hide = ffmpeg_output(&strings(&["-buildconf", "-hide_banner"]))
                .expect("ffmpeg should keep trailing -hide_banner after -buildconf");
            assert_eq!(rust_buildconf.output_format(), None);
            assert_eq!(rust_buildconf_tail_hide.output_format(), None);
            let rust_buildconf_output =
                format!("{}{}", rust_buildconf.stdout(), rust_buildconf.stderr());
            let rust_buildconf_tail_output = format!(
                "{}{}",
                rust_buildconf_tail_hide.stdout(),
                rust_buildconf_tail_hide.stderr()
            );
            assert_buildconf_shape(
                tool_name,
                normalized_buildconf_output(tool_name, &rust_buildconf_output),
                "Rust",
            );
            assert_eq!(
                normalized_buildconf_output(tool_name, &rust_buildconf_output),
                normalized_buildconf_output(tool_name, &rust_buildconf_tail_output)
            );
        }
        "ffprobe" => {
            let rust_version = ffprobe_output(&strings(&["-version"]))
                .expect("ffprobe version request should succeed");
            let rust_version_tail_hide = ffprobe_output(&strings(&["-version", "-hide_banner"]))
                .expect("ffprobe should keep trailing -hide_banner after -version");
            assert_eq!(rust_version_tail_hide, rust_version);

            let rust_buildconf = ffprobe_output(&strings(&["-buildconf"]))
                .expect("ffprobe buildconf request should succeed");
            let rust_buildconf_tail_hide =
                ffprobe_output(&strings(&["-buildconf", "-hide_banner"]))
                    .expect("ffprobe should keep trailing -hide_banner after -buildconf");
            assert_buildconf_shape(tool_name, &rust_buildconf, "Rust");
            assert_eq!(
                normalized_buildconf_output(tool_name, &rust_buildconf),
                normalized_buildconf_output(tool_name, &rust_buildconf_tail_hide)
            );
        }
        other => panic!("unsupported tool `{other}`"),
    }
}

fn compare_trailing_invalid_loglevel_warning(tool_name: &str, request: &str, value: &str) {
    let oracle = oracle_tool(tool_name);
    let oracle_output = run_oracle(&oracle, tool_name, &[request, "-loglevel", value]);
    assert!(
        oracle_output.status_success,
        "oracle `{}` should keep {request} successful despite trailing invalid loglevel, stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        oracle_output.stdout,
        oracle_output.stderr
    );
    assert!(
        oracle_output.stderr.contains("Invalid loglevel"),
        "oracle `{}` should report invalid trailing loglevel, stderr:\n{}",
        oracle.display(),
        oracle_output.stderr
    );

    let rust = run_rust_tool(tool_name, &[request, "-loglevel", value]);
    assert!(
        rust.status_success,
        "Rust {tool_name}-rs should keep {request} successful despite trailing invalid loglevel, stdout:\n{}\nstderr:\n{}",
        rust.stdout,
        rust.stderr
    );
    assert!(
        rust.stderr
            .contains(&format!("Invalid loglevel \"{value}\"")),
        "Rust {tool_name}-rs should report invalid trailing loglevel, stderr:\n{}",
        rust.stderr
    );
    if request == "-version" {
        assert!(rust.stdout.starts_with(&format!("{tool_name} version")));
    } else {
        assert!(rust.stdout.starts_with("  configuration:\n"));
    }
}

fn compare_version_buildconf_invalid_loglevel_preempts_banner(tool_name: &str) {
    let oracle = oracle_tool(tool_name);
    let invalid_prefix_values = ["-v", "-loglevel"];

    for option_name in &invalid_prefix_values {
        let oracle_version = run_oracle(&oracle, tool_name, &[option_name, "foo", "-version"]);
        let oracle_buildconf = run_oracle(&oracle, tool_name, &[option_name, "foo", "-buildconf"]);
        let oracle_version_combined = format!("{}{}", oracle_version.stdout, oracle_version.stderr);
        let oracle_buildconf_combined =
            format!("{}{}", oracle_buildconf.stdout, oracle_buildconf.stderr);

        assert!(
            !oracle_version.status_success,
            "oracle `{}` should fail for `{option_name} foo -version`, got status success={}",
            oracle.display(),
            oracle_version.status_success
        );
        assert!(
            !oracle_version_combined.starts_with(&format!("{tool_name} version")),
            "oracle `{}` should not preempt invalid loglevel prefix as version banner for `{option_name} foo -version`, got: {oracle_version_combined}",
            oracle.display()
        );
        assert!(
            oracle_version_combined.to_lowercase().contains("invalid loglevel"),
            "oracle `{}` should mention invalid loglevel for `{option_name} foo -version`, got: {oracle_version_combined}",
            oracle.display()
        );

        assert!(
            !oracle_buildconf.status_success,
            "oracle `{}` should fail for `{option_name} foo -buildconf`, got status success={}",
            oracle.display(),
            oracle_buildconf.status_success
        );
        assert!(
            !oracle_buildconf_combined.starts_with("  configuration:\n"),
            "oracle `{}` should not preempt invalid loglevel prefix as buildconf banner for `{option_name} foo -buildconf`, got: {oracle_buildconf_combined}",
            oracle.display()
        );
        assert!(
            oracle_buildconf_combined.to_lowercase().contains("invalid loglevel"),
            "oracle `{}` should mention invalid loglevel for `{option_name} foo -buildconf`, got: {oracle_buildconf_combined}",
            oracle.display()
        );
    }

    for option_name in &invalid_prefix_values {
        let rust_version = run_rust_tool(tool_name, &[option_name, "foo", "-version"]);
        let rust_buildconf = run_rust_tool(tool_name, &[option_name, "foo", "-buildconf"]);
        let rust_version_combined = format!("{}{}", rust_version.stdout, rust_version.stderr);
        let rust_buildconf_combined = format!("{}{}", rust_buildconf.stdout, rust_buildconf.stderr);

        assert!(
            !rust_version.status_success,
            "Rust {tool_name}-rs should fail for `{option_name} foo -version` with status={}",
            rust_version.status_success
        );
        assert!(
            !rust_version_combined.contains("ffmpeg version") && !rust_version_combined.contains("ffprobe version"),
            "Rust {tool_name}-rs should not emit version banner for `{option_name} foo -version`, got: {rust_version_combined}"
        );
        assert!(
            rust_version_combined.to_lowercase().contains("invalid loglevel"),
            "Rust {tool_name}-rs should mention invalid loglevel for `{option_name} foo -version`, got: {rust_version_combined}"
        );

        assert!(
            !rust_buildconf.status_success,
            "Rust {tool_name}-rs should fail for `{option_name} foo -buildconf` with status={}",
            rust_buildconf.status_success
        );
        assert!(
            !rust_buildconf_combined.contains("  configuration:\n") && !rust_buildconf_combined.contains("ffmpeg version") && !rust_buildconf_combined.contains("ffprobe version"),
            "Rust {tool_name}-rs should not emit buildconf banner for `{option_name} foo -buildconf`, got: {rust_buildconf_combined}"
        );
        assert!(
            rust_buildconf_combined.to_lowercase().contains("invalid loglevel"),
            "Rust {tool_name}-rs should mention invalid loglevel for `{option_name} foo -buildconf`, got: {rust_buildconf_combined}"
        );
    }
}

fn compare_dash_prefixed_value_case(
    tool_name: &str,
    args: &[&str],
    oracle_snippet: &str,
    rust_snippet: &str,
) {
    let oracle = oracle_tool(tool_name);
    let oracle_output = run_oracle(&oracle, tool_name, args);
    let oracle_combined = format!("{}{}", oracle_output.stdout, oracle_output.stderr);
    assert!(
        !oracle_output.status_success,
        "oracle `{}` should reject {tool_name} args {:?}, got stdout:\n{}\nstderr:\n{}",
        oracle.display(),
        args,
        oracle_output.stdout,
        oracle_output.stderr
    );
    assert!(
        oracle_combined.contains(oracle_snippet),
        "oracle `{}` should mention `{oracle_snippet}` for args {:?}, got:\n{}",
        oracle.display(),
        args,
        oracle_combined
    );

    match tool_name {
        "ffmpeg" => {
            let err = ffmpeg_output(&strings(args))
                .expect_err("Rust ffmpeg should reject the dash-prefixed value case");
            assert!(
                err.message().contains(rust_snippet),
                "Rust ffmpeg should mention `{rust_snippet}` for args {:?}, got: {}",
                args,
                err.message()
            );
        }
        "ffprobe" => {
            let err = ffprobe_output(&strings(args))
                .expect_err("Rust ffprobe should reject the dash-prefixed value case");
            assert!(
                err.message().contains(rust_snippet),
                "Rust ffprobe should mention `{rust_snippet}` for args {:?}, got: {}",
                args,
                err.message()
            );
        }
        other => panic!("unsupported tool `{other}`"),
    }
}

fn assert_version_request_case(
    tool_name: &str,
    oracle: &Path,
    args: &[&str],
    expected_success: bool,
    expected_oracle_prefix: &str,
    expected_rust_banner: &str,
    label: &str,
) {
    let oracle_output = run_oracle(oracle, tool_name, args);
    let oracle_combined = format!("{}{}", oracle_output.stdout, oracle_output.stderr);
    assert_eq!(
        oracle_output.status_success,
        expected_success,
        "oracle `{}` {label} success mismatch, output:\n{}",
        oracle.display(),
        oracle_combined
    );
    if expected_success {
        assert!(
            oracle_output.stdout.starts_with(expected_oracle_prefix),
            "oracle `{}` {label} should emit the banner on stdout, got stdout:\n{}\nstderr:\n{}",
            oracle.display(),
            oracle_output.stdout,
            oracle_output.stderr
        );
        assert!(
            oracle_output.stderr.is_empty(),
            "oracle `{}` {label} should not emit stderr, got:\n{}",
            oracle.display(),
            oracle_output.stderr
        );
    } else {
        assert!(
            oracle_output.stdout.is_empty(),
            "oracle `{}` {label} should not emit stdout on failure, got stdout:\n{}\nstderr:\n{}",
            oracle.display(),
            oracle_output.stdout,
            oracle_output.stderr
        );
        assert!(
            oracle_output.stderr.starts_with(expected_oracle_prefix),
            "oracle `{}` {label} should emit the banner on stderr, got stdout:\n{}\nstderr:\n{}",
            oracle.display(),
            oracle_output.stdout,
            oracle_output.stderr
        );
        assert!(
            version_error_output(&oracle_combined),
            "oracle `{}` {label} should report an option error, got:\n{}",
            oracle.display(),
            oracle_combined
        );
    }

    match tool_name {
        "ffmpeg" => {
            let rust = ffmpeg_output(&strings(args));
            if expected_success {
                let output = rust.expect("Rust ffmpeg should accept the version/buildconf request");
                assert!(
                    output.stdout().starts_with(expected_rust_banner),
                    "Rust ffmpeg {label} should emit the banner, got:\n{}",
                    output.stdout()
                );
                assert!(output.stderr().is_empty());
                assert_eq!(output.output_format(), None);
            } else {
                let err = rust.expect_err("Rust ffmpeg should reject the earlier unknown option");
                assert!(
                    err.message().contains("unknown option"),
                    "Rust ffmpeg {label} should report an option error, got: {}",
                    err.message()
                );
                assert_eq!(err.banner(), Some(expected_rust_banner));
            }
        }
        "ffprobe" => {
            let rust = ffprobe_output(&strings(args));
            if expected_success {
                let output =
                    rust.expect("Rust ffprobe should accept the version/buildconf request");
                assert!(
                    output.starts_with(expected_rust_banner),
                    "Rust ffprobe {label} should emit the banner, got:\n{}",
                    output
                );
            } else {
                let err = rust.expect_err("Rust ffprobe should reject the earlier unknown option");
                assert!(
                    err.message().contains("unknown option"),
                    "Rust ffprobe {label} should report an option error, got: {}",
                    err.message()
                );
                assert_eq!(err.banner(), Some(expected_rust_banner));
            }
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

fn run_rust_tool(tool_name: &str, args: &[&str]) -> OracleOutput {
    let bin_name = match tool_name {
        "ffmpeg" => "ffmpeg-rs",
        "ffprobe" => "ffprobe-rs",
        other => panic!("unsupported Rust tool `{other}`"),
    };
    let env_name = format!("CARGO_BIN_EXE_{bin_name}");
    let path = env::var_os(&env_name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("Cargo should set {env_name} for integration tests"));
    let output = Command::new(&path)
        .args(args)
        .output()
        .unwrap_or_else(|err| {
            panic!(
                "failed to run Rust {tool_name} `{}` with args {:?}: {err}",
                path.display(),
                args
            )
        });
    OracleOutput {
        status_success: output.status.success(),
        stdout: String::from_utf8(output.stdout).unwrap_or_else(|err| {
            panic!(
                "Rust {tool_name} `{}` stdout must be UTF-8 for args {:?}: {err}",
                path.display(),
                args
            )
        }),
        stderr: String::from_utf8(output.stderr).unwrap_or_else(|err| {
            panic!(
                "Rust {tool_name} `{}` stderr must be UTF-8 for args {:?}: {err}",
                path.display(),
                args
            )
        }),
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
