use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use avutil::{
    AvLogContextPrefix, AvLogFormatLine, AvLogFormatLine2, DefaultCallbackColorState,
    DefaultCallbackPrefixState, LogColorMode, LogFlags, LogFormatOptions, LogLevel, LogRecord,
    LogTimestamp, Logger,
};

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 libavutil oracle under third_party/ffmpeg-oracle/wsl"]
fn libavutil_logging_constants_and_state_match_current_model() {
    let repo_root = repo_root();
    let oracle_root = oracle_root(&repo_root);
    let include_dir = oracle_root.join("wsl/include");
    let libavutil = oracle_root.join("wsl/lib/libavutil.a");

    assert!(
        include_dir.join("libavutil/log.h").is_file(),
        "missing pinned FFmpeg libavutil headers under `{}`",
        include_dir.display()
    );
    assert!(
        libavutil.is_file(),
        "missing pinned FFmpeg libavutil static library `{}`",
        libavutil.display()
    );

    let work_dir = repo_root.join("target/oracle/avutil-logging");
    fs::create_dir_all(&work_dir).expect("create avutil-logging oracle work dir");
    let source = work_dir.join("logging_oracle.c");
    let executable = work_dir.join("logging_oracle");
    fs::write(&source, oracle_c_source()).expect("write avutil-logging oracle C source");

    let stdout = compile_and_run_oracle(&include_dir, &libavutil, &source, &executable);
    let oracle = parse_oracle_output(&stdout);
    let expected = expected_rows();

    for (name, expected_value) in expected {
        let actual = oracle
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle row `{name}`"));
        let actual = actual
            .parse::<i32>()
            .unwrap_or_else(|err| panic!("invalid value in oracle row `{name}`: {err}"));
        assert_eq!(
            actual, expected_value,
            "logging oracle mismatch for `{name}`"
        );
    }

    let expected_text = expected_text_rows();
    for (name, expected_value) in expected_text {
        let actual = oracle
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle row `{name}`"));
        assert_eq!(
            actual, &expected_value,
            "logging oracle text mismatch for `{name}`"
        );
    }
}

#[test]
#[ignore = "requires pinned FFmpeg 8.1.1 source/build cache; set FFMPEG_FATE_BUILD_DIR or run scripts/bootstrap_ffmpeg_oracle_wsl.sh"]
fn upstream_libavutil_log_test_program_passes_and_has_no_fate_target() {
    let output = if cfg!(windows) {
        let build_dir = match env::var("FFMPEG_FATE_BUILD_DIR") {
            Ok(build_dir) => {
                let build_dir = if build_dir.starts_with('/') || build_dir.starts_with('~') {
                    build_dir
                } else {
                    to_wsl_path(Path::new(&build_dir))
                };
                shell_quote(&build_dir)
            }
            Err(_) => {
                "\"${FFMPEGRUST_ORACLE_WORK:-$HOME/.cache/ffmpegrust/ffmpeg-oracle-n8.1.1}/build\""
                    .to_string()
            }
        };
        Command::new("wsl")
            .args([
                "-d",
                "Ubuntu",
                "--exec",
                "bash",
                "-lc",
                &upstream_log_test_script(&build_dir),
            ])
            .output()
            .expect("run upstream FFmpeg libavutil/tests/log through WSL")
    } else {
        let build_dir = env::var_os("FFMPEG_FATE_BUILD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env::var("HOME").expect("HOME must be set"))
                    .join(".cache/ffmpegrust/ffmpeg-oracle-n8.1.1/build")
            });
        Command::new("sh")
            .arg("-c")
            .arg(upstream_log_test_script(&shell_quote(
                &build_dir.display().to_string(),
            )))
            .output()
            .unwrap_or_else(|err| {
                panic!(
                    "run upstream FFmpeg libavutil/tests/log in `{}`: {err}",
                    build_dir.display()
                )
            })
    };

    assert!(
        output.status.success(),
        "upstream FFmpeg libavutil/tests/log disposition failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn expected_rows() -> BTreeMap<&'static str, i32> {
    let all_flags = LogFlags::SKIP_REPEATED
        | LogFlags::PRINT_LEVEL
        | LogFlags::PRINT_TIME
        | LogFlags::PRINT_DATETIME;
    let mut rows = [
        ("AV_LOG_QUIET", LogLevel::Quiet.as_ffmpeg_value()),
        ("AV_LOG_PANIC", LogLevel::Panic.as_ffmpeg_value()),
        ("AV_LOG_FATAL", LogLevel::Fatal.as_ffmpeg_value()),
        ("AV_LOG_ERROR", LogLevel::Error.as_ffmpeg_value()),
        ("AV_LOG_WARNING", LogLevel::Warning.as_ffmpeg_value()),
        ("AV_LOG_INFO", LogLevel::Info.as_ffmpeg_value()),
        ("AV_LOG_VERBOSE", LogLevel::Verbose.as_ffmpeg_value()),
        ("AV_LOG_DEBUG", LogLevel::Debug.as_ffmpeg_value()),
        ("AV_LOG_TRACE", LogLevel::Trace.as_ffmpeg_value()),
        (
            "AV_LOG_MAX_OFFSET",
            LogLevel::Trace.as_ffmpeg_value() - LogLevel::Quiet.as_ffmpeg_value(),
        ),
        (
            "AV_LOG_SKIP_REPEATED",
            LogFlags::SKIP_REPEATED.bits() as i32,
        ),
        ("AV_LOG_PRINT_LEVEL", LogFlags::PRINT_LEVEL.bits() as i32),
        ("AV_LOG_PRINT_TIME", LogFlags::PRINT_TIME.bits() as i32),
        (
            "AV_LOG_PRINT_DATETIME",
            LogFlags::PRINT_DATETIME.bits() as i32,
        ),
        ("default-level", LogLevel::Info.as_ffmpeg_value()),
        ("set-level-quiet", LogLevel::Quiet.as_ffmpeg_value()),
        ("set-level-panic", LogLevel::Panic.as_ffmpeg_value()),
        ("set-level-fatal", LogLevel::Fatal.as_ffmpeg_value()),
        ("set-level-error", LogLevel::Error.as_ffmpeg_value()),
        ("set-level-warning", LogLevel::Warning.as_ffmpeg_value()),
        ("set-level-info", LogLevel::Info.as_ffmpeg_value()),
        ("set-level-verbose", LogLevel::Verbose.as_ffmpeg_value()),
        ("set-level-debug", LogLevel::Debug.as_ffmpeg_value()),
        ("set-level-trace", LogLevel::Trace.as_ffmpeg_value()),
        ("set-level-raw-minus-one", -1),
        ("set-level-raw-between-error-warning", 23),
        ("set-level-raw-above-trace", 57),
        ("set-flags-empty", LogFlags::empty().bits() as i32),
        (
            "set-flags-skip-repeated",
            LogFlags::SKIP_REPEATED.bits() as i32,
        ),
        ("set-flags-print-level", LogFlags::PRINT_LEVEL.bits() as i32),
        ("set-flags-print-time", LogFlags::PRINT_TIME.bits() as i32),
        (
            "set-flags-print-datetime",
            LogFlags::PRINT_DATETIME.bits() as i32,
        ),
        ("set-flags-all-known", all_flags.bits() as i32),
        (
            "set-flags-unknown-bit",
            LogFlags::from_bits_retain(0x10).bits() as i32,
        ),
        (
            "set-flags-mixed-unknown",
            LogFlags::from_bits_retain(0x1234).bits() as i32,
        ),
        (
            "set-flags-negative-all-raw",
            LogFlags::from_bits_retain(u32::MAX).bits() as i32,
        ),
        ("log-once-first-state", 1),
        ("log-once-first-count", 1),
        ("log-once-first-level", LogLevel::Warning.as_ffmpeg_value()),
        ("log-once-second-state", 1),
        ("log-once-second-count", 2),
        ("log-once-second-level", LogLevel::Debug.as_ffmpeg_value()),
        ("log-once-preseed-state", 1),
        ("log-once-preseed-count", 3),
        ("log-once-preseed-level", LogLevel::Error.as_ffmpeg_value()),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();

    add_format_line2_int_rows(&mut rows);
    add_format_line_int_rows(&mut rows);
    rows.insert("custom-callback-above-level-count", 1);
    rows.insert(
        "custom-callback-above-level-level",
        LogLevel::Info.as_ffmpeg_value(),
    );
    rows.insert("custom-callback-raw-level-count", 1);
    rows.insert("custom-callback-raw-level-level", 23);
    rows.insert("custom-callback-null-count", 1);
    rows.insert(
        "custom-callback-null-level",
        LogLevel::Error.as_ffmpeg_value(),
    );
    rows.insert("custom-callback-repeat-count", 2);
    rows.insert(
        "custom-callback-repeat-level",
        LogLevel::Warning.as_ffmpeg_value(),
    );
    rows.insert("custom-callback-context-count", 1);
    rows.insert(
        "custom-callback-context-level",
        LogLevel::Warning.as_ffmpeg_value(),
    );
    rows.insert("custom-callback-context-repeat-count", 2);
    rows.insert(
        "custom-callback-context-repeat-level",
        LogLevel::Warning.as_ffmpeg_value(),
    );
    rows
}

fn upstream_log_test_script(build_dir: &str) -> String {
    format!(
        r#"set -e
build_dir={build_dir}
test -d "$build_dir" || {{ echo "missing FFmpeg FATE build dir: $build_dir" >&2; exit 66; }}
fate_file="$build_dir/src/tests/fate/libavutil.mak"
test -f "$fate_file" || {{ echo "missing FFmpeg libavutil FATE makefile: $fate_file" >&2; exit 66; }}
if grep -nE 'fate-.*log|libavutil/tests/log|tests/log' "$fate_file"; then
    echo "unexpected upstream FATE mapping for libavutil log test" >&2
    exit 67
fi
make -C "$build_dir" libavutil/tests/log
out_file="$(mktemp)"
err_file="$(mktemp)"
"$build_dir/libavutil/tests/log" >"$out_file" 2>"$err_file"
status=$?
if [ "$status" -ne 0 ]; then
    echo "libavutil/tests/log failed with status $status" >&2
    cat "$out_file"
    cat "$err_file" >&2
    exit "$status"
fi
if [ -s "$out_file" ]; then
    echo "libavutil/tests/log unexpectedly wrote stdout" >&2
    cat "$out_file"
    exit 68
fi
grep -q 'use_color: 0' "$err_file"
grep -q 'use_color: 1' "$err_file"
grep -q 'use_color: 256' "$err_file"
rm -f "$out_file" "$err_file"
"#
    )
}

fn expected_text_rows() -> BTreeMap<&'static str, String> {
    let mut rows = BTreeMap::new();
    let (plain, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line2-plain-line", escape_row_text(plain.bytes()));

    let (level, _) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line2-level-line", escape_row_text(level.bytes()));

    let (quiet_level, _) =
        rust_format_line2(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-quiet-level-line",
        escape_row_text(quiet_level.bytes()),
    );

    let (no_prefix, _) =
        rust_format_line2(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert(
        "format-line2-noprefix-line",
        escape_row_text(no_prefix.bytes()),
    );

    let (newline, _) =
        rust_format_line2(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-newline-line",
        escape_row_text(newline.bytes()),
    );

    let (carriage_return, _) =
        rust_format_line2(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-carriage-return-line",
        escape_row_text(carriage_return.bytes()),
    );

    let (small, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line2-small-line", escape_row_text(small.bytes()));

    let (size1, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line2-size1-line", escape_row_text(size1.bytes()));

    let (context_plain, _) =
        rust_format_line2_context(LogLevel::Warning, "ctxmsg", LogFlags::empty(), true, 128);
    rows.insert(
        "format-line2-context-plain-line",
        escape_row_text(context_plain.bytes()),
    );

    let (context_level, _) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-level-line",
        escape_row_text(context_level.bytes()),
    );

    let (context_quiet_level, _) =
        rust_format_line2_context(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-quiet-level-line",
        escape_row_text(context_quiet_level.bytes()),
    );

    let (context_no_prefix, _) = rust_format_line2_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line2-context-noprefix-line",
        escape_row_text(context_no_prefix.bytes()),
    );

    let (context_newline, _) =
        rust_format_line2_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-newline-line",
        escape_row_text(context_newline.bytes()),
    );

    let (context_carriage_return, _) =
        rust_format_line2_context(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-carriage-return-line",
        escape_row_text(context_carriage_return.bytes()),
    );

    let (time, _) = rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_TIME, true, 128);
    rows.insert("format-line2-time-line", escape_row_text(time.bytes()));

    let (datetime_level, _) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-datetime-level-line",
        escape_row_text(datetime_level.bytes()),
    );

    let (both_level, _) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-time-datetime-level-line",
        escape_row_text(both_level.bytes()),
    );

    let (context_time_level, _) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-time-level-line",
        escape_row_text(context_time_level.bytes()),
    );

    let timestamp = LogTimestamp::from_unix_micros(1_704_112_705_123_456);
    let context = AvLogContextPrefix::new("rustctx", "<ptr>");
    let default_time = rust_default_callback_line(LogFlags::PRINT_TIME, Some(timestamp));
    rows.insert(
        "default-callback-time-line",
        escape_row_text(normalize_default_callback_timestamp(&default_time).as_bytes()),
    );
    let default_time_level = rust_default_callback_line(
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-time-level-line",
        escape_row_text(normalize_default_callback_timestamp(&default_time_level).as_bytes()),
    );
    let default_datetime = rust_default_callback_line(LogFlags::PRINT_DATETIME, Some(timestamp));
    rows.insert(
        "default-callback-datetime-line",
        escape_row_text(normalize_default_callback_timestamp(&default_datetime).as_bytes()),
    );
    let default_datetime_level = rust_default_callback_line(
        LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-datetime-level-line",
        escape_row_text(normalize_default_callback_timestamp(&default_datetime_level).as_bytes()),
    );
    let default_both_level = rust_default_callback_line(
        LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-time-datetime-level-line",
        escape_row_text(normalize_default_callback_timestamp(&default_both_level).as_bytes()),
    );
    let default_context = rust_default_callback_context_line(LogFlags::empty(), None);
    rows.insert(
        "default-callback-context-line",
        escape_row_text(default_context.as_bytes()),
    );
    let default_context_level = rust_default_callback_context_line(LogFlags::PRINT_LEVEL, None);
    rows.insert(
        "default-callback-context-level-line",
        escape_row_text(default_context_level.as_bytes()),
    );
    let default_context_time_level = rust_default_callback_context_line(
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-context-time-level-line",
        escape_row_text(
            normalize_default_callback_timestamp(&default_context_time_level).as_bytes(),
        ),
    );
    let default_context_datetime_level = rust_default_callback_context_line(
        LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        Some(timestamp),
    );
    rows.insert(
        "default-callback-context-datetime-level-line",
        escape_row_text(
            normalize_default_callback_timestamp(&default_context_datetime_level).as_bytes(),
        ),
    );
    let fixed_local_time = LogRecord::new(LogLevel::Warning, "ignored", "local\n")
        .with_timestamp(timestamp)
        .format_default_callback_line_null_context_with_options(
            LogFormatOptions::new(LogFlags::PRINT_TIME)
                .with_default_callback_time_offset_seconds(2 * 3_600),
        );
    rows.insert(
        "default-callback-fixed-time-utcplus2-line",
        escape_row_text(fixed_local_time.as_bytes()),
    );
    let fixed_local_datetime_plus530 = LogRecord::new(LogLevel::Warning, "ignored", "local\n")
        .with_timestamp(timestamp)
        .format_default_callback_line_null_context_with_options(
            LogFormatOptions::new(LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL)
                .with_default_callback_time_offset_seconds(5 * 3_600 + 30 * 60),
        );
    rows.insert(
        "default-callback-fixed-datetime-utcplus530-level-line",
        escape_row_text(fixed_local_datetime_plus530.as_bytes()),
    );
    let fixed_local_datetime = LogRecord::new(LogLevel::Warning, "ignored", "local\n")
        .with_timestamp(LogTimestamp::from_unix_micros(1_704_070_923_456_789))
        .format_default_callback_line_null_context_with_options(
            LogFormatOptions::new(LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL)
                .with_default_callback_time_offset_seconds(-8 * 3_600),
        );
    rows.insert(
        "default-callback-fixed-datetime-utcminus8-level-line",
        escape_row_text(fixed_local_datetime.as_bytes()),
    );
    let mut threshold_logger = Logger::new_with_flags(LogLevel::Warning, LogFlags::PRINT_LEVEL);
    assert!(!threshold_logger.log(LogRecord::new(LogLevel::Info, "ignored", "hidden\n")));
    rows.insert(
        "default-callback-filter-info-at-warning-line",
        String::new(),
    );
    assert!(threshold_logger.log(LogRecord::new(LogLevel::Warning, "ignored", "shown\n")));
    rows.insert(
        "default-callback-filter-warning-at-warning-line",
        escape_row_text(
            threshold_logger
                .records()
                .last()
                .unwrap()
                .format_default_callback_line_null_context_with_flags(LogFlags::PRINT_LEVEL)
                .as_bytes(),
        ),
    );
    let mut raw_threshold_logger = Logger::new_with_raw_level(23, LogFlags::PRINT_LEVEL);
    assert!(raw_threshold_logger.log(LogRecord::new(LogLevel::Error, "ignored", "raw shown\n")));
    rows.insert(
        "default-callback-filter-error-at-raw23-line",
        escape_row_text(
            raw_threshold_logger
                .records()
                .last()
                .unwrap()
                .format_default_callback_line_null_context_with_flags(LogFlags::PRINT_LEVEL)
                .as_bytes(),
        ),
    );
    assert!(!raw_threshold_logger.log(LogRecord::new(
        LogLevel::Warning,
        "ignored",
        "raw hidden\n"
    )));
    rows.insert(
        "default-callback-filter-warning-at-raw23-line",
        String::new(),
    );
    let mut quiet_logger = Logger::new_with_flags(
        LogLevel::Quiet,
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
    );
    assert!(quiet_logger
        .log(LogRecord::new(LogLevel::Quiet, "ignored", "quiet\n").with_timestamp(timestamp)));
    rows.insert(
        "default-callback-quiet-at-quiet-line",
        escape_row_text(
            quiet_logger
                .records()
                .last()
                .unwrap()
                .format_default_callback_line_null_context_with_flags(
                    LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
                )
                .as_bytes(),
        ),
    );
    let quiet_context = quiet_logger
        .records()
        .last()
        .unwrap()
        .format_default_callback_line_context_with_flags(
            &context,
            LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        );
    rows.insert(
        "default-callback-quiet-context-at-quiet-line",
        escape_row_text(quiet_context.as_bytes()),
    );
    let default_repeat_skip = rust_default_callback_repeat_lines(LogFlags::SKIP_REPEATED);
    rows.insert(
        "default-callback-repeat-skip-line",
        escape_row_text(default_repeat_skip.as_bytes()),
    );
    let default_repeat_skip_level =
        rust_default_callback_repeat_lines(LogFlags::SKIP_REPEATED | LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-repeat-skip-level-line",
        escape_row_text(default_repeat_skip_level.as_bytes()),
    );
    let default_repeat_context_level = rust_default_callback_repeat_context_lines(
        LogFlags::SKIP_REPEATED | LogFlags::PRINT_LEVEL,
        &context,
    );
    rows.insert(
        "default-callback-repeat-context-level-line",
        escape_row_text(default_repeat_context_level.as_bytes()),
    );
    let default_repeat_context_switch_level =
        rust_default_callback_repeat_context_switch_lines(&context);
    rows.insert(
        "default-callback-repeat-context-switch-level-line",
        escape_row_text(default_repeat_context_switch_level.as_bytes()),
    );
    let default_repeat_no_skip = rust_default_callback_repeat_lines(LogFlags::empty());
    rows.insert(
        "default-callback-repeat-noskip-line",
        escape_row_text(default_repeat_no_skip.as_bytes()),
    );
    let default_prefix_continuation_plain =
        rust_default_callback_prefix_continuation_lines(LogFlags::empty(), None);
    rows.insert(
        "default-callback-prefix-continuation-plain-line",
        escape_row_text(default_prefix_continuation_plain.as_bytes()),
    );
    let default_prefix_continuation_level =
        rust_default_callback_prefix_continuation_lines(LogFlags::PRINT_LEVEL, None);
    rows.insert(
        "default-callback-prefix-continuation-level-line",
        escape_row_text(default_prefix_continuation_level.as_bytes()),
    );
    let default_prefix_continuation_context_level =
        rust_default_callback_prefix_continuation_lines(LogFlags::PRINT_LEVEL, Some(&context));
    rows.insert(
        "default-callback-prefix-continuation-context-level-line",
        escape_row_text(default_prefix_continuation_context_level.as_bytes()),
    );
    let default_prefix_carriage_return_level =
        rust_default_callback_prefix_carriage_return_lines(LogFlags::PRINT_LEVEL, None);
    rows.insert(
        "default-callback-prefix-carriage-return-level-line",
        escape_row_text(default_prefix_carriage_return_level.as_bytes()),
    );
    let default_prefix_carriage_return_context_level =
        rust_default_callback_prefix_carriage_return_lines(LogFlags::PRINT_LEVEL, Some(&context));
    rows.insert(
        "default-callback-prefix-carriage-return-context-level-line",
        escape_row_text(default_prefix_carriage_return_context_level.as_bytes()),
    );
    let default_color_warning =
        rust_default_callback_color_line(LogLevel::Warning, None, LogFlags::empty());
    rows.insert(
        "default-callback-color-warning-line",
        escape_row_text(default_color_warning.as_bytes()),
    );
    let default_color_warning_level =
        rust_default_callback_color_line(LogLevel::Warning, None, LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-color-warning-level-line",
        escape_row_text(default_color_warning_level.as_bytes()),
    );
    let default_color_warning_context_level =
        rust_default_callback_color_line(LogLevel::Warning, Some(&context), LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-color-warning-context-level-line",
        escape_row_text(default_color_warning_context_level.as_bytes()),
    );
    let quiet_color_options = LogFormatOptions::new(LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL)
        .with_color_mode(LogColorMode::Always);
    let default_color_quiet = LogRecord::new(LogLevel::Quiet, "ignored", "quiet\n")
        .format_default_callback_line_null_context_with_options(quiet_color_options);
    rows.insert(
        "default-callback-color-quiet-line",
        escape_row_text(default_color_quiet.as_bytes()),
    );
    let default_color_quiet_context = LogRecord::new(LogLevel::Quiet, "ignored", "quiet\n")
        .format_default_callback_line_context_with_options(&context, quiet_color_options);
    rows.insert(
        "default-callback-color-quiet-context-level-line",
        escape_row_text(default_color_quiet_context.as_bytes()),
    );
    let default_color_repeat_level = rust_default_callback_color_repeat_lines(
        LogFlags::SKIP_REPEATED | LogFlags::PRINT_LEVEL,
        None,
    );
    rows.insert(
        "default-callback-color-repeat-level-line",
        escape_row_text(default_color_repeat_level.as_bytes()),
    );
    let default_color_repeat_context_level = rust_default_callback_color_repeat_lines(
        LogFlags::SKIP_REPEATED | LogFlags::PRINT_LEVEL,
        Some(&context),
    );
    rows.insert(
        "default-callback-color-repeat-context-level-line",
        escape_row_text(default_color_repeat_context_level.as_bytes()),
    );
    let default_color_prefix_continuation_level =
        rust_default_callback_color_prefix_continuation_lines(LogFlags::PRINT_LEVEL, None);
    rows.insert(
        "default-callback-color-prefix-continuation-level-line",
        escape_row_text(default_color_prefix_continuation_level.as_bytes()),
    );
    let default_color_prefix_continuation_context_level =
        rust_default_callback_color_prefix_continuation_lines(
            LogFlags::PRINT_LEVEL,
            Some(&context),
        );
    rows.insert(
        "default-callback-color-prefix-continuation-context-level-line",
        escape_row_text(default_color_prefix_continuation_context_level.as_bytes()),
    );
    let default_color_prefix_carriage_return_level =
        rust_default_callback_color_prefix_carriage_return_lines(LogFlags::PRINT_LEVEL, None);
    rows.insert(
        "default-callback-color-prefix-carriage-return-level-line",
        escape_row_text(default_color_prefix_carriage_return_level.as_bytes()),
    );
    let default_color_prefix_carriage_return_context_level =
        rust_default_callback_color_prefix_carriage_return_lines(
            LogFlags::PRINT_LEVEL,
            Some(&context),
        );
    rows.insert(
        "default-callback-color-prefix-carriage-return-context-level-line",
        escape_row_text(default_color_prefix_carriage_return_context_level.as_bytes()),
    );
    let default_color_error =
        rust_default_callback_color_line(LogLevel::Error, None, LogFlags::empty());
    rows.insert(
        "default-callback-color-error-line",
        escape_row_text(default_color_error.as_bytes()),
    );
    let default_color_fatal_level =
        rust_default_callback_color_line(LogLevel::Fatal, None, LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-color-fatal-level-line",
        escape_row_text(default_color_fatal_level.as_bytes()),
    );
    let default_color_panic_level =
        rust_default_callback_color_line(LogLevel::Panic, None, LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-color-panic-level-line",
        escape_row_text(default_color_panic_level.as_bytes()),
    );
    let default_color_info =
        rust_default_callback_color_line(LogLevel::Info, None, LogFlags::empty());
    rows.insert(
        "default-callback-color-info-line",
        escape_row_text(default_color_info.as_bytes()),
    );
    let default_color_cache_after_nocolor = rust_default_callback_color_cache_after_nocolor_line();
    rows.insert(
        "default-callback-color-cache-after-nocolor-line",
        escape_row_text(default_color_cache_after_nocolor.as_bytes()),
    );
    let default_no_force_redirected =
        rust_default_callback_no_force_redirected_line(None, LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-no-force-redirected-warning-level-line",
        escape_row_text(default_no_force_redirected.as_bytes()),
    );
    let default_no_force_redirected_context =
        rust_default_callback_no_force_redirected_line(Some(&context), LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-no-force-redirected-context-level-line",
        escape_row_text(default_no_force_redirected_context.as_bytes()),
    );
    let default_no_force_tty = rust_default_callback_no_force_tty_line(None, LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-no-force-tty-warning-level-line",
        escape_row_text(default_no_force_tty.as_bytes()),
    );
    let default_no_force_tty_context =
        rust_default_callback_no_force_tty_line(Some(&context), LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-no-force-tty-context-level-line",
        escape_row_text(default_no_force_tty_context.as_bytes()),
    );
    let default_no_force_tty_term_256_context = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("xterm-256color")),
        LogLevel::Warning,
        Some(&context),
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-256color-context-warning-level-line",
        escape_row_text(default_no_force_tty_term_256_context.as_bytes()),
    );
    let default_no_force_tty_term_256_error = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("xterm-256color")),
        LogLevel::Error,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-256color-error-level-line",
        escape_row_text(default_no_force_tty_term_256_error.as_bytes()),
    );
    let default_no_force_tty_term_256_fatal = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("xterm-256color")),
        LogLevel::Fatal,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-256color-fatal-level-line",
        escape_row_text(default_no_force_tty_term_256_fatal.as_bytes()),
    );
    let default_no_force_tty_term_256_panic = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("xterm-256color")),
        LogLevel::Panic,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-256color-panic-level-line",
        escape_row_text(default_no_force_tty_term_256_panic.as_bytes()),
    );
    let default_no_force_tty_term_256_info = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("xterm-256color")),
        LogLevel::Info,
        None,
        LogFlags::empty(),
    );
    rows.insert(
        "default-callback-no-force-tty-term-256color-info-line",
        escape_row_text(default_no_force_tty_term_256_info.as_bytes()),
    );
    let default_no_force_tty_term_unset = rust_default_callback_no_force_tty_term_line(
        None,
        LogLevel::Warning,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-unset-warning-level-line",
        escape_row_text(default_no_force_tty_term_unset.as_bytes()),
    );
    let default_no_force_tty_term_dumb = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("dumb")),
        LogLevel::Warning,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-dumb-warning-level-line",
        escape_row_text(default_no_force_tty_term_dumb.as_bytes()),
    );
    let default_no_force_tty_term_dumb_context = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("dumb")),
        LogLevel::Warning,
        Some(&context),
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-dumb-context-warning-level-line",
        escape_row_text(default_no_force_tty_term_dumb_context.as_bytes()),
    );
    let default_no_force_tty_term_dumb_error = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("dumb")),
        LogLevel::Error,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-dumb-error-level-line",
        escape_row_text(default_no_force_tty_term_dumb_error.as_bytes()),
    );
    let default_no_force_tty_term_dumb_fatal = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("dumb")),
        LogLevel::Fatal,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-dumb-fatal-level-line",
        escape_row_text(default_no_force_tty_term_dumb_fatal.as_bytes()),
    );
    let default_no_force_tty_term_dumb_panic = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("dumb")),
        LogLevel::Panic,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-dumb-panic-level-line",
        escape_row_text(default_no_force_tty_term_dumb_panic.as_bytes()),
    );
    let default_no_force_tty_term_dumb_info = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("dumb")),
        LogLevel::Info,
        None,
        LogFlags::empty(),
    );
    rows.insert(
        "default-callback-no-force-tty-term-dumb-info-line",
        escape_row_text(default_no_force_tty_term_dumb_info.as_bytes()),
    );
    let default_no_force_tty_term_empty = rust_default_callback_no_force_tty_term_line(
        Some(OsStr::new("")),
        LogLevel::Warning,
        None,
        LogFlags::PRINT_LEVEL,
    );
    rows.insert(
        "default-callback-no-force-tty-term-empty-warning-level-line",
        escape_row_text(default_no_force_tty_term_empty.as_bytes()),
    );
    let default_nocolor_wins = rust_default_callback_nocolor_wins_line();
    rows.insert(
        "default-callback-nocolor-wins-warning-line",
        escape_row_text(default_nocolor_wins.as_bytes()),
    );
    let default_force_color_empty =
        rust_default_callback_color_line(LogLevel::Warning, None, LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-force-color-empty-warning-level-line",
        escape_row_text(default_force_color_empty.as_bytes()),
    );
    let default_force_color_zero =
        rust_default_callback_color_line(LogLevel::Warning, None, LogFlags::PRINT_LEVEL);
    rows.insert(
        "default-callback-force-color-zero-warning-level-line",
        escape_row_text(default_force_color_zero.as_bytes()),
    );
    let default_force_nocolor_empty = rust_default_callback_nocolor_wins_line();
    rows.insert(
        "default-callback-force-nocolor-empty-wins-warning-line",
        escape_row_text(default_force_nocolor_empty.as_bytes()),
    );
    let default_force_nocolor_zero = rust_default_callback_nocolor_wins_line();
    rows.insert(
        "default-callback-force-nocolor-zero-wins-warning-line",
        escape_row_text(default_force_nocolor_zero.as_bytes()),
    );
    let (custom_above_level_message, custom_above_level_item) =
        rust_custom_callback_above_level_text();
    rows.insert(
        "custom-callback-above-level-message",
        escape_row_text(custom_above_level_message.as_bytes()),
    );
    rows.insert(
        "custom-callback-above-level-item",
        escape_row_text(custom_above_level_item.as_bytes()),
    );
    let (custom_raw_level_message, custom_raw_level_item) = rust_custom_callback_raw_level_text();
    rows.insert(
        "custom-callback-raw-level-message",
        escape_row_text(custom_raw_level_message.as_bytes()),
    );
    rows.insert(
        "custom-callback-raw-level-item",
        escape_row_text(custom_raw_level_item.as_bytes()),
    );
    let (custom_null_message, custom_null_item) = rust_custom_callback_null_text();
    rows.insert(
        "custom-callback-null-message",
        escape_row_text(custom_null_message.as_bytes()),
    );
    rows.insert(
        "custom-callback-null-item",
        escape_row_text(custom_null_item.as_bytes()),
    );
    let (custom_repeat_message, custom_repeat_item) = rust_custom_callback_repeat_text();
    rows.insert(
        "custom-callback-repeat-message",
        escape_row_text(custom_repeat_message.as_bytes()),
    );
    rows.insert(
        "custom-callback-repeat-item",
        escape_row_text(custom_repeat_item.as_bytes()),
    );
    let (custom_context_message, custom_context_item) = rust_custom_callback_context_text(&context);
    rows.insert(
        "custom-callback-context-message",
        escape_row_text(custom_context_message.as_bytes()),
    );
    rows.insert(
        "custom-callback-context-item",
        escape_row_text(custom_context_item.as_bytes()),
    );
    let (custom_context_repeat_message, custom_context_repeat_item) =
        rust_custom_callback_context_repeat_text(&context);
    rows.insert(
        "custom-callback-context-repeat-message",
        escape_row_text(custom_context_repeat_message.as_bytes()),
    );
    rows.insert(
        "custom-callback-context-repeat-item",
        escape_row_text(custom_context_repeat_item.as_bytes()),
    );

    let (plain, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line-plain-line", escape_row_text(plain.bytes()));

    let (level, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-level-line", escape_row_text(level.bytes()));

    let (quiet_level, _) =
        rust_format_line(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-quiet-level-line",
        escape_row_text(quiet_level.bytes()),
    );

    let (no_prefix, _) =
        rust_format_line(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert(
        "format-line-noprefix-line",
        escape_row_text(no_prefix.bytes()),
    );

    let (newline, _) =
        rust_format_line(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-newline-line", escape_row_text(newline.bytes()));

    let (carriage_return, _) =
        rust_format_line(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-carriage-return-line",
        escape_row_text(carriage_return.bytes()),
    );

    let (small, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line-small-line", escape_row_text(small.bytes()));

    let (size1, _) = rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line-size1-line", escape_row_text(size1.bytes()));

    let (context_level, _) = rust_format_line_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-context-level-line",
        escape_row_text(context_level.bytes()),
    );

    let (context_quiet_level, _) =
        rust_format_line_context(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-quiet-level-line",
        escape_row_text(context_quiet_level.bytes()),
    );

    let (context_no_prefix, _) = rust_format_line_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line-context-noprefix-line",
        escape_row_text(context_no_prefix.bytes()),
    );

    let (context_newline, _) =
        rust_format_line_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-newline-line",
        escape_row_text(context_newline.bytes()),
    );

    let (context_carriage_return, _) =
        rust_format_line_context(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-carriage-return-line",
        escape_row_text(context_carriage_return.bytes()),
    );

    let (time_level, _) = rust_format_line(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-time-level-line",
        escape_row_text(time_level.bytes()),
    );

    rows
}

fn add_format_line2_int_rows(rows: &mut BTreeMap<&'static str, i32>) {
    let (plain, plain_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line2-plain-ret", usize_to_i32(plain.full_len()));
    rows.insert("format-line2-plain-prefix", bool_to_i32(plain_prefix));
    rows.insert("format-line2-plain-len", usize_to_i32(plain.bytes().len()));

    let (level, level_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line2-level-ret", usize_to_i32(level.full_len()));
    rows.insert("format-line2-level-prefix", bool_to_i32(level_prefix));
    rows.insert("format-line2-level-len", usize_to_i32(level.bytes().len()));

    let (quiet_level, quiet_level_prefix) =
        rust_format_line2(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-quiet-level-ret",
        usize_to_i32(quiet_level.full_len()),
    );
    rows.insert(
        "format-line2-quiet-level-prefix",
        bool_to_i32(quiet_level_prefix),
    );
    rows.insert(
        "format-line2-quiet-level-len",
        usize_to_i32(quiet_level.bytes().len()),
    );

    let (no_prefix, no_prefix_state) =
        rust_format_line2(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert(
        "format-line2-noprefix-ret",
        usize_to_i32(no_prefix.full_len()),
    );
    rows.insert("format-line2-noprefix-prefix", bool_to_i32(no_prefix_state));
    rows.insert(
        "format-line2-noprefix-len",
        usize_to_i32(no_prefix.bytes().len()),
    );

    let (newline, newline_prefix) =
        rust_format_line2(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line2-newline-ret", usize_to_i32(newline.full_len()));
    rows.insert("format-line2-newline-prefix", bool_to_i32(newline_prefix));
    rows.insert(
        "format-line2-newline-len",
        usize_to_i32(newline.bytes().len()),
    );

    let (carriage_return, carriage_return_prefix) =
        rust_format_line2(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-carriage-return-ret",
        usize_to_i32(carriage_return.full_len()),
    );
    rows.insert(
        "format-line2-carriage-return-prefix",
        bool_to_i32(carriage_return_prefix),
    );
    rows.insert(
        "format-line2-carriage-return-len",
        usize_to_i32(carriage_return.bytes().len()),
    );

    let (small, small_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line2-small-ret", usize_to_i32(small.full_len()));
    rows.insert("format-line2-small-prefix", bool_to_i32(small_prefix));
    rows.insert("format-line2-small-len", usize_to_i32(small.bytes().len()));

    let (null_zero, null_zero_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 0);
    rows.insert(
        "format-line2-nullzero-ret",
        usize_to_i32(null_zero.full_len()),
    );
    rows.insert(
        "format-line2-nullzero-prefix",
        bool_to_i32(null_zero_prefix),
    );

    let (size1, size1_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line2-size1-ret", usize_to_i32(size1.full_len()));
    rows.insert("format-line2-size1-prefix", bool_to_i32(size1_prefix));
    rows.insert("format-line2-size1-len", usize_to_i32(size1.bytes().len()));

    let (_, context_plain_prefix) =
        rust_format_line2_context(LogLevel::Warning, "ctxmsg", LogFlags::empty(), true, 128);
    rows.insert(
        "format-line2-context-plain-prefix",
        bool_to_i32(context_plain_prefix),
    );

    let (_, context_level_prefix) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-level-prefix",
        bool_to_i32(context_level_prefix),
    );

    let (_, context_quiet_level_prefix) =
        rust_format_line2_context(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-quiet-level-prefix",
        bool_to_i32(context_quiet_level_prefix),
    );

    let (_, context_no_prefix_state) = rust_format_line2_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line2-context-noprefix-prefix",
        bool_to_i32(context_no_prefix_state),
    );

    let (_, context_newline_prefix) =
        rust_format_line2_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-newline-prefix",
        bool_to_i32(context_newline_prefix),
    );

    let (_, context_carriage_return_prefix) =
        rust_format_line2_context(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line2-context-carriage-return-prefix",
        bool_to_i32(context_carriage_return_prefix),
    );

    let (time, time_prefix) =
        rust_format_line2(LogLevel::Warning, "plain", LogFlags::PRINT_TIME, true, 128);
    rows.insert("format-line2-time-ret", usize_to_i32(time.full_len()));
    rows.insert("format-line2-time-prefix", bool_to_i32(time_prefix));
    rows.insert("format-line2-time-len", usize_to_i32(time.bytes().len()));

    let (datetime_level, datetime_level_prefix) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-datetime-level-ret",
        usize_to_i32(datetime_level.full_len()),
    );
    rows.insert(
        "format-line2-datetime-level-prefix",
        bool_to_i32(datetime_level_prefix),
    );
    rows.insert(
        "format-line2-datetime-level-len",
        usize_to_i32(datetime_level.bytes().len()),
    );

    let (both_level, both_level_prefix) = rust_format_line2(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_DATETIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-time-datetime-level-ret",
        usize_to_i32(both_level.full_len()),
    );
    rows.insert(
        "format-line2-time-datetime-level-prefix",
        bool_to_i32(both_level_prefix),
    );
    rows.insert(
        "format-line2-time-datetime-level-len",
        usize_to_i32(both_level.bytes().len()),
    );

    let (_, context_time_level_prefix) = rust_format_line2_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line2-context-time-level-prefix",
        bool_to_i32(context_time_level_prefix),
    );
}

fn add_format_line_int_rows(rows: &mut BTreeMap<&'static str, i32>) {
    let (plain, plain_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::empty(), true, 128);
    rows.insert("format-line-plain-prefix", bool_to_i32(plain_prefix));
    rows.insert("format-line-plain-len", usize_to_i32(plain.bytes().len()));

    let (level, level_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-level-prefix", bool_to_i32(level_prefix));
    rows.insert("format-line-level-len", usize_to_i32(level.bytes().len()));

    let (quiet_level, quiet_level_prefix) =
        rust_format_line(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-quiet-level-prefix",
        bool_to_i32(quiet_level_prefix),
    );
    rows.insert(
        "format-line-quiet-level-len",
        usize_to_i32(quiet_level.bytes().len()),
    );

    let (no_prefix, no_prefix_state) =
        rust_format_line(LogLevel::Error, "after", LogFlags::PRINT_LEVEL, false, 128);
    rows.insert("format-line-noprefix-prefix", bool_to_i32(no_prefix_state));
    rows.insert(
        "format-line-noprefix-len",
        usize_to_i32(no_prefix.bytes().len()),
    );

    let (newline, newline_prefix) =
        rust_format_line(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert("format-line-newline-prefix", bool_to_i32(newline_prefix));
    rows.insert(
        "format-line-newline-len",
        usize_to_i32(newline.bytes().len()),
    );

    let (carriage_return, carriage_return_prefix) =
        rust_format_line(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-carriage-return-prefix",
        bool_to_i32(carriage_return_prefix),
    );
    rows.insert(
        "format-line-carriage-return-len",
        usize_to_i32(carriage_return.bytes().len()),
    );

    let (small, small_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 8);
    rows.insert("format-line-small-prefix", bool_to_i32(small_prefix));
    rows.insert("format-line-small-len", usize_to_i32(small.bytes().len()));

    let (size1, size1_prefix) =
        rust_format_line(LogLevel::Warning, "plain", LogFlags::PRINT_LEVEL, true, 1);
    rows.insert("format-line-size1-prefix", bool_to_i32(size1_prefix));
    rows.insert("format-line-size1-len", usize_to_i32(size1.bytes().len()));

    let (_, context_level_prefix) = rust_format_line_context(
        LogLevel::Warning,
        "ctxmsg",
        LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-context-level-prefix",
        bool_to_i32(context_level_prefix),
    );

    let (_, context_quiet_level_prefix) =
        rust_format_line_context(LogLevel::Quiet, "quiet", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-quiet-level-prefix",
        bool_to_i32(context_quiet_level_prefix),
    );

    let (_, context_no_prefix_state) = rust_format_line_context(
        LogLevel::Warning,
        "nopfx",
        LogFlags::PRINT_LEVEL,
        false,
        128,
    );
    rows.insert(
        "format-line-context-noprefix-prefix",
        bool_to_i32(context_no_prefix_state),
    );

    let (_, context_newline_prefix) =
        rust_format_line_context(LogLevel::Info, "withnl\n", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-newline-prefix",
        bool_to_i32(context_newline_prefix),
    );

    let (_, context_carriage_return_prefix) =
        rust_format_line_context(LogLevel::Info, "withcr\r", LogFlags::PRINT_LEVEL, true, 128);
    rows.insert(
        "format-line-context-carriage-return-prefix",
        bool_to_i32(context_carriage_return_prefix),
    );

    let (time_level, time_level_prefix) = rust_format_line(
        LogLevel::Warning,
        "plain",
        LogFlags::PRINT_TIME | LogFlags::PRINT_LEVEL,
        true,
        128,
    );
    rows.insert(
        "format-line-time-level-prefix",
        bool_to_i32(time_level_prefix),
    );
    rows.insert(
        "format-line-time-level-len",
        usize_to_i32(time_level.bytes().len()),
    );
}

fn rust_format_line(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine, bool) {
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line_null_context(flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line model should support this flag shape");
    (line, prefix)
}

fn rust_format_line_context(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine, bool) {
    let context = AvLogContextPrefix::new("rustctx", "<ptr>");
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line_context(&context, flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line context model should support this flag shape");
    (line, prefix)
}

fn rust_format_line2(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine2, bool) {
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line2_null_context(flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line2 model should support this flag shape");
    (line, prefix)
}

fn rust_format_line2_context(
    level: LogLevel,
    message: &str,
    flags: LogFlags,
    initial_prefix: bool,
    line_size: usize,
) -> (AvLogFormatLine2, bool) {
    let context = AvLogContextPrefix::new("rustctx", "<ptr>");
    let mut prefix = initial_prefix;
    let line = LogRecord::new(level, "ignored", message)
        .format_av_log_line2_context(&context, flags, &mut prefix, line_size)
        .expect("bounded av_log_format_line2 context model should support this flag shape");
    (line, prefix)
}

fn rust_default_callback_line(flags: LogFlags, timestamp: Option<LogTimestamp>) -> String {
    let mut record = LogRecord::new(LogLevel::Warning, "ignored", "plain\n");
    if let Some(timestamp) = timestamp {
        record = record.with_timestamp(timestamp);
    }
    record.format_default_callback_line_null_context_with_flags(flags)
}

fn rust_default_callback_context_line(flags: LogFlags, timestamp: Option<LogTimestamp>) -> String {
    let context = AvLogContextPrefix::new("rustctx", "<ptr>");
    let mut record = LogRecord::new(LogLevel::Warning, "ignored", "ctxmsg\n");
    if let Some(timestamp) = timestamp {
        record = record.with_timestamp(timestamp);
    }
    record.format_default_callback_line_context_with_flags(&context, flags)
}

fn rust_default_callback_repeat_lines(flags: LogFlags) -> String {
    rust_default_callback_repeat_lines_with_options(flags, None, LogFormatOptions::new(flags))
}

fn rust_default_callback_repeat_context_lines(
    flags: LogFlags,
    context: &AvLogContextPrefix,
) -> String {
    rust_default_callback_repeat_lines_with_options(
        flags,
        Some(context),
        LogFormatOptions::new(flags),
    )
}

fn rust_default_callback_color_repeat_lines(
    flags: LogFlags,
    context: Option<&AvLogContextPrefix>,
) -> String {
    rust_default_callback_repeat_lines_with_options(
        flags,
        context,
        LogFormatOptions::new(flags).with_color_mode(LogColorMode::Always),
    )
}

fn rust_default_callback_repeat_context_switch_lines(context: &AvLogContextPrefix) -> String {
    let flags = LogFlags::SKIP_REPEATED | LogFlags::PRINT_LEVEL;
    let mut logger = Logger::new_with_flags(LogLevel::Trace, flags);
    assert!(logger.log(LogRecord::new(LogLevel::Warning, "ctx@one", "repeat\n")));
    assert!(logger.log(LogRecord::new(LogLevel::Warning, "ctx@two", "repeat\n")));
    assert!(logger.log(LogRecord::new(LogLevel::Warning, "ctx@one", "repeat\n")));
    assert!(logger.log(LogRecord::new(LogLevel::Error, "ctx@one", "next\n")));

    logger
        .records()
        .iter()
        .map(|record| {
            record.format_default_callback_line_context_with_options(
                context,
                LogFormatOptions::new(flags),
            )
        })
        .collect()
}

fn rust_default_callback_repeat_lines_with_options(
    flags: LogFlags,
    context: Option<&AvLogContextPrefix>,
    options: LogFormatOptions,
) -> String {
    let mut logger = Logger::new_with_flags(LogLevel::Trace, flags);
    let repeated = LogRecord::new(LogLevel::Warning, "ignored", "repeat\n");
    assert!(logger.log(repeated.clone()));
    assert!(logger.log(repeated.clone()));
    assert!(logger.log(repeated));
    assert!(logger.log(LogRecord::new(LogLevel::Error, "ignored", "next\n")));
    logger
        .records()
        .iter()
        .map(|record| match context {
            Some(context) => {
                record.format_default_callback_line_context_with_options(context, options)
            }
            None => record.format_default_callback_line_null_context_with_options(options),
        })
        .collect()
}

fn rust_default_callback_prefix_continuation_lines(
    flags: LogFlags,
    context: Option<&AvLogContextPrefix>,
) -> String {
    let mut state = DefaultCallbackPrefixState::new();
    ["part", "tail\n", "next\n"]
        .into_iter()
        .map(|message| {
            let record = LogRecord::new(LogLevel::Warning, "ignored", message);
            match context {
                Some(context) => record
                    .format_default_callback_line_context_with_state(context, flags, &mut state),
                None => {
                    record.format_default_callback_line_null_context_with_state(flags, &mut state)
                }
            }
        })
        .collect()
}

fn rust_default_callback_prefix_carriage_return_lines(
    flags: LogFlags,
    context: Option<&AvLogContextPrefix>,
) -> String {
    let mut state = DefaultCallbackPrefixState::new();
    ["progress\r", "done\n"]
        .into_iter()
        .map(|message| {
            let record = LogRecord::new(LogLevel::Warning, "ignored", message);
            match context {
                Some(context) => record
                    .format_default_callback_line_context_with_state(context, flags, &mut state),
                None => {
                    record.format_default_callback_line_null_context_with_state(flags, &mut state)
                }
            }
        })
        .collect()
}

fn rust_default_callback_color_line(
    level: LogLevel,
    context: Option<&AvLogContextPrefix>,
    flags: LogFlags,
) -> String {
    let options = LogFormatOptions::new(flags).with_color_mode(LogColorMode::Always);
    let record = LogRecord::new(level, "ignored", "plain\n");
    match context {
        Some(context) => record.format_default_callback_line_context_with_options(context, options),
        None => record.format_default_callback_line_null_context_with_options(options),
    }
}

fn rust_default_callback_color_prefix_continuation_lines(
    flags: LogFlags,
    context: Option<&AvLogContextPrefix>,
) -> String {
    let mut state = DefaultCallbackPrefixState::new();
    let options = LogFormatOptions::new(flags).with_color_mode(LogColorMode::Always);
    ["part", "tail\n", "next\n"]
        .into_iter()
        .map(|message| {
            let record = LogRecord::new(LogLevel::Warning, "ignored", message);
            match context {
                Some(context) => record
                    .format_default_callback_line_context_with_options_and_state(
                        context, options, &mut state,
                    ),
                None => record.format_default_callback_line_null_context_with_options_and_state(
                    options, &mut state,
                ),
            }
        })
        .collect()
}

fn rust_default_callback_color_prefix_carriage_return_lines(
    flags: LogFlags,
    context: Option<&AvLogContextPrefix>,
) -> String {
    let mut state = DefaultCallbackPrefixState::new();
    let options = LogFormatOptions::new(flags).with_color_mode(LogColorMode::Always);
    ["progress\r", "done\n"]
        .into_iter()
        .map(|message| {
            let record = LogRecord::new(LogLevel::Warning, "ignored", message);
            match context {
                Some(context) => record
                    .format_default_callback_line_context_with_options_and_state(
                        context, options, &mut state,
                    ),
                None => record.format_default_callback_line_null_context_with_options_and_state(
                    options, &mut state,
                ),
            }
        })
        .collect()
}

fn rust_default_callback_color_cache_after_nocolor_line() -> String {
    let mut color_state = DefaultCallbackColorState::new();
    let first_options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
        .with_default_callback_color_state_and_resolver(&mut color_state, || LogColorMode::Never);
    assert_eq!(first_options.color_mode(), LogColorMode::Never);

    let cached_options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
        .with_default_callback_color_state_and_resolver(&mut color_state, || LogColorMode::Always);
    assert_eq!(cached_options.color_mode(), LogColorMode::Never);
    LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
        .format_default_callback_line_null_context_with_options(cached_options)
}

fn rust_default_callback_no_force_redirected_line(
    context: Option<&AvLogContextPrefix>,
    flags: LogFlags,
) -> String {
    let mut color_state = DefaultCallbackColorState::new();
    let options = LogFormatOptions::new(flags)
        .with_default_callback_color_state_and_resolver(&mut color_state, || {
            LogColorMode::from_ffmpeg_env_vars_and_stderr(|_| false, false)
        });
    assert_eq!(options.color_mode(), LogColorMode::Never);
    assert_eq!(color_state.cached_mode(), Some(LogColorMode::Never));
    let record = LogRecord::new(LogLevel::Warning, "ignored", "plain\n");
    match context {
        Some(context) => record.format_default_callback_line_context_with_options(context, options),
        None => record.format_default_callback_line_null_context_with_options(options),
    }
}

fn rust_default_callback_no_force_tty_line(
    context: Option<&AvLogContextPrefix>,
    flags: LogFlags,
) -> String {
    let mut color_state = DefaultCallbackColorState::new();
    let options = LogFormatOptions::new(flags)
        .with_default_callback_color_state_and_resolver(&mut color_state, || {
            LogColorMode::from_ffmpeg_env_vars_and_stderr(|_| false, true)
        });
    assert_eq!(options.color_mode(), LogColorMode::Always);
    assert_eq!(color_state.cached_mode(), Some(LogColorMode::Always));
    let record = LogRecord::new(LogLevel::Warning, "ignored", "plain\n");
    match context {
        Some(context) => record.format_default_callback_line_context_with_options(context, options),
        None => record.format_default_callback_line_null_context_with_options(options),
    }
}

fn rust_default_callback_no_force_tty_term_line(
    term: Option<&OsStr>,
    level: LogLevel,
    context: Option<&AvLogContextPrefix>,
    flags: LogFlags,
) -> String {
    let mut color_state = DefaultCallbackColorState::new();
    let options = LogFormatOptions::new(flags)
        .with_default_callback_color_state_and_resolver(&mut color_state, || {
            LogColorMode::from_ffmpeg_env_vars_stderr_and_term(|_| false, true, term)
        });
    let expected_mode = match term {
        None => LogColorMode::Never,
        Some(term) if term.to_string_lossy().contains("256color") => LogColorMode::Always,
        Some(_) => LogColorMode::Basic,
    };
    assert_eq!(options.color_mode(), expected_mode);
    assert_eq!(color_state.cached_mode(), Some(expected_mode));
    let record = LogRecord::new(level, "ignored", "plain\n");
    match context {
        Some(context) => record.format_default_callback_line_context_with_options(context, options),
        None => record.format_default_callback_line_null_context_with_options(options),
    }
}

fn rust_default_callback_nocolor_wins_line() -> String {
    let mut color_state = DefaultCallbackColorState::new();
    let options = LogFormatOptions::new(LogFlags::PRINT_LEVEL)
        .with_default_callback_color_state_and_resolver(&mut color_state, || {
            LogColorMode::from_ffmpeg_env_vars(|name| {
                name == avutil::AV_LOG_FORCE_NOCOLOR_ENV || name == avutil::AV_LOG_FORCE_COLOR_ENV
            })
        });
    assert_eq!(options.color_mode(), LogColorMode::Never);
    LogRecord::new(LogLevel::Warning, "ignored", "plain\n")
        .format_default_callback_line_null_context_with_options(options)
}

fn rust_custom_callback_above_level_text() -> (String, String) {
    let mut logger = Logger::new_with_flags(LogLevel::Warning, LogFlags::PRINT_LEVEL);
    let mut seen = Vec::new();
    logger.log_custom_callback(LogRecord::new(LogLevel::Info, "", "hidden"), |record| {
        seen.push((record.message().to_owned(), "<none>".to_owned()))
    });
    assert_eq!(seen.len(), 1);
    seen.remove(0)
}

fn rust_custom_callback_raw_level_text() -> (String, String) {
    let mut logger = Logger::new_with_flags(LogLevel::Warning, LogFlags::PRINT_LEVEL);
    let mut seen = Vec::new();
    logger.log_custom_callback(
        LogRecord::new(LogLevel::Warning, "", "rawlevel").with_raw_level(23),
        |record| {
            assert_eq!(record.raw_level(), 23);
            assert_eq!(record.known_level(), None);
            seen.push((record.message().to_owned(), "<none>".to_owned()));
        },
    );
    assert_eq!(seen.len(), 1);
    seen.remove(0)
}

fn rust_custom_callback_null_text() -> (String, String) {
    let mut logger = Logger::new_with_flags(LogLevel::Warning, LogFlags::PRINT_LEVEL);
    let mut seen = Vec::new();
    logger.log_custom_callback(LogRecord::new(LogLevel::Error, "", "raw:5\n"), |record| {
        seen.push((record.message().to_owned(), "<none>".to_owned()))
    });
    assert_eq!(seen.len(), 1);
    seen.remove(0)
}

fn rust_custom_callback_repeat_text() -> (String, String) {
    let mut flags = LogFlags::PRINT_LEVEL;
    flags.insert(LogFlags::SKIP_REPEATED);
    let mut logger = Logger::new_with_flags(LogLevel::Warning, flags);
    let mut seen = Vec::new();
    for _ in 0..2 {
        logger.log_custom_callback(LogRecord::new(LogLevel::Warning, "", "repeat"), |record| {
            seen.push((record.message().to_owned(), "<none>".to_owned()))
        });
    }
    assert_eq!(seen.len(), 2);
    seen.pop().expect("repeat callback row")
}

fn rust_custom_callback_context_text(context: &AvLogContextPrefix) -> (String, String) {
    let mut logger = Logger::new_with_flags(LogLevel::Warning, LogFlags::PRINT_LEVEL);
    let mut seen = Vec::new();
    logger.log_custom_callback(
        LogRecord::new(LogLevel::Warning, context.item_name(), "ctx:3"),
        |record| seen.push((record.message().to_owned(), record.target().to_owned())),
    );
    assert_eq!(seen.len(), 1);
    seen.remove(0)
}

fn rust_custom_callback_context_repeat_text(context: &AvLogContextPrefix) -> (String, String) {
    let mut flags = LogFlags::PRINT_LEVEL;
    flags.insert(LogFlags::SKIP_REPEATED);
    let mut logger = Logger::new_with_flags(LogLevel::Warning, flags);
    let mut seen = Vec::new();
    for _ in 0..2 {
        logger.log_custom_callback(
            LogRecord::new(LogLevel::Warning, context.item_name(), "ctxrepeat"),
            |record| seen.push((record.message().to_owned(), record.target().to_owned())),
        );
    }
    assert_eq!(seen.len(), 2);
    seen.pop().expect("context repeat callback row")
}

fn normalize_default_callback_timestamp(line: &str) -> String {
    if is_default_datetime_prefix(line) {
        format!("<datetime> {}", &line[24..])
    } else if is_default_time_prefix(line) {
        format!("<time> {}", &line[13..])
    } else {
        line.to_owned()
    }
}

fn is_default_time_prefix(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 13
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5] == b':'
        && bytes[6].is_ascii_digit()
        && bytes[7].is_ascii_digit()
        && bytes[8] == b'.'
        && bytes[9].is_ascii_digit()
        && bytes[10].is_ascii_digit()
        && bytes[11].is_ascii_digit()
        && bytes[12] == b' '
}

fn is_default_datetime_prefix(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 24
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[7] == b'-'
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
        && bytes[10] == b' '
        && bytes[11].is_ascii_digit()
        && bytes[12].is_ascii_digit()
        && bytes[13] == b':'
        && bytes[14].is_ascii_digit()
        && bytes[15].is_ascii_digit()
        && bytes[16] == b':'
        && bytes[17].is_ascii_digit()
        && bytes[18].is_ascii_digit()
        && bytes[19] == b'.'
        && bytes[20].is_ascii_digit()
        && bytes[21].is_ascii_digit()
        && bytes[22].is_ascii_digit()
        && bytes[23] == b' '
}

fn bool_to_i32(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).expect("oracle row should fit in i32")
}

fn escape_row_text(bytes: &[u8]) -> String {
    let mut escaped = String::new();
    for &byte in bytes {
        match byte {
            b'\n' => escaped.push_str("\\n"),
            b'\r' => escaped.push_str("\\r"),
            b'\t' => escaped.push_str("\\t"),
            b'\\' => escaped.push_str("\\\\"),
            b'|' => escaped.push_str("\\x7c"),
            0x20..=0x7e => escaped.push(char::from(byte)),
            _ => escaped.push_str(&format!("\\x{byte:02x}")),
        }
    }
    escaped
}

fn parse_oracle_output(stdout: &str) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let mut parts = line.splitn(2, '|');
        let name = parts.next().expect("row name").to_string();
        let value = parts
            .next()
            .unwrap_or_else(|| panic!("missing value in oracle row `{line}`"))
            .to_string();
        assert!(
            rows.insert(name, value).is_none(),
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
            "gcc -I {} {} {} -lm -pthread -ldl -lutil -o {} && {} && {} --plain && {} --tty && {} --tty-term-unset && {} --tty-term-dumb && {} --tty-term-empty && {} --color && {} --nocolor && {} --force-color-empty && {} --force-color-zero && {} --force-nocolor-empty && {} --force-nocolor-zero",
            shell_quote(&to_wsl_path(include_dir)),
            shell_quote(&to_wsl_path(source)),
            shell_quote(&to_wsl_path(libavutil)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable)),
            shell_quote(&to_wsl_path(executable))
        );
        Command::new("wsl")
            .args(["-d", "Ubuntu", "--exec", "bash", "-lc", &script])
            .output()
            .expect("run WSL libavutil logging oracle")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gcc -I {} {} {} -lm -pthread -ldl -lutil -o {} && {} && {} --plain && {} --tty && {} --tty-term-unset && {} --tty-term-dumb && {} --tty-term-empty && {} --color && {} --nocolor && {} --force-color-empty && {} --force-color-zero && {} --force-nocolor-empty && {} --force-nocolor-zero",
                shell_quote(&include_dir.display().to_string()),
                shell_quote(&source.display().to_string()),
                shell_quote(&libavutil.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string()),
                shell_quote(&executable.display().to_string())
            ))
            .output()
            .expect("run libavutil logging oracle")
    };

    assert!(
        output.status.success(),
        "libavutil logging oracle failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8")
}

fn oracle_c_source() -> &'static str {
    r#"#include <stdarg.h>
#include <ctype.h>
#include <pty.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <termios.h>
#include <time.h>
#include <unistd.h>
#include <libavutil/log.h>
#include <libavutil/version.h>

#define ROW(name, value) printf("%s|%d\n", name, (int)(value))

static void ROW_STR(const char *name, const char *value) {
    printf("%s|", name);
    if (!value) {
        printf("<null>\n");
        return;
    }
    for (const unsigned char *p = (const unsigned char *)value; *p; p++) {
        if (*p == '\n')
            printf("\\n");
        else if (*p == '\r')
            printf("\\r");
        else if (*p == '\t')
            printf("\\t");
        else if (*p == '\\')
            printf("\\\\");
        else if (*p == '|')
            printf("\\x7c");
        else if (*p < 32 || *p > 126)
            printf("\\x%02x", *p);
        else
            putchar(*p);
    }
    putchar('\n');
}

static int64_t fixed_av_gettime_value = 0;

int64_t av_gettime(void) {
    return fixed_av_gettime_value;
}

static void ROW_STR_NORMALIZED_CONTEXT(const char *name, const char *value) {
    char normalized[512];
    const char *marker = value ? strstr(value, " @ 0x") : NULL;
    if (!marker) {
        ROW_STR(name, value);
        return;
    }
    const char *ptr_end = marker + 3;
    while (*ptr_end && *ptr_end != ']')
        ptr_end++;
    size_t head_len = (size_t)(marker - value);
    snprintf(normalized, sizeof(normalized), "%.*s @ <ptr>%s",
             (int)head_len, value, ptr_end);
    ROW_STR(name, normalized);
}

static int has_default_time_prefix(const char *value) {
    return value && strlen(value) >= 13 &&
           isdigit((unsigned char)value[0]) &&
           isdigit((unsigned char)value[1]) &&
           value[2] == ':' &&
           isdigit((unsigned char)value[3]) &&
           isdigit((unsigned char)value[4]) &&
           value[5] == ':' &&
           isdigit((unsigned char)value[6]) &&
           isdigit((unsigned char)value[7]) &&
           value[8] == '.' &&
           isdigit((unsigned char)value[9]) &&
           isdigit((unsigned char)value[10]) &&
           isdigit((unsigned char)value[11]) &&
           value[12] == ' ';
}

static int has_default_datetime_prefix(const char *value) {
    return value && strlen(value) >= 24 &&
           isdigit((unsigned char)value[0]) &&
           isdigit((unsigned char)value[1]) &&
           isdigit((unsigned char)value[2]) &&
           isdigit((unsigned char)value[3]) &&
           value[4] == '-' &&
           isdigit((unsigned char)value[5]) &&
           isdigit((unsigned char)value[6]) &&
           value[7] == '-' &&
           isdigit((unsigned char)value[8]) &&
           isdigit((unsigned char)value[9]) &&
           value[10] == ' ' &&
           isdigit((unsigned char)value[11]) &&
           isdigit((unsigned char)value[12]) &&
           value[13] == ':' &&
           isdigit((unsigned char)value[14]) &&
           isdigit((unsigned char)value[15]) &&
           value[16] == ':' &&
           isdigit((unsigned char)value[17]) &&
           isdigit((unsigned char)value[18]) &&
           value[19] == '.' &&
           isdigit((unsigned char)value[20]) &&
           isdigit((unsigned char)value[21]) &&
           isdigit((unsigned char)value[22]) &&
           value[23] == ' ';
}

static void normalize_context_pointer(const char *value, char *normalized, size_t normalized_size) {
    const char *input = value ? value : "";
    const char *cursor = input;
    size_t out_len = 0;
    if (normalized_size == 0)
        return;

    while (*cursor && out_len + 1 < normalized_size) {
        const char *marker = strstr(cursor, " @ 0x");
        if (!marker) {
            size_t tail_len = strlen(cursor);
            size_t copy_len = tail_len;
            if (copy_len > normalized_size - out_len - 1)
                copy_len = normalized_size - out_len - 1;
            memcpy(normalized + out_len, cursor, copy_len);
            out_len += copy_len;
            break;
        }

        size_t head_len = (size_t)(marker - cursor);
        if (head_len > normalized_size - out_len - 1)
            head_len = normalized_size - out_len - 1;
        memcpy(normalized + out_len, cursor, head_len);
        out_len += head_len;

        const char replacement[] = " @ <ptr>";
        size_t replacement_len = sizeof(replacement) - 1;
        if (replacement_len > normalized_size - out_len - 1)
            replacement_len = normalized_size - out_len - 1;
        memcpy(normalized + out_len, replacement, replacement_len);
        out_len += replacement_len;

        cursor = marker + 3;
        while (*cursor && *cursor != ']')
            cursor++;
    }

    normalized[out_len] = '\0';
}

static void ROW_STR_NORMALIZED_DEFAULT_CALLBACK(const char *name, const char *value) {
    char timestamped[1024];
    char normalized[1024];
    if (has_default_datetime_prefix(value)) {
        snprintf(timestamped, sizeof(timestamped), "<datetime> %s", value + 24);
    } else if (has_default_time_prefix(value)) {
        snprintf(timestamped, sizeof(timestamped), "<time> %s", value + 13);
    } else {
        snprintf(timestamped, sizeof(timestamped), "%s", value ? value : "");
    }
    normalize_context_pointer(timestamped, normalized, sizeof(normalized));
    ROW_STR(name, normalized);
}

typedef struct TestLogContext {
    const AVClass *av_class;
} TestLogContext;

static const AVClass test_log_class = {
    .class_name = "rustctx",
    .item_name = av_default_item_name,
    .version = LIBAVUTIL_VERSION_INT,
};

static int call_format_line2(void *ptr, char *line, int line_size, int *print_prefix,
                             int level, const char *fmt, ...) {
    va_list vl;
    va_start(vl, fmt);
    int ret = av_log_format_line2(ptr, level, fmt, vl, line, line_size,
                                  print_prefix);
    va_end(vl);
    return ret;
}

static void call_format_line(void *ptr, char *line, int line_size, int *print_prefix,
                             int level, const char *fmt, ...) {
    va_list vl;
    va_start(vl, fmt);
    av_log_format_line(ptr, level, fmt, vl, line, line_size, print_prefix);
    va_end(vl);
}

static int captured_count = 0;
static int captured_level = -999;
static char captured_message[512];
static char captured_item[128];

static void reset_capture(void) {
    captured_count = 0;
    captured_level = -999;
    captured_message[0] = '\0';
    snprintf(captured_item, sizeof(captured_item), "%s", "<none>");
}

static void capture_log_callback(void *ptr, int level, const char *fmt, va_list vl) {
    const AVClass *av_class = ptr ? *(const AVClass **)ptr : NULL;
    captured_count++;
    captured_level = level;
    vsnprintf(captured_message, sizeof(captured_message), fmt, vl);
    if (av_class && av_class->item_name)
        snprintf(captured_item, sizeof(captured_item), "%s", av_class->item_name(ptr));
    else
        snprintf(captured_item, sizeof(captured_item), "%s", "<none>");
}

static void print_level_after_set(const char *name, int level) {
    av_log_set_level(level);
    ROW(name, av_log_get_level());
}

static void print_flags_after_set(const char *name, int flags) {
    av_log_set_flags(flags);
    ROW(name, av_log_get_flags());
}

static void print_format_line2_rows(void) {
    char line[128];
    int print_prefix;
    int ret;

    av_log_set_flags(0);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-plain-ret", ret);
    ROW("format-line2-plain-prefix", print_prefix);
    ROW("format-line2-plain-len", strlen(line));
    ROW_STR("format-line2-plain-line", line);

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-level-ret", ret);
    ROW("format-line2-level-prefix", print_prefix);
    ROW("format-line2-level-len", strlen(line));
    ROW_STR("format-line2-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_QUIET, "%s", "quiet");
    ROW("format-line2-quiet-level-ret", ret);
    ROW("format-line2-quiet-level-prefix", print_prefix);
    ROW("format-line2-quiet-level-len", strlen(line));
    ROW_STR("format-line2-quiet-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_ERROR, "%s", "after");
    ROW("format-line2-noprefix-ret", ret);
    ROW("format-line2-noprefix-prefix", print_prefix);
    ROW("format-line2-noprefix-len", strlen(line));
    ROW_STR("format-line2-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_INFO, "%s\n", "withnl");
    ROW("format-line2-newline-ret", ret);
    ROW("format-line2-newline-prefix", print_prefix);
    ROW("format-line2-newline-len", strlen(line));
    ROW_STR("format-line2-newline-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_INFO, "%s", "withcr\r");
    ROW("format-line2-carriage-return-ret", ret);
    ROW("format-line2-carriage-return-prefix", print_prefix);
    ROW("format-line2-carriage-return-len", strlen(line));
    ROW_STR("format-line2-carriage-return-line", line);

    char small[8];
    memset(small, 'X', sizeof(small));
    print_prefix = 1;
    ret = call_format_line2(NULL, small, sizeof(small), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-small-ret", ret);
    ROW("format-line2-small-prefix", print_prefix);
    ROW("format-line2-small-len", strlen(small));
    ROW_STR("format-line2-small-line", small);

    print_prefix = 1;
    ret = call_format_line2(NULL, NULL, 0, &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-nullzero-ret", ret);
    ROW("format-line2-nullzero-prefix", print_prefix);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, 1, &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-size1-ret", ret);
    ROW("format-line2-size1-prefix", print_prefix);
    ROW("format-line2-size1-len", strlen(line));
    ROW_STR("format-line2-size1-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-time-ret", ret);
    ROW("format-line2-time-prefix", print_prefix);
    ROW("format-line2-time-len", strlen(line));
    ROW_STR("format-line2-time-line", line);

    av_log_set_flags(AV_LOG_PRINT_DATETIME | AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-datetime-level-ret", ret);
    ROW("format-line2-datetime-level-prefix", print_prefix);
    ROW("format-line2-datetime-level-len", strlen(line));
    ROW_STR("format-line2-datetime-level-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME | AV_LOG_PRINT_DATETIME |
                     AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(NULL, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "plain");
    ROW("format-line2-time-datetime-level-ret", ret);
    ROW("format-line2-time-datetime-level-prefix", print_prefix);
    ROW("format-line2-time-datetime-level-len", strlen(line));
    ROW_STR("format-line2-time-datetime-level-line", line);

    TestLogContext ctx = { &test_log_class };

    av_log_set_flags(0);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "ctxmsg");
    (void)ret;
    ROW("format-line2-context-plain-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-plain-line", line);

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "ctxmsg");
    (void)ret;
    ROW("format-line2-context-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_QUIET, "%s", "quiet");
    (void)ret;
    ROW("format-line2-context-quiet-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-quiet-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "nopfx");
    (void)ret;
    ROW("format-line2-context-noprefix-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_INFO, "%s\n", "withnl");
    (void)ret;
    ROW("format-line2-context-newline-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-newline-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_INFO, "%s", "withcr\r");
    (void)ret;
    ROW("format-line2-context-carriage-return-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-carriage-return-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    ret = call_format_line2(&ctx, line, sizeof(line), &print_prefix,
                            AV_LOG_WARNING, "%s", "ctxmsg");
    (void)ret;
    ROW("format-line2-context-time-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line2-context-time-level-line", line);
}

static void print_format_line_rows(void) {
    char line[128];
    int print_prefix;

    av_log_set_flags(0);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-plain-prefix", print_prefix);
    ROW("format-line-plain-len", strlen(line));
    ROW_STR("format-line-plain-line", line);

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-level-prefix", print_prefix);
    ROW("format-line-level-len", strlen(line));
    ROW_STR("format-line-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_QUIET, "%s", "quiet");
    ROW("format-line-quiet-level-prefix", print_prefix);
    ROW("format-line-quiet-level-len", strlen(line));
    ROW_STR("format-line-quiet-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_ERROR, "%s", "after");
    ROW("format-line-noprefix-prefix", print_prefix);
    ROW("format-line-noprefix-len", strlen(line));
    ROW_STR("format-line-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_INFO, "%s\n", "withnl");
    ROW("format-line-newline-prefix", print_prefix);
    ROW("format-line-newline-len", strlen(line));
    ROW_STR("format-line-newline-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_INFO, "%s", "withcr\r");
    ROW("format-line-carriage-return-prefix", print_prefix);
    ROW("format-line-carriage-return-len", strlen(line));
    ROW_STR("format-line-carriage-return-line", line);

    char small[8];
    memset(small, 'X', sizeof(small));
    print_prefix = 1;
    call_format_line(NULL, small, sizeof(small), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-small-prefix", print_prefix);
    ROW("format-line-small-len", strlen(small));
    ROW_STR("format-line-small-line", small);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, 1, &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-size1-prefix", print_prefix);
    ROW("format-line-size1-len", strlen(line));
    ROW_STR("format-line-size1-line", line);

    TestLogContext ctx = { &test_log_class };

    av_log_set_flags(AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "ctxmsg");
    ROW("format-line-context-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_QUIET, "%s", "quiet");
    ROW("format-line-context-quiet-level-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-quiet-level-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 0;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "nopfx");
    ROW("format-line-context-noprefix-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-noprefix-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_INFO, "%s\n", "withnl");
    ROW("format-line-context-newline-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-newline-line", line);

    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(&ctx, line, sizeof(line), &print_prefix,
                     AV_LOG_INFO, "%s", "withcr\r");
    ROW("format-line-context-carriage-return-prefix", print_prefix);
    ROW_STR_NORMALIZED_CONTEXT("format-line-context-carriage-return-line", line);

    av_log_set_flags(AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL);
    memset(line, 'X', sizeof(line));
    print_prefix = 1;
    call_format_line(NULL, line, sizeof(line), &print_prefix,
                     AV_LOG_WARNING, "%s", "plain");
    ROW("format-line-time-level-prefix", print_prefix);
    ROW("format-line-time-level-len", strlen(line));
    ROW_STR("format-line-time-level-line", line);
}

static void print_default_callback_level_row(const char *name, void *ptr, int level,
                                             int flags, const char *message) {
    char captured[1024];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(ptr, level, "%s\n", message);
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR_NORMALIZED_DEFAULT_CALLBACK(name, captured);
}

static void print_default_callback_row(const char *name, void *ptr, int flags,
                                       const char *message) {
    print_default_callback_level_row(name, ptr, AV_LOG_WARNING, flags, message);
}

static void print_default_callback_fixed_time_row(const char *name,
                                                  const char *tz,
                                                  int64_t time_us,
                                                  int flags,
                                                  const char *message) {
    char captured[1024];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    setenv("AV_LOG_FORCE_NOCOLOR", "1", 1);
    unsetenv("AV_LOG_FORCE_COLOR");
    setenv("TZ", tz, 1);
    tzset();
    fixed_av_gettime_value = time_us;

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(NULL, AV_LOG_WARNING, "%s\n", message);
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR(name, captured);
}

static void print_default_callback_threshold_row(const char *name,
                                                 void *ptr,
                                                 int threshold,
                                                 int level,
                                                 int flags,
                                                 const char *message) {
    char captured[1024];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    setenv("AV_LOG_FORCE_NOCOLOR", "1", 1);
    unsetenv("AV_LOG_FORCE_COLOR");

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(threshold);
    av_log_set_flags(flags);
    av_log(ptr, level, "%s\n", message);
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR_NORMALIZED_DEFAULT_CALLBACK(name, captured);
}

static void print_default_callback_repeat_row(const char *name, void *ptr, int flags) {
    char captured[2048];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(ptr, AV_LOG_WARNING, "%s\n", "repeat");
    av_log(ptr, AV_LOG_WARNING, "%s\n", "repeat");
    av_log(ptr, AV_LOG_WARNING, "%s\n", "repeat");
    av_log(ptr, AV_LOG_ERROR, "%s\n", "next");
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR_NORMALIZED_DEFAULT_CALLBACK(name, captured);
}

static void print_default_callback_repeat_context_switch_row(const char *name,
                                                             int flags) {
    char captured[2048];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    TestLogContext ctx_a = { &test_log_class };
    TestLogContext ctx_b = { &test_log_class };

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(&ctx_a, AV_LOG_WARNING, "%s\n", "repeat");
    av_log(&ctx_b, AV_LOG_WARNING, "%s\n", "repeat");
    av_log(&ctx_a, AV_LOG_WARNING, "%s\n", "repeat");
    av_log(&ctx_a, AV_LOG_ERROR, "%s\n", "next");
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR_NORMALIZED_DEFAULT_CALLBACK(name, captured);
}

static void print_default_callback_prefix_continuation_row(const char *name,
                                                           void *ptr,
                                                           int flags) {
    char captured[2048];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(ptr, AV_LOG_WARNING, "%s", "part");
    av_log(ptr, AV_LOG_WARNING, "%s\n", "tail");
    av_log(ptr, AV_LOG_WARNING, "%s\n", "next");
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR_NORMALIZED_DEFAULT_CALLBACK(name, captured);
}

static void print_default_callback_prefix_carriage_return_row(const char *name,
                                                              void *ptr,
                                                              int flags) {
    char captured[2048];
    FILE *capture = tmpfile();
    if (!capture) {
        ROW_STR(name, "<tmpfile-error>");
        return;
    }

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(fileno(capture), fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        fclose(capture);
        ROW_STR(name, "<stderr-redirect-error>");
        return;
    }

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(ptr, AV_LOG_WARNING, "%s", "progress\r");
    av_log(ptr, AV_LOG_WARNING, "%s\n", "done");
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    rewind(capture);
    size_t len = fread(captured, 1, sizeof(captured) - 1, capture);
    captured[len] = '\0';
    fclose(capture);

    ROW_STR_NORMALIZED_DEFAULT_CALLBACK(name, captured);
}

static void print_default_callback_rows(void) {
    setenv("AV_LOG_FORCE_NOCOLOR", "1", 1);
    TestLogContext ctx = { &test_log_class };
    print_default_callback_row("default-callback-time-line", NULL, AV_LOG_PRINT_TIME,
                               "plain");
    print_default_callback_row("default-callback-time-level-line",
                               NULL, AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL,
                               "plain");
    print_default_callback_row("default-callback-datetime-line", NULL,
                               AV_LOG_PRINT_DATETIME, "plain");
    print_default_callback_row("default-callback-datetime-level-line",
                               NULL, AV_LOG_PRINT_DATETIME | AV_LOG_PRINT_LEVEL,
                               "plain");
    print_default_callback_row("default-callback-time-datetime-level-line",
                               NULL, AV_LOG_PRINT_TIME | AV_LOG_PRINT_DATETIME |
                               AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_row("default-callback-context-line", &ctx, 0,
                               "ctxmsg");
    print_default_callback_row("default-callback-context-level-line", &ctx,
                               AV_LOG_PRINT_LEVEL, "ctxmsg");
    print_default_callback_row("default-callback-context-time-level-line", &ctx,
                               AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL,
                               "ctxmsg");
    print_default_callback_row("default-callback-context-datetime-level-line",
                               &ctx, AV_LOG_PRINT_DATETIME | AV_LOG_PRINT_LEVEL,
                               "ctxmsg");
    print_default_callback_fixed_time_row(
        "default-callback-fixed-time-utcplus2-line", "Etc/GMT-2",
        1704112705123456LL, AV_LOG_PRINT_TIME, "local");
    print_default_callback_fixed_time_row(
        "default-callback-fixed-datetime-utcplus530-level-line", "UTC-5:30",
        1704112705123456LL, AV_LOG_PRINT_DATETIME | AV_LOG_PRINT_LEVEL,
        "local");
    print_default_callback_fixed_time_row(
        "default-callback-fixed-datetime-utcminus8-level-line", "Etc/GMT+8",
        1704070923456789LL, AV_LOG_PRINT_DATETIME | AV_LOG_PRINT_LEVEL,
        "local");
    print_default_callback_threshold_row(
        "default-callback-filter-info-at-warning-line", NULL, AV_LOG_WARNING,
        AV_LOG_INFO, AV_LOG_PRINT_LEVEL, "hidden");
    print_default_callback_threshold_row(
        "default-callback-filter-warning-at-warning-line", NULL,
        AV_LOG_WARNING, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "shown");
    print_default_callback_threshold_row(
        "default-callback-filter-error-at-raw23-line", NULL,
        23, AV_LOG_ERROR, AV_LOG_PRINT_LEVEL, "raw shown");
    print_default_callback_threshold_row(
        "default-callback-filter-warning-at-raw23-line", NULL,
        23, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "raw hidden");
    print_default_callback_threshold_row(
        "default-callback-quiet-at-quiet-line", NULL, AV_LOG_QUIET,
        AV_LOG_QUIET, AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL, "quiet");
    print_default_callback_threshold_row(
        "default-callback-quiet-context-at-quiet-line", &ctx,
        AV_LOG_QUIET, AV_LOG_QUIET, AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL,
        "quiet");
    print_default_callback_repeat_row("default-callback-repeat-skip-line", NULL,
                                      AV_LOG_SKIP_REPEATED);
    print_default_callback_repeat_row("default-callback-repeat-skip-level-line", NULL,
                                      AV_LOG_SKIP_REPEATED | AV_LOG_PRINT_LEVEL);
    print_default_callback_repeat_row("default-callback-repeat-context-level-line", &ctx,
                                      AV_LOG_SKIP_REPEATED | AV_LOG_PRINT_LEVEL);
    print_default_callback_repeat_context_switch_row(
        "default-callback-repeat-context-switch-level-line",
        AV_LOG_SKIP_REPEATED | AV_LOG_PRINT_LEVEL);
    print_default_callback_repeat_row("default-callback-repeat-noskip-line", NULL, 0);
    print_default_callback_prefix_continuation_row(
        "default-callback-prefix-continuation-plain-line", NULL, 0);
    print_default_callback_prefix_continuation_row(
        "default-callback-prefix-continuation-level-line", NULL,
        AV_LOG_PRINT_LEVEL);
    print_default_callback_prefix_continuation_row(
        "default-callback-prefix-continuation-context-level-line", &ctx,
        AV_LOG_PRINT_LEVEL);
    print_default_callback_prefix_carriage_return_row(
        "default-callback-prefix-carriage-return-level-line", NULL,
        AV_LOG_PRINT_LEVEL);
    print_default_callback_prefix_carriage_return_row(
        "default-callback-prefix-carriage-return-context-level-line", &ctx,
        AV_LOG_PRINT_LEVEL);
}

static void print_default_callback_color_rows(void) {
    TestLogContext ctx = { &test_log_class };
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    setenv("AV_LOG_FORCE_COLOR", "1", 1);
    print_default_callback_level_row("default-callback-color-warning-line", NULL,
                                     AV_LOG_WARNING, 0, "plain");
    print_default_callback_level_row("default-callback-color-warning-level-line", NULL,
                                     AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_level_row("default-callback-color-warning-context-level-line",
                                     &ctx, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL,
                                     "plain");
    print_default_callback_level_row("default-callback-color-quiet-line", NULL,
                                     AV_LOG_QUIET,
                                     AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL,
                                     "quiet");
    print_default_callback_level_row("default-callback-color-quiet-context-level-line",
                                     &ctx, AV_LOG_QUIET,
                                     AV_LOG_PRINT_TIME | AV_LOG_PRINT_LEVEL,
                                     "quiet");
    print_default_callback_repeat_row("default-callback-color-repeat-level-line", NULL,
                                      AV_LOG_SKIP_REPEATED | AV_LOG_PRINT_LEVEL);
    print_default_callback_repeat_row("default-callback-color-repeat-context-level-line", &ctx,
                                      AV_LOG_SKIP_REPEATED | AV_LOG_PRINT_LEVEL);
    print_default_callback_prefix_continuation_row(
        "default-callback-color-prefix-continuation-level-line", NULL,
        AV_LOG_PRINT_LEVEL);
    print_default_callback_prefix_continuation_row(
        "default-callback-color-prefix-continuation-context-level-line", &ctx,
        AV_LOG_PRINT_LEVEL);
    print_default_callback_prefix_carriage_return_row(
        "default-callback-color-prefix-carriage-return-level-line", NULL,
        AV_LOG_PRINT_LEVEL);
    print_default_callback_prefix_carriage_return_row(
        "default-callback-color-prefix-carriage-return-context-level-line", &ctx,
        AV_LOG_PRINT_LEVEL);
    print_default_callback_level_row("default-callback-color-error-line", NULL,
                                     AV_LOG_ERROR, 0, "plain");
    print_default_callback_level_row("default-callback-color-fatal-level-line", NULL,
                                     AV_LOG_FATAL, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_level_row("default-callback-color-panic-level-line", NULL,
                                     AV_LOG_PANIC, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_level_row("default-callback-color-info-line", NULL,
                                     AV_LOG_INFO, 0, "plain");
    unsetenv("AV_LOG_FORCE_COLOR");
}

static void print_default_callback_color_cache_after_nocolor_rows(void) {
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    setenv("AV_LOG_FORCE_COLOR", "1", 1);
    print_default_callback_level_row("default-callback-color-cache-after-nocolor-line",
                                     NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL,
                                     "plain");
    unsetenv("AV_LOG_FORCE_COLOR");
}

static void print_default_callback_no_force_redirected_rows(void) {
    TestLogContext ctx = { &test_log_class };
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    unsetenv("AV_LOG_FORCE_COLOR");
    print_default_callback_level_row(
        "default-callback-no-force-redirected-warning-level-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_level_row(
        "default-callback-no-force-redirected-context-level-line",
        &ctx, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
}

static void print_default_callback_tty_level_row(const char *name, void *ptr, int level,
                                                 int flags, const char *message) {
    char captured[1024];
    int master_fd = -1;
    int slave_fd = -1;
    if (openpty(&master_fd, &slave_fd, NULL, NULL, NULL) < 0) {
        ROW_STR(name, "<pty-open-error>");
        return;
    }

    struct termios tio;
    if (tcgetattr(slave_fd, &tio) == 0) {
        tio.c_oflag &= ~(OPOST | ONLCR);
        tcsetattr(slave_fd, TCSANOW, &tio);
    }

    fflush(stderr);
    int saved_stderr = dup(fileno(stderr));
    if (saved_stderr < 0 || dup2(slave_fd, fileno(stderr)) < 0) {
        if (saved_stderr >= 0)
            close(saved_stderr);
        close(slave_fd);
        close(master_fd);
        ROW_STR(name, "<stderr-pty-error>");
        return;
    }
    close(slave_fd);

    av_log_set_callback(av_log_default_callback);
    av_log_set_level(AV_LOG_TRACE);
    av_log_set_flags(flags);
    av_log(ptr, level, "%s\n", message);
    fflush(stderr);

    dup2(saved_stderr, fileno(stderr));
    close(saved_stderr);

    size_t len = 0;
    while (len < sizeof(captured) - 1) {
        ssize_t read_len = read(master_fd, captured + len,
                                sizeof(captured) - 1 - len);
        if (read_len <= 0)
            break;
        len += (size_t)read_len;
    }
    captured[len] = '\0';
    close(master_fd);

    ROW_STR_NORMALIZED_DEFAULT_CALLBACK(name, captured);
}

static void print_default_callback_no_force_tty_rows(void) {
    TestLogContext ctx = { &test_log_class };
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    unsetenv("AV_LOG_FORCE_COLOR");
    setenv("TERM", "xterm-256color", 1);
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-warning-level-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-context-level-line",
        &ctx, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-256color-context-warning-level-line",
        &ctx, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-256color-error-level-line",
        NULL, AV_LOG_ERROR, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-256color-fatal-level-line",
        NULL, AV_LOG_FATAL, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-256color-panic-level-line",
        NULL, AV_LOG_PANIC, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-256color-info-line",
        NULL, AV_LOG_INFO, 0, "plain");
}

static void print_default_callback_no_force_tty_term_unset_row(void) {
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    unsetenv("AV_LOG_FORCE_COLOR");
    unsetenv("TERM");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-unset-warning-level-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
}

static void print_default_callback_no_force_tty_term_dumb_row(void) {
    TestLogContext ctx = { &test_log_class };
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    unsetenv("AV_LOG_FORCE_COLOR");
    setenv("TERM", "dumb", 1);
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-dumb-warning-level-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-dumb-context-warning-level-line",
        &ctx, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-dumb-error-level-line",
        NULL, AV_LOG_ERROR, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-dumb-fatal-level-line",
        NULL, AV_LOG_FATAL, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-dumb-panic-level-line",
        NULL, AV_LOG_PANIC, AV_LOG_PRINT_LEVEL, "plain");
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-dumb-info-line",
        NULL, AV_LOG_INFO, 0, "plain");
}

static void print_default_callback_no_force_tty_term_empty_row(void) {
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    unsetenv("AV_LOG_FORCE_COLOR");
    setenv("TERM", "", 1);
    print_default_callback_tty_level_row(
        "default-callback-no-force-tty-term-empty-warning-level-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
}

static void print_default_callback_nocolor_rows(void) {
    setenv("AV_LOG_FORCE_NOCOLOR", "1", 1);
    setenv("AV_LOG_FORCE_COLOR", "1", 1);
    print_default_callback_level_row("default-callback-nocolor-wins-warning-line",
                                     NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL,
                                     "plain");
    unsetenv("AV_LOG_FORCE_COLOR");
    unsetenv("AV_LOG_FORCE_NOCOLOR");
}

static void print_default_callback_force_color_empty_row(void) {
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    setenv("AV_LOG_FORCE_COLOR", "", 1);
    print_default_callback_level_row(
        "default-callback-force-color-empty-warning-level-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    unsetenv("AV_LOG_FORCE_COLOR");
}

static void print_default_callback_force_color_zero_row(void) {
    unsetenv("AV_LOG_FORCE_NOCOLOR");
    setenv("AV_LOG_FORCE_COLOR", "0", 1);
    print_default_callback_level_row(
        "default-callback-force-color-zero-warning-level-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    unsetenv("AV_LOG_FORCE_COLOR");
}

static void print_default_callback_force_nocolor_empty_row(void) {
    setenv("AV_LOG_FORCE_NOCOLOR", "", 1);
    setenv("AV_LOG_FORCE_COLOR", "1", 1);
    print_default_callback_level_row(
        "default-callback-force-nocolor-empty-wins-warning-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    unsetenv("AV_LOG_FORCE_COLOR");
    unsetenv("AV_LOG_FORCE_NOCOLOR");
}

static void print_default_callback_force_nocolor_zero_row(void) {
    setenv("AV_LOG_FORCE_NOCOLOR", "0", 1);
    setenv("AV_LOG_FORCE_COLOR", "1", 1);
    print_default_callback_level_row(
        "default-callback-force-nocolor-zero-wins-warning-line",
        NULL, AV_LOG_WARNING, AV_LOG_PRINT_LEVEL, "plain");
    unsetenv("AV_LOG_FORCE_COLOR");
    unsetenv("AV_LOG_FORCE_NOCOLOR");
}

static void print_custom_callback_rows(void) {
    TestLogContext ctx = { &test_log_class };
    av_log_set_callback(capture_log_callback);
    av_log_set_level(AV_LOG_WARNING);
    av_log_set_flags(AV_LOG_PRINT_LEVEL | AV_LOG_SKIP_REPEATED);

    reset_capture();
    av_log(NULL, AV_LOG_INFO, "%s", "hidden");
    ROW("custom-callback-above-level-count", captured_count);
    ROW("custom-callback-above-level-level", captured_level);
    ROW_STR("custom-callback-above-level-message", captured_message);
    ROW_STR("custom-callback-above-level-item", captured_item);

    reset_capture();
    av_log(NULL, 23, "%s", "rawlevel");
    ROW("custom-callback-raw-level-count", captured_count);
    ROW("custom-callback-raw-level-level", captured_level);
    ROW_STR("custom-callback-raw-level-message", captured_message);
    ROW_STR("custom-callback-raw-level-item", captured_item);

    reset_capture();
    av_log(NULL, AV_LOG_ERROR, "%s:%d\n", "raw", 5);
    ROW("custom-callback-null-count", captured_count);
    ROW("custom-callback-null-level", captured_level);
    ROW_STR("custom-callback-null-message", captured_message);
    ROW_STR("custom-callback-null-item", captured_item);

    reset_capture();
    av_log(NULL, AV_LOG_WARNING, "%s", "repeat");
    av_log(NULL, AV_LOG_WARNING, "%s", "repeat");
    ROW("custom-callback-repeat-count", captured_count);
    ROW("custom-callback-repeat-level", captured_level);
    ROW_STR("custom-callback-repeat-message", captured_message);
    ROW_STR("custom-callback-repeat-item", captured_item);

    reset_capture();
    av_log(&ctx, AV_LOG_WARNING, "%s:%d", "ctx", 3);
    ROW("custom-callback-context-count", captured_count);
    ROW("custom-callback-context-level", captured_level);
    ROW_STR("custom-callback-context-message", captured_message);
    ROW_STR("custom-callback-context-item", captured_item);

    reset_capture();
    av_log(&ctx, AV_LOG_WARNING, "%s", "ctxrepeat");
    av_log(&ctx, AV_LOG_WARNING, "%s", "ctxrepeat");
    ROW("custom-callback-context-repeat-count", captured_count);
    ROW("custom-callback-context-repeat-level", captured_level);
    ROW_STR("custom-callback-context-repeat-message", captured_message);
    ROW_STR("custom-callback-context-repeat-item", captured_item);

    av_log_set_callback(av_log_default_callback);
}

int main(int argc, char **argv) {
    if (argc > 1 && strcmp(argv[1], "--plain") == 0) {
        print_default_callback_no_force_redirected_rows();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--tty") == 0) {
        print_default_callback_no_force_tty_rows();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--tty-term-unset") == 0) {
        print_default_callback_no_force_tty_term_unset_row();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--tty-term-dumb") == 0) {
        print_default_callback_no_force_tty_term_dumb_row();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--tty-term-empty") == 0) {
        print_default_callback_no_force_tty_term_empty_row();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--color") == 0) {
        print_default_callback_color_rows();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--nocolor") == 0) {
        print_default_callback_nocolor_rows();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--force-color-empty") == 0) {
        print_default_callback_force_color_empty_row();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--force-color-zero") == 0) {
        print_default_callback_force_color_zero_row();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--force-nocolor-empty") == 0) {
        print_default_callback_force_nocolor_empty_row();
        return 0;
    }
    if (argc > 1 && strcmp(argv[1], "--force-nocolor-zero") == 0) {
        print_default_callback_force_nocolor_zero_row();
        return 0;
    }

    ROW("AV_LOG_QUIET", AV_LOG_QUIET);
    ROW("AV_LOG_PANIC", AV_LOG_PANIC);
    ROW("AV_LOG_FATAL", AV_LOG_FATAL);
    ROW("AV_LOG_ERROR", AV_LOG_ERROR);
    ROW("AV_LOG_WARNING", AV_LOG_WARNING);
    ROW("AV_LOG_INFO", AV_LOG_INFO);
    ROW("AV_LOG_VERBOSE", AV_LOG_VERBOSE);
    ROW("AV_LOG_DEBUG", AV_LOG_DEBUG);
    ROW("AV_LOG_TRACE", AV_LOG_TRACE);
    ROW("AV_LOG_MAX_OFFSET", AV_LOG_MAX_OFFSET);
    ROW("AV_LOG_SKIP_REPEATED", AV_LOG_SKIP_REPEATED);
    ROW("AV_LOG_PRINT_LEVEL", AV_LOG_PRINT_LEVEL);
    ROW("AV_LOG_PRINT_TIME", AV_LOG_PRINT_TIME);
    ROW("AV_LOG_PRINT_DATETIME", AV_LOG_PRINT_DATETIME);

    ROW("default-level", av_log_get_level());
    print_level_after_set("set-level-quiet", AV_LOG_QUIET);
    print_level_after_set("set-level-panic", AV_LOG_PANIC);
    print_level_after_set("set-level-fatal", AV_LOG_FATAL);
    print_level_after_set("set-level-error", AV_LOG_ERROR);
    print_level_after_set("set-level-warning", AV_LOG_WARNING);
    print_level_after_set("set-level-info", AV_LOG_INFO);
    print_level_after_set("set-level-verbose", AV_LOG_VERBOSE);
    print_level_after_set("set-level-debug", AV_LOG_DEBUG);
    print_level_after_set("set-level-trace", AV_LOG_TRACE);
    print_level_after_set("set-level-raw-minus-one", -1);
    print_level_after_set("set-level-raw-between-error-warning", 23);
    print_level_after_set("set-level-raw-above-trace", 57);

    print_flags_after_set("set-flags-empty", 0);
    print_flags_after_set("set-flags-skip-repeated", AV_LOG_SKIP_REPEATED);
    print_flags_after_set("set-flags-print-level", AV_LOG_PRINT_LEVEL);
    print_flags_after_set("set-flags-print-time", AV_LOG_PRINT_TIME);
    print_flags_after_set("set-flags-print-datetime", AV_LOG_PRINT_DATETIME);
    print_flags_after_set("set-flags-all-known",
                          AV_LOG_SKIP_REPEATED | AV_LOG_PRINT_LEVEL |
                          AV_LOG_PRINT_TIME | AV_LOG_PRINT_DATETIME);
    print_flags_after_set("set-flags-unknown-bit", 0x10);
    print_flags_after_set("set-flags-mixed-unknown", 0x1234);
    print_flags_after_set("set-flags-negative-all-raw", -1);
    print_format_line2_rows();
    print_format_line_rows();
    print_default_callback_rows();
    print_default_callback_color_cache_after_nocolor_rows();
    print_custom_callback_rows();
    av_log_set_callback(capture_log_callback);
    reset_capture();
    int once_state = 0;
    av_log_once(NULL, AV_LOG_WARNING, AV_LOG_DEBUG, &once_state, "%s", "once");
    ROW("log-once-first-state", once_state);
    ROW("log-once-first-count", captured_count);
    ROW("log-once-first-level", captured_level);
    av_log_once(NULL, AV_LOG_WARNING, AV_LOG_DEBUG, &once_state, "%s", "once");
    ROW("log-once-second-state", once_state);
    ROW("log-once-second-count", captured_count);
    ROW("log-once-second-level", captured_level);
    int preseed_state = 7;
    av_log_once(NULL, AV_LOG_INFO, AV_LOG_ERROR, &preseed_state, "%s", "preseed");
    ROW("log-once-preseed-state", preseed_state);
    ROW("log-once-preseed-count", captured_count);
    ROW("log-once-preseed-level", captured_level);
    av_log_set_callback(av_log_default_callback);
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
